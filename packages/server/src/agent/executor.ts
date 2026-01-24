import { EventEmitter } from 'events';
import type {
  AgentSession,
  AgentConfig,
  AgentOutputEvent,
  AgentStatusEvent,
  AgentSessionStatus,
} from '@opencode-vibe/protocol';
import { OpencodeClient, type OpencodeEvent } from './opencode-client';

interface SessionEntry {
  session: AgentSession;
  client: OpencodeClient;
  config: AgentConfig;
}

/**
 * AgentExecutor - 管理 AI 编码代理
 *
 * 使用 OpencodeClient 通过 HTTP API 与 OpenCode 通信，
 * 支持多会话管理、输入输出处理和状态跟踪。
 */
export class AgentExecutor {
  private sessions: Map<string, SessionEntry> = new Map();
  private eventEmitter: EventEmitter = new EventEmitter();

  constructor() {}

  /**
   * 启动 Agent 会话
   */
  async start(session: AgentSession, config: AgentConfig): Promise<void> {
    // Check if session already exists and is active
    const existing = this.sessions.get(session.id);
    if (existing && this.isActive(existing.session.status)) {
      throw new Error(`Session ${session.id} is already active`);
    }

    // Create OpenCode client
    const client = new OpencodeClient({
      cwd: config.cwd || process.cwd(),
      env: config.env,
    });

    // Update session state
    const previousStatus = session.status;
    const updatedSession: AgentSession = {
      ...session,
      status: 'starting',
      startedAt: Date.now(),
    };

    // Store session entry
    this.sessions.set(session.id, {
      session: updatedSession,
      client,
      config,
    });

    // Set up event handlers
    this.setupClientEvents(session.id, client);

    // Emit starting status
    this.emitStatusChange(session.id, previousStatus, 'starting');

    try {
      // Start the OpenCode server and run the prompt
      await this.runSession(session.id, session.prompt);
    } catch (error) {
      // Handle startup errors
      const entry = this.sessions.get(session.id);
      if (entry) {
        const prevStatus = entry.session.status;
        entry.session = {
          ...entry.session,
          status: 'failed',
          endedAt: Date.now(),
          error: error instanceof Error ? error.message : String(error),
        };
        this.emitStatusChange(session.id, prevStatus, 'failed', entry.session.error);
      }
    }
  }

  /**
   * 运行会话
   */
  private async runSession(sessionId: string, prompt: string): Promise<void> {
    const entry = this.sessions.get(sessionId);
    if (!entry) return;

    const { client } = entry;

    try {
      // Start OpenCode server
      await client.start();

      // Update status to running
      const prevStatus = entry.session.status;
      entry.session = {
        ...entry.session,
        status: 'running',
      };
      this.emitStatusChange(sessionId, prevStatus, 'running');

      // Wait for health check
      const healthy = await client.waitForHealth();
      if (!healthy) {
        throw new Error('OpenCode server failed health check');
      }

      // Create session and connect event stream
      const opencodeSessionId = await client.createSession();
      
      // Start event stream listener (non-blocking)
      const eventPromise = client.connectEventStream(opencodeSessionId);
      
      // Wait a bit for event stream to connect
      await new Promise(r => setTimeout(r, 100));

      // Send the message
      await client.sendMessage(opencodeSessionId, prompt);

      // Wait for completion
      await eventPromise;

    } catch (error) {
      throw error;
    }
  }

  /**
   * 设置客户端事件监听
   */
  private setupClientEvents(sessionId: string, client: OpencodeClient): void {
    // Handle output events
    client.on('output', (output: { type: string; data: string }) => {
      const event: AgentOutputEvent = {
        sessionId,
        type: output.type as 'stdout' | 'stderr',
        data: output.data,
        timestamp: Date.now(),
      };
      this.eventEmitter.emit('output', event);
    });

    // Handle OpenCode events
    client.on('event', (event: OpencodeEvent) => {
      // Convert OpenCode events to output for display
      const outputData = this.formatOpencodeEvent(event);
      if (outputData) {
        const outputEvent: AgentOutputEvent = {
          sessionId,
          type: 'stdout',
          data: outputData,
          timestamp: Date.now(),
        };
        this.eventEmitter.emit('output', outputEvent);
      }
    });

    // Handle idle (completion)
    client.on('idle', () => {
      const entry = this.sessions.get(sessionId);
      if (entry) {
        const prevStatus = entry.session.status;
        entry.session = {
          ...entry.session,
          status: 'completed',
          endedAt: Date.now(),
        };
        this.emitStatusChange(sessionId, prevStatus, 'completed');
      }
    });

    // Handle done
    client.on('done', () => {
      const entry = this.sessions.get(sessionId);
      if (entry && this.isActive(entry.session.status)) {
        const prevStatus = entry.session.status;
        entry.session = {
          ...entry.session,
          status: 'completed',
          endedAt: Date.now(),
        };
        this.emitStatusChange(sessionId, prevStatus, 'completed');
      }
    });

    // Handle errors
    client.on('error', (error: Error) => {
      const entry = this.sessions.get(sessionId);
      if (entry) {
        const prevStatus = entry.session.status;
        entry.session = {
          ...entry.session,
          status: 'failed',
          endedAt: Date.now(),
          error: error.message,
        };
        this.emitStatusChange(sessionId, prevStatus, 'failed', error.message);
      }
    });

    // Handle exit
    client.on('exit', (code: number) => {
      const entry = this.sessions.get(sessionId);
      if (entry && this.isActive(entry.session.status)) {
        const prevStatus = entry.session.status;
        const newStatus: AgentSessionStatus = code === 0 ? 'completed' : 'failed';
        entry.session = {
          ...entry.session,
          status: newStatus,
          endedAt: Date.now(),
          error: code !== 0 ? `OpenCode exited with code ${code}` : undefined,
        };
        this.emitStatusChange(sessionId, prevStatus, newStatus, entry.session.error);
      }
    });
  }

