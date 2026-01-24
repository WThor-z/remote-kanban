/**
 * OpenCode HTTP API Client
 * 
 * 与 OpenCode serve 模式通信
 */

import { spawn, ChildProcess } from 'child_process';
import { EventEmitter } from 'events';

export interface OpencodeServerConfig {
  cwd?: string;
  env?: Record<string, string>;
}

export interface OpencodeSession {
  id: string;
  baseUrl: string;
}

export interface OpencodeEvent {
  type: string;
  properties?: Record<string, unknown>;
}

/**
 * OpenCode Server 管理器
 * 
 * 启动 opencode serve 并通过 HTTP API 通信
 */
export class OpencodeClient extends EventEmitter {
  private process: ChildProcess | null = null;
  private baseUrl: string | null = null;
  private serverPassword: string;
  private directory: string;
  private eventSource: AbortController | null = null;
  private config: OpencodeServerConfig;

  constructor(config: OpencodeServerConfig = {}) {
    super();
    this.config = config;
    this.serverPassword = this.generatePassword();
    this.directory = config.cwd || process.cwd();
  }

  private generatePassword(): string {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    for (let i = 0; i < 32; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  /**
   * 启动 OpenCode 服务器
   */
  async start(): Promise<string> {
    if (this.process) {
      throw new Error('OpenCode server already running');
    }

    return new Promise((resolve, reject) => {
      const env = {
        ...process.env,
        ...this.config.env,
        NPM_CONFIG_LOGLEVEL: 'error',
        NODE_NO_WARNINGS: '1',
        NO_COLOR: '1',
        OPENCODE_SERVER_PASSWORD: this.serverPassword,
      };

      // 使用 opencode serve 命令
      this.process = spawn('opencode', ['serve', '--hostname', '127.0.0.1', '--port', '0'], {
        cwd: this.config.cwd || process.cwd(),
        env: env as NodeJS.ProcessEnv,
        stdio: ['ignore', 'pipe', 'pipe'],
        shell: true,
      });

      let startupOutput = '';
      const timeout = setTimeout(() => {
        reject(new Error(`OpenCode server startup timeout. Output: ${startupOutput}`));
        this.stop();
      }, 60000);

      this.process.stdout?.on('data', (data: Buffer) => {
        const text = data.toString();
        startupOutput += text;
        this.emit('output', { type: 'stdout', data: text });

        // 检测服务器启动成功
        const match = text.match(/opencode server listening on\s+(\S+)/i);
        if (match) {
          clearTimeout(timeout);
          this.baseUrl = match[1].trim();
          this.emit('ready', this.baseUrl);
          resolve(this.baseUrl);
        }
      });

      this.process.stderr?.on('data', (data: Buffer) => {
        const text = data.toString();
        startupOutput += text;
        this.emit('output', { type: 'stderr', data: text });
      });

      this.process.on('error', (err) => {
        clearTimeout(timeout);
        reject(err);
      });

      this.process.on('exit', (code) => {
        clearTimeout(timeout);
        this.emit('exit', code);
        this.process = null;
        this.baseUrl = null;
      });
    });
  }

  /**
   * 停止服务器
   */
  stop(): void {
    if (this.eventSource) {
      this.eventSource.abort();
      this.eventSource = null;
    }
    if (this.process) {
      this.process.kill();
      this.process = null;
    }
    this.baseUrl = null;
  }

  /**
   * 检查服务器健康状态
   */
  async waitForHealth(): Promise<boolean> {
    if (!this.baseUrl) {
      throw new Error('Server not started');
    }

    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) {
      try {
        const resp = await fetch(`${this.baseUrl}/global/health`, {
          headers: this.getHeaders(),
        });
        if (resp.ok) {
          const data = await resp.json() as { healthy: boolean };
          if (data.healthy) {
            return true;
          }
        }
      } catch {
        // 忽略错误，继续重试
      }
      await new Promise(r => setTimeout(r, 150));
    }
    return false;
  }

  /**
   * 创建会话
   */
  async createSession(): Promise<string> {
    if (!this.baseUrl) {
      throw new Error('Server not started');
    }

    const resp = await fetch(`${this.baseUrl}/session?directory=${encodeURIComponent(this.directory)}`, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify({}),
    });

    if (!resp.ok) {
      throw new Error(`Failed to create session: HTTP ${resp.status}`);
    }

    const data = await resp.json() as { id: string };
    return data.id;
  }

