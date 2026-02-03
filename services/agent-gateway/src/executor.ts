import { spawn, ChildProcess } from 'child_process';
import { EventEmitter } from 'events';
import { createOpencodeClient } from '@opencode-ai/sdk';
import type { TaskRequest, GatewayAgentEvent, TaskResult, ProviderInfo, ModelInfo } from './types.js';

export interface ExecutorOptions {
  defaultCwd: string;
  defaultAgent: string;
  /** OpenCode server port (will start server if not provided) */
  serverPort?: number;
}

interface ActiveTask {
  taskId: string;
  sessionId: string;
  abortController: AbortController;
}

type OpencodeClient = ReturnType<typeof createOpencodeClient>;

export class TaskExecutor extends EventEmitter {
  private activeTasks = new Map<string, ActiveTask>();
  private opencodeClient: OpencodeClient | null = null;
  private serverProcess: ChildProcess | null = null;
  private serverUrl: string | null = null;
  private initPromise: Promise<void> | null = null;

  constructor(private options: ExecutorOptions) {
    super();
  }

  /**
   * 初始化 OpenCode SDK 客户端
   * 手动启动 opencode serve，然后使用客户端连接
   */
  private async ensureInitialized(): Promise<void> {
    if (this.opencodeClient) return;
    
    if (this.initPromise) {
      await this.initPromise;
      return;
    }

    this.initPromise = this.initialize();
    await this.initPromise;
  }

  private async initialize(): Promise<void> {
    console.log('[executor] Initializing OpenCode server...');
    
    try {
      // 在 Windows 上使用 opencode.cmd，其他平台使用 opencode
      const isWindows = process.platform === 'win32';
      const command = isWindows ? 'opencode.cmd' : 'opencode';
      const port = this.options.serverPort || 0;
      
      // 启动 opencode serve
      const args = ['serve', '--hostname=127.0.0.1', `--port=${port}`];
      
      console.log(`[executor] Starting: ${command} ${args.join(' ')}`);
      
      this.serverProcess = spawn(command, args, {
        cwd: this.options.defaultCwd,
        shell: isWindows, // Windows 需要 shell
        env: {
          ...process.env,
          // 确保非交互模式
          CI: '1',
          NO_COLOR: '1',
        },
        stdio: ['ignore', 'pipe', 'pipe'],
      });

      // 等待服务器启动并获取 URL
      this.serverUrl = await this.waitForServerReady();
      
      console.log(`[executor] OpenCode server started at ${this.serverUrl}`);

      // 创建客户端连接
      this.opencodeClient = createOpencodeClient({
        baseUrl: this.serverUrl,
      });

      // 验证连接 - 使用 session.list() 作为健康检查
      const sessions = await this.opencodeClient.session.list();
      console.log(`[executor] Connected to OpenCode (${sessions.data?.length ?? 0} existing sessions)`);

    } catch (error) {
      console.error('[executor] Failed to initialize OpenCode server:', error);
      this.cleanup();
      throw error;
    }
  }

  /**
   * 等待服务器启动并返回 URL
   */
  private waitForServerReady(): Promise<string> {
    return new Promise((resolve, reject) => {
      if (!this.serverProcess) {
        reject(new Error('Server process not started'));
        return;
      }

      const timeout = setTimeout(() => {
        reject(new Error('Server startup timeout'));
      }, 30000);

      let output = '';
      let errorOutput = '';

      this.serverProcess.stdout?.on('data', (data) => {
        const text = data.toString();
        output += text;
        console.log('[executor:stdout]', text.trim());
        
        // 查找服务器 URL，格式类似: "Listening on http://127.0.0.1:4096"
        const urlMatch = text.match(/(?:listening on|server.*?at|started.*?on)\s*(https?:\/\/[\w.:]+)/i);
        if (urlMatch) {
          clearTimeout(timeout);
          resolve(urlMatch[1]);
        }
        
        // 也检查 JSON 格式的输出
        try {
          const lines = text.split('\n');
          for (const line of lines) {
            if (line.trim().startsWith('{')) {
              const json = JSON.parse(line);
              if (json.url || json.server?.url) {
                clearTimeout(timeout);
                resolve(json.url || json.server.url);
              }
            }
          }
        } catch {
          // 不是 JSON，忽略
        }
      });

      this.serverProcess.stderr?.on('data', (data) => {
        const text = data.toString();
        errorOutput += text;
        console.error('[executor:stderr]', text.trim());
      });

      this.serverProcess.on('error', (err) => {
        clearTimeout(timeout);
        reject(err);
      });

      this.serverProcess.on('exit', (code) => {
        if (code !== 0 && code !== null) {
          clearTimeout(timeout);
          reject(new Error(`Server exited with code ${code}: ${errorOutput}`));
        }
      });
    });
  }