  /**
   * 格式化 OpenCode 事件为可显示文本
   */
  private formatOpencodeEvent(event: OpencodeEvent): string | null {
    const props = event.properties || {};

    switch (event.type) {
      case 'message.part.delta': {
        // Legacy format
        const part = props.part as { content?: string } | undefined;
        if (part?.content) {
          return part.content;
        }
        break;
      }
      case 'message.part.updated': {
        // OpenCode 1.1.x format - extract text from part
        const part = props.part as { type?: string; text?: string } | undefined;
        if (part?.type === 'text' && part?.text) {
          // Store last seen text to detect deltas
          const partId = (props.part as { id?: string })?.id;
          if (partId) {
            const lastText = this.lastPartText.get(partId) || '';
            const newText = part.text;
            // Only emit the new content (delta)
            if (newText.length > lastText.length && newText.startsWith(lastText)) {
              const delta = newText.slice(lastText.length);
              this.lastPartText.set(partId, newText);
              return delta;
            } else if (newText !== lastText) {
              // Text changed completely, emit all
              this.lastPartText.set(partId, newText);
              return newText;
            }
          }
        }
        break;
      }
      case 'message.part.done': {
        // Completion of a message part, no output needed
        return null;
      }
      case 'message.updated': {
        // Message update, no output needed  
        return null;
      }
      case 'session.idle': {
        return '\n[Session completed]\n';
      }
      case 'session.error': {
        const error = props.error as { message?: string } | undefined;
        return `\n[Error: ${error?.message || 'Unknown error'}]\n`;
      }
      case 'tool.start': {
        const toolName = props.name as string | undefined;
        return toolName ? `\n[Tool: ${toolName}]\n` : null;
      }
      case 'tool.done': {
        return null; // Tool completion, no output needed
      }
      default:
        return null;
    }
    return null;
  }

  // Track last seen text for each part to compute deltas
  private lastPartText: Map<string, string> = new Map();

  /**
   * 停止 Agent 会话
   */
  async stop(sessionId: string): Promise<void> {
    const entry = this.sessions.get(sessionId);

    if (!entry) {
      throw new Error(`Session ${sessionId} not found`);
    }

    if (!this.isActive(entry.session.status)) {
      throw new Error(`Session ${sessionId} is not active`);
    }

    const previousStatus = entry.session.status;

    // Stop the OpenCode client
    entry.client.stop();

    // Update session state
    entry.session = {
      ...entry.session,
      status: 'aborted',
      endedAt: Date.now(),
    };

    // Emit status change
    this.emitStatusChange(sessionId, previousStatus, 'aborted');
  }

  /**
   * 向 Agent 发送输入 (不再支持，保留接口兼容性)
   */
  write(sessionId: string, _data: string): void {
    const entry = this.sessions.get(sessionId);

    if (!entry) {
      throw new Error(`Session ${sessionId} not found`);
    }

    if (!this.isActive(entry.session.status)) {
      throw new Error(`Session ${sessionId} is not active`);
    }

    // Note: OpenCode HTTP API 不支持直接写入
    // 如果需要发送新消息，应该调用 sendMessage
    console.warn('write() is not supported with OpenCode HTTP API. Use a new session for new prompts.');
  }

  /**
   * 注册输出事件回调
   */
  onOutput(callback: (event: AgentOutputEvent) => void): void {
    this.eventEmitter.on('output', callback);
  }

  /**
   * 注册状态变更事件回调
   */
  onStatus(callback: (event: AgentStatusEvent) => void): void {
    this.eventEmitter.on('status', callback);
  }

  /**
   * 获取指定会话
   */
  getSession(sessionId: string): AgentSession | undefined {
    return this.sessions.get(sessionId)?.session;
  }

  /**
   * 获取所有会话
   */
  getAllSessions(): AgentSession[] {
    return Array.from(this.sessions.values()).map((entry) => entry.session);
  }

  /**
   * 检查状态是否为活跃状态
   */
  private isActive(status: AgentSessionStatus): boolean {
    return ['starting', 'running', 'paused'].includes(status);
  }

  /**
   * 发送状态变更事件
   */
  private emitStatusChange(
    sessionId: string,
    previousStatus: AgentSessionStatus,
    currentStatus: AgentSessionStatus,
    error?: string
  ): void {
    const event: AgentStatusEvent = {
      sessionId,
      previousStatus,
      currentStatus,
      timestamp: Date.now(),
      error,
    };
    this.eventEmitter.emit('status', event);
  }
}