  /**
   * 发送消息
   */
  async sendMessage(sessionId: string, prompt: string): Promise<void> {
    if (!this.baseUrl) {
      throw new Error('Server not started');
    }

    const resp = await fetch(
      `${this.baseUrl}/session/${sessionId}/message?directory=${encodeURIComponent(this.directory)}`,
      {
        method: 'POST',
        headers: this.getHeaders(),
        body: JSON.stringify({
          parts: [{ type: 'text', text: prompt }],
        }),
      }
    );

    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(`Failed to send message: HTTP ${resp.status} ${text}`);
    }
  }

  /**
   * 连接事件流
   */
  async connectEventStream(sessionId: string): Promise<void> {
    if (!this.baseUrl) {
      throw new Error('Server not started');
    }

    this.eventSource = new AbortController();

    try {
      const resp = await fetch(
        `${this.baseUrl}/event?directory=${encodeURIComponent(this.directory)}`,
        {
          headers: {
            ...this.getHeaders(),
            'Accept': 'text/event-stream',
          },
          signal: this.eventSource.signal,
        }
      );

      if (!resp.ok || !resp.body) {
        throw new Error(`Failed to connect event stream: HTTP ${resp.status}`);
      }

      const reader = resp.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            try {
              const data = JSON.parse(line.slice(6)) as OpencodeEvent;
              
              // 检查是否是当前会话的事件
              const eventSessionId = this.extractSessionId(data);
              if (eventSessionId && eventSessionId !== sessionId) {
                continue;
              }

              this.emit('event', data);

              // 检测会话空闲
              if (data.type === 'session.idle') {
                this.emit('idle');
                return;
              }

              // 检测错误
              if (data.type === 'session.error') {
                const message = (data.properties as Record<string, unknown>)?.error as Record<string, unknown>;
                this.emit('error', new Error(String(message?.message || 'Session error')));
              }
            } catch {
              // 忽略解析错误
            }
          }
        }
      }
    } catch (err) {
      if ((err as Error).name !== 'AbortError') {
        this.emit('error', err);
      }
    }
  }

  /**
   * 中止会话
   */
  async abort(sessionId: string): Promise<void> {
    if (!this.baseUrl) return;

    try {
      await fetch(
        `${this.baseUrl}/session/${sessionId}/abort?directory=${encodeURIComponent(this.directory)}`,
        {
          method: 'POST',
          headers: this.getHeaders(),
        }
      );
    } catch {
      // 忽略中止错误
    }
  }

  /**
   * 运行完整会话
   */
  async run(prompt: string): Promise<void> {
    await this.start();
    
    try {
      const healthy = await this.waitForHealth();
      if (!healthy) {
        throw new Error('OpenCode server failed health check');
      }

      const sessionId = await this.createSession();
      this.emit('session', { id: sessionId, baseUrl: this.baseUrl });

      // 并行启动事件流和发送消息
      const eventPromise = this.connectEventStream(sessionId);
      
      // 等待一小段时间让事件流连接
      await new Promise(r => setTimeout(r, 100));
      
      await this.sendMessage(sessionId, prompt);
      await eventPromise;
      
      this.emit('done');
    } finally {
      this.stop();
    }
  }

  private getHeaders(): Record<string, string> {
    const credentials = Buffer.from(`opencode:${this.serverPassword}`).toString('base64');
    return {
      'Content-Type': 'application/json',
      'Authorization': `Basic ${credentials}`,
      'x-opencode-directory': this.directory,
    };
  }

  private extractSessionId(event: OpencodeEvent): string | undefined {
    const props = event.properties || {};
    return (
      (props.sessionID as string) ||
      ((props.info as Record<string, unknown>)?.sessionID as string) ||
      ((props.part as Record<string, unknown>)?.sessionID as string)
    );
  }

  get isRunning(): boolean {
    return this.process !== null;
  }

  get serverUrl(): string | null {
    return this.baseUrl;
  }
}

export default OpencodeClient;