  async execute(task: TaskRequest): Promise<TaskResult> {
    const startTime = Date.now();
    
    try {
      await this.ensureInitialized();

      if (!this.opencodeClient) {
        throw new Error('OpenCode client not initialized');
      }

      this.emitEvent(task.taskId, {
        type: 'log',
        content: `[executor] Starting task via SDK: ${task.prompt.slice(0, 100)}...`,
        timestamp: Date.now(),
      });

      // 创建新的 session
      const sessionResult = await this.opencodeClient.session.create({
        body: { 
          title: `Gateway Task: ${task.taskId}`,
        },
      });

      if (!sessionResult.data) {
        throw new Error('Failed to create session');
      }

      const session = sessionResult.data;
      const abortController = new AbortController();

      this.activeTasks.set(task.taskId, {
        taskId: task.taskId,
        sessionId: session.id,
        abortController,
      });

      this.emitEvent(task.taskId, {
        type: 'log',
        content: `[executor] Session created: ${session.id}`,
        timestamp: Date.now(),
      });

      // 解析模型参数
      const parsedModel = task.model ? this.parseModel(task.model) : undefined;
      
      // 记录使用的模型
      if (parsedModel) {
        console.log(`[executor] Using model: ${parsedModel.providerID}/${parsedModel.modelID}`);
        this.emitEvent(task.taskId, {
          type: 'log',
          content: `[executor] Using model: ${parsedModel.providerID}/${parsedModel.modelID}`,
          timestamp: Date.now(),
        });
      } else {
        console.log(`[executor] WARNING: No model specified, using OpenCode default (may not work!)`);
        this.emitEvent(task.taskId, {
          type: 'log',
          content: `[executor] WARNING: No model specified, using OpenCode default`,
          timestamp: Date.now(),
        });
      }

      // 发送 prompt (使用异步方式)
      // @ts-ignore - promptAsync 可能不在类型定义中
      await this.opencodeClient.session.promptAsync({
        path: { id: session.id },
        body: {
          model: parsedModel,
          parts: [{ type: 'text', text: task.prompt }],
        },
      });

      this.emitEvent(task.taskId, {
        type: 'log',
        content: `[executor] Prompt sent, waiting for response...`,
        timestamp: Date.now(),
      });

      // 等待完成 (优先使用 SSE 流式模式)
      const output = await this.waitForSessionCompleteStreaming(task.taskId, session.id);

      const duration = Date.now() - startTime;
      this.activeTasks.delete(task.taskId);

      return {
        success: true,
        output: output.slice(-2000),
        duration,
      };
    } catch (error) {
      const duration = Date.now() - startTime;
      this.activeTasks.delete(task.taskId);

      this.emitEvent(task.taskId, {
        type: 'error',
        content: `[executor] Task failed: ${error}`,
        timestamp: Date.now(),
      });

      return {
        success: false,
        output: String(error),
        duration,
      };
    }
  }

  /**
   * 使用 SSE 流式等待 session 完成
   * 监听 message.part.updated 事件获取实时增量输出
   * 监听 session.idle 事件判断完成
   */
  private async waitForSessionCompleteStreaming(taskId: string, sessionId: string): Promise<string> {
    if (!this.opencodeClient) {
      throw new Error('OpenCode client not initialized');
    }

    const timeout = 300000; // 5 分钟超时
    const startTime = Date.now();
    let fullOutput = '';

    console.log(`[executor:streaming] Starting SSE streaming for session ${sessionId}`);

    return new Promise<string>(async (resolve, reject) => {
      const timeoutId = setTimeout(() => {
        console.log(`[executor:streaming] Timeout reached after ${timeout}ms`);
        reject(new Error('Session timeout'));
      }, timeout);

      try {
        // 订阅事件流
        const eventStream = await this.opencodeClient!.event.subscribe({
          query: { directory: this.options.defaultCwd },
        });

        // 迭代事件流
        for await (const event of eventStream.stream) {
          // 检查是否已中止
          const activeTask = this.activeTasks.get(taskId);
          if (!activeTask) {
            clearTimeout(timeoutId);
            reject(new Error('Task was aborted'));
            return;
          }

          const payload = (event as any).payload || event;
          const eventType = payload?.type;

          // 过滤只处理当前 session 的事件
          const eventSessionId = payload?.properties?.info?.sessionID || 
                                  payload?.properties?.part?.sessionID ||
                                  payload?.properties?.sessionID;
          
          if (eventSessionId && eventSessionId !== sessionId) {
            continue; // 不是当前 session 的事件
          }

          // console.log(`[executor:streaming] Event:`, eventType);

          switch (eventType) {
            case 'message.part.updated': {
              const part = payload.properties?.part;
              const delta = payload.properties?.delta;
              
              if (part?.type === 'text') {
                // 优先使用 delta（增量文本）
                if (delta) {
                  fullOutput += delta;
                  this.emitEvent(taskId, {
                    type: 'stdout',
                    content: delta,
                    timestamp: Date.now(),
                  });
                  console.log(`[executor:streaming] Delta: ${delta.slice(0, 50)}${delta.length > 50 ? '...' : ''}`);
                }
              } else if (part?.type === 'tool') {
                // 工具调用
                this.emitEvent(taskId, {
                  type: 'log',
                  content: `[executor] Tool: ${part.name || part.id || 'unknown'}`,
                  timestamp: Date.now(),
                });
              }
              break;
            }

            case 'message.updated': {
              const msgInfo = payload.properties?.info;
              
              // 检查是否有错误
              if (msgInfo?.error) {
                const errorMsg = msgInfo.error.data?.message || msgInfo.error.name || 'Unknown error';
                console.error(`[executor:streaming] AI error:`, errorMsg);
                this.emitEvent(taskId, {
                  type: 'error',
                  content: `[executor] AI error: ${errorMsg}`,
                  timestamp: Date.now(),
                });
              }
              break;
            }

            case 'session.idle': {
              // Session 完成
              console.log(`[executor:streaming] Session idle, completing...`);
              clearTimeout(timeoutId);
              resolve(fullOutput);
              return;
            }

            case 'session.error': {
              const errorMsg = payload.properties?.error || 'Unknown session error';
              console.error(`[executor:streaming] Session error:`, errorMsg);
              clearTimeout(timeoutId);
              reject(new Error(errorMsg));
              return;
            }
          }
        }

        // 流结束但没有收到 idle 事件
        console.log(`[executor:streaming] Stream ended without idle event`);
        clearTimeout(timeoutId);
        resolve(fullOutput);

      } catch (error) {
        clearTimeout(timeoutId);
        console.error(`[executor:streaming] SSE error:`, error);
        
        // 如果 SSE 失败，回退到轮询模式
        console.log(`[executor:streaming] Falling back to polling mode...`);
        try {
          const result = await this.waitForSessionCompletePolling(taskId, sessionId);
          resolve(result);
        } catch (pollError) {
          reject(pollError);
        }
      }
    });
  }

  /**
   * 轮询模式等待 session 完成（作为 SSE 的回退）
   * 通过检查消息的 time.completed 字段来判断是否完成
   */
  private async waitForSessionCompletePolling(taskId: string, sessionId: string): Promise<string> {
    if (!this.opencodeClient) {
      throw new Error('OpenCode client not initialized');
    }

    const timeout = 300000; // 5 分钟超时
    const pollInterval = 1000; // 1 秒轮询（比之前更快）
    const startTime = Date.now();
    let lastOutputLength = 0;

    console.log(`[executor:polling] Starting polling for session ${sessionId}`);

    while (Date.now() - startTime < timeout) {
      // 检查是否已中止
      const activeTask = this.activeTasks.get(taskId);
      if (!activeTask) {
        throw new Error('Task was aborted');
      }

      try {
        const messagesResult = await this.opencodeClient.session.messages({
          path: { id: sessionId },
        });

        const messages = messagesResult.data;
        if (messages && messages.length > 0) {
          const assistantMessages = messages.filter((msg: any) => msg.info?.role === 'assistant');
          
          if (assistantMessages.length > 0) {
            const lastAssistant = assistantMessages[assistantMessages.length - 1] as any;
            const msgInfo = lastAssistant.info;
            const msgParts = lastAssistant.parts || [];

            // 检查是否有错误
            if (msgInfo?.error) {
              const errorMsg = msgInfo.error.data?.message || msgInfo.error.name || 'Unknown error';
              if (msgInfo?.time?.completed) {
                throw new Error(`AI request failed: ${errorMsg}`);
              }
            }

            // 获取文本内容
            const textParts = msgParts.filter((p: any) => p.type === 'text');
            const currentText = textParts.map((p: any) => p.text || '').join('');
            
            // 只发送新增的内容
            if (currentText.length > lastOutputLength) {
              const newContent = currentText.slice(lastOutputLength);
              lastOutputLength = currentText.length;
              
              this.emitEvent(taskId, {
                type: 'stdout',
                content: newContent,
                timestamp: Date.now(),
              });
            }

            // 检查是否完成
            if (msgInfo?.time?.completed) {
              console.log(`[executor:polling] Session completed!`);
              return currentText;
            }
          }
        }
      } catch (msgError: any) {
        if (msgError.message?.includes('AI request failed')) {
          throw msgError;
        }
        console.error('[executor:polling] Error getting messages:', msgError);
      }

      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    throw new Error('Session timeout');
  }

  /**
   * 解析 model 字符串为 SDK 格式
   * 格式: "provider/model" -> { providerID, modelID }
   */
  private parseModel(model: string): { providerID: string; modelID: string } | undefined {
    const parts = model.split('/');
    if (parts.length >= 2) {
      return {
        providerID: parts[0],
        modelID: parts.slice(1).join('/'),
      };
    }
    return undefined;
  }

  abort(taskId: string): boolean {
    const task = this.activeTasks.get(taskId);
    if (task) {
      // 安全地调用 abort
      if (task.abortController) {
        task.abortController.abort();
      }
      
      // 尝试通过 SDK 中止 session
      if (this.opencodeClient && task.sessionId) {
        this.opencodeClient.session.abort({
          path: { id: task.sessionId },
        }).catch(err => {
          console.error(`[executor] Failed to abort session ${task.sessionId}:`, err);
        });
      }

      this.activeTasks.delete(taskId);
      return true;
    }
    return false;
  }

  sendInput(taskId: string, content: string): boolean {
    // SDK 方式暂不支持交互式输入
    console.warn('[executor] sendInput not supported in SDK mode');
    return false;
  }

  private emitEvent(taskId: string, event: GatewayAgentEvent): void {
    this.emit('event', { taskId, event });
  }

  private cleanup(): void {
    if (this.serverProcess) {
      this.serverProcess.kill();
      this.serverProcess = null;
    }
    this.opencodeClient = null;
    this.serverUrl = null;
    this.initPromise = null;
  }

  get activeTaskCount(): number {
    return this.activeTasks.size;
  }

  get activeTaskIds(): string[] {
    return Array.from(this.activeTasks.keys());
  }

  /**
   * 关闭服务器
   */
  async shutdown(): Promise<void> {
    console.log('[executor] Shutting down OpenCode server...');
    this.cleanup();
  }

  /**
   * 获取可用的模型列表
   * 从 OpenCode 获取所有提供商及其模型
   * 
   * @param connectedOnly - 如果为 true，只返回已连接（有 API key）的提供商
   */
  async getAvailableModels(connectedOnly: boolean = true): Promise<ProviderInfo[]> {
    console.log('[executor] getAvailableModels called, connectedOnly:', connectedOnly);
    
    await this.ensureInitialized();

    if (!this.opencodeClient) {
      throw new Error('OpenCode client not initialized');
    }

    try {
      // 获取提供商列表
      // @ts-ignore - provider.list 可能不在类型定义中
      const providersResult = await this.opencodeClient.provider.list();
      
      // 打印原始响应用于调试
      console.log('[executor] Raw provider.list() response:', JSON.stringify(providersResult, null, 2).slice(0, 1000));
      
      // 数据结构: { all: Provider[], default: {...}, connected: string[] }
      const providersData = providersResult.data;
      
      if (!providersData) {
        console.log('[executor] No data in response');
        return [];
      }

      if (!providersData.all || !Array.isArray(providersData.all)) {
        console.log('[executor] No providers.all array, providersData keys:', Object.keys(providersData));
        return [];
      }

      const allProviders = providersData.all;
      const connectedProviderIds = new Set(providersData.connected || []);
      
      // 如果 connectedOnly 为 true，只处理已连接的提供商
      const providers = connectedOnly 
        ? allProviders.filter((p: any) => connectedProviderIds.has(p.id))
        : allProviders;
      
      console.log(`[executor] Found ${allProviders.length} total providers, ${connectedProviderIds.size} connected: ${[...connectedProviderIds].join(', ')}`);
      console.log(`[executor] Processing ${providers.length} providers (connectedOnly=${connectedOnly})`);

      const result: ProviderInfo[] = [];

      for (const provider of providers) {
        const models: ModelInfo[] = [];
        
        // 提取模型信息
        if (provider.models && typeof provider.models === 'object') {
          for (const [modelId, modelData] of Object.entries(provider.models)) {
            const model = modelData as any;
            models.push({
              id: modelId,
              providerId: provider.id,
              name: model.name || modelId,
              capabilities: {
                temperature: model.temperature || false,
                reasoning: model.reasoning || false,
                attachment: model.attachment || false,
                toolcall: model.tool_call || false,  // SDK uses tool_call (with underscore)
              },
            });
          }
        }

        if (models.length > 0) {
          result.push({
            id: provider.id,
            name: provider.name || provider.id,
            models,
          });
        }
      }

      console.log(`[executor] Returning ${result.length} providers with ${result.reduce((sum, p) => sum + p.models.length, 0)} total models`);
      return result;
    } catch (error) {
      console.error('[executor] Failed to get models:', error);
      throw error;
    }
  }
}
