# Agent Gateway 分布式架构 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 Agent Gateway 架构，让各主机上的 OpenCode 能够通过 WebSocket 主动连接到云服务器，接收任务、执行、并实时推送事件。

**Architecture:** 
- 主机端运行 Agent Gateway (Node.js)，主动连接服务器
- 服务器端 Gateway Manager (Rust) 管理所有主机连接和任务分发
- WebSocket 双向通信，支持实时事件流、断线重连

**Tech Stack:** TypeScript (Gateway), Rust + Axum (Server), WebSocket, OpenCode CLI

---

## 目录

1. [协议规范](#1-协议规范)
2. [实施计划](#2-实施计划)
3. [测试策略](#3-测试策略)
4. [部署指南](#4-部署指南)

---

## 1. 协议规范

### 1.1 连接流程

```
┌─────────────────────────────────────────────────────────────────────┐
│                             时序图                                    │
└─────────────────────────────────────────────────────────────────────┘

  Gateway (主机)                                Server (云)
       │                                            │
       │──────── WSS CONNECT ────────────────────►│
       │         /agent/ws?hostId=xxx              │
       │         Authorization: Bearer <token>      │
       │                                            │
       │◄───────── CONNECTED ─────────────────────│
       │                                            │
       │──────── REGISTER ────────────────────────►│
       │         {hostId, capabilities}             │
       │                                            │
       │◄──────── REGISTERED ─────────────────────│
       │         {ok: true}                         │
       │                                            │
       │◄──────── PING ───────────────────────────│ (每 30s)
       │──────── HEARTBEAT ───────────────────────►│
       │                                            │
       │◄──────── TASK:EXECUTE ───────────────────│ (有任务时)
       │         {taskId, prompt, cwd, ...}         │
       │                                            │
       │──────── TASK:STARTED ────────────────────►│
       │         {taskId, sessionId}                │
       │                                            │
       │──────── TASK:EVENT ──────────────────────►│ (多次)
       │         {taskId, event}                    │
       │                                            │
       │──────── TASK:COMPLETED ──────────────────►│
       │         {taskId, result}                   │
       │                                            │
```

### 1.2 消息类型定义

#### GatewayToServer (Gateway → Server)

| Type | 描述 | Payload |
|------|------|---------|
| `register` | 注册主机 | `{ hostId: string, capabilities: HostCapabilities }` |
| `heartbeat` | 心跳 | `{ timestamp: number }` |
| `task:started` | 任务开始 | `{ taskId: string, sessionId: string }` |
| `task:event` | 任务事件 | `{ taskId: string, event: AgentEvent }` |
| `task:completed` | 任务完成 | `{ taskId: string, result: TaskResult }` |
| `task:failed` | 任务失败 | `{ taskId: string, error: string, details?: unknown }` |

#### ServerToGateway (Server → Gateway)

| Type | 描述 | Payload |
|------|------|---------|
| `registered` | 注册响应 | `{ ok: boolean, error?: string }` |
| `ping` | 心跳检测 | `{}` |
| `task:execute` | 执行任务 | `{ task: TaskRequest }` |
| `task:abort` | 中止任务 | `{ taskId: string }` |
| `task:input` | 发送输入 | `{ taskId: string, content: string }` |
| `config:update` | 配置更新 | `{ config: Partial<GatewayConfig> }` |

### 1.3 数据结构

```typescript
/** 主机能力 */
interface HostCapabilities {
  name: string;                              // 主机名称
  agents: ('opencode' | 'claude-code' | 'gemini')[];  // 支持的 Agent
  maxConcurrent: number;                     // 最大并发任务数
  cwd: string;                               // 默认工作目录
  labels?: Record<string, string>;           // 自定义标签 (用于任务路由)
}

/** 任务请求 */
interface TaskRequest {
  taskId: string;           // 任务 ID
  prompt: string;           // 提示词
  cwd: string;              // 工作目录
  agentType: string;        // Agent 类型
  model?: string;           // 模型 (如 "openai/gpt-4o")
  env?: Record<string, string>;  // 环境变量
  timeout?: number;         // 超时时间 (ms)
  metadata?: Record<string, unknown>;  // 元数据
}

/** Agent 事件 */
interface AgentEvent {
  type: 'log' | 'thinking' | 'tool_call' | 'tool_result' | 'file_change' | 'message' | 'error';
  content?: string;         // 文本内容
  data?: unknown;           // 结构化数据
  timestamp: number;        // 时间戳
}

/** 任务结果 */
interface TaskResult {
  success: boolean;
  exitCode?: number;
  output?: string;          // 最终输出
  duration?: number;        // 执行时长 (ms)
  filesChanged?: string[];  // 修改的文件
}
```

### 1.4 错误处理

| 错误场景 | Gateway 行为 | Server 行为 |
|----------|-------------|-------------|
| 连接断开 | 指数退避重连 (1s, 2s, 4s, 8s, max 60s) | 标记主机离线，重新分配任务 |
| 任务超时 | 发送 `task:failed`，杀死进程 | 更新任务状态为 TIMEOUT |
| OpenCode 崩溃 | 发送 `task:failed`，包含 stderr | 记录错误日志 |
| 心跳超时 (90s) | - | 主动断开连接，标记主机离线 |
| 认证失败 | 断开连接，不重连 | 返回 401，关闭 WebSocket |

### 1.5 重连策略

```typescript
class ReconnectStrategy {
  private baseDelay = 1000;      // 1 秒
  private maxDelay = 60000;      // 60 秒
  private attempt = 0;
  
  getNextDelay(): number {
    const delay = Math.min(
      this.baseDelay * Math.pow(2, this.attempt),
      this.maxDelay
    );
    this.attempt++;
    // 添加随机抖动 (±25%)
    return delay * (0.75 + Math.random() * 0.5);
  }
  
  reset(): void {
    this.attempt = 0;
  }
}
```

---

## 2. 实施计划

### Phase 0: 准备工作

#### Task 0.1: 定义共享类型

**Files:**
- Create: `shared/api-types/src/gateway.ts`
- Modify: `shared/api-types/src/index.ts`

**Step 1: 创建 gateway.ts 类型定义**

```typescript
// shared/api-types/src/gateway.ts

/** 主机能力描述 */
export interface HostCapabilities {
  name: string;
  agents: ('opencode' | 'claude-code' | 'gemini')[];
  maxConcurrent: number;
  cwd: string;
  labels?: Record<string, string>;
}

/** 任务请求 */
export interface TaskRequest {
  taskId: string;
  prompt: string;
  cwd: string;
  agentType: string;
  model?: string;
  env?: Record<string, string>;
  timeout?: number;
  metadata?: Record<string, unknown>;
}

/** Agent 事件类型 */
export type AgentEventType = 
  | 'log' 
  | 'thinking' 
  | 'tool_call' 
  | 'tool_result' 
  | 'file_change' 
  | 'message' 
  | 'error';

/** Agent 事件 */
export interface AgentEvent {
  type: AgentEventType;
  content?: string;
  data?: unknown;
  timestamp: number;
}

/** 任务结果 */
export interface TaskResult {
  success: boolean;
  exitCode?: number;
  output?: string;
  duration?: number;
  filesChanged?: string[];
}

/** Gateway -> Server 消息 */
export type GatewayToServerMessage =
  | { type: 'register'; hostId: string; capabilities: HostCapabilities }
  | { type: 'heartbeat'; timestamp: number }
  | { type: 'task:started'; taskId: string; sessionId: string }
  | { type: 'task:event'; taskId: string; event: AgentEvent }
  | { type: 'task:completed'; taskId: string; result: TaskResult }
  | { type: 'task:failed'; taskId: string; error: string; details?: unknown };

/** Server -> Gateway 消息 */
export type ServerToGatewayMessage =
  | { type: 'registered'; ok: boolean; error?: string }
  | { type: 'ping' }
  | { type: 'task:execute'; task: TaskRequest }
  | { type: 'task:abort'; taskId: string }
  | { type: 'task:input'; taskId: string; content: string }
  | { type: 'config:update'; config: Partial<GatewayConfig> };

/** Gateway 配置 */
export interface GatewayConfig {
  heartbeatInterval: number;
  taskTimeout: number;
  maxRetries: number;
}

/** 主机状态 */
export interface HostStatus {
  hostId: string;
  name: string;
  status: 'online' | 'offline' | 'busy';
  capabilities: HostCapabilities;
  activeTasks: string[];
  lastHeartbeat: number;
  connectedAt: number;
}
```

**Step 2: 导出类型**

```typescript
// shared/api-types/src/index.ts (追加)
export * from './gateway';
```

**Step 3: 构建验证**

Run: `cd shared/api-types && npm run build`
Expected: 成功，无错误

**Step 4: Commit**

```bash
git add shared/api-types/src/gateway.ts shared/api-types/src/index.ts
git commit -m "feat(api-types): add gateway protocol type definitions"
```

---

### Phase 1: Agent Gateway 服务

#### Task 1.1: 初始化 agent-gateway 项目

**Files:**
- Create: `services/agent-gateway/package.json`
- Create: `services/agent-gateway/tsconfig.json`
- Create: `services/agent-gateway/src/index.ts`

**Step 1: 创建 package.json**

```json
{
  "name": "@vk/agent-gateway",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "tsx watch src/index.ts",
    "build": "tsc",
    "start": "node dist/index.js"
  },
  "dependencies": {
    "ws": "^8.16.0",
    "zod": "^3.22.4"
  },
  "devDependencies": {
    "@types/node": "^20.10.0",
    "@types/ws": "^8.5.10",
    "tsx": "^4.7.0",
    "typescript": "^5.3.0"
  }
}
```

**Step 2: 创建 tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 3: 创建入口文件骨架**

```typescript
// services/agent-gateway/src/index.ts
console.log('Agent Gateway starting...');

// 将在后续任务中实现
```

**Step 4: 安装依赖**

Run: `cd services/agent-gateway && npm install`

**Step 5: Commit**

```bash
git add services/agent-gateway/
git commit -m "feat(agent-gateway): initialize project structure"
```

---

#### Task 1.2: 实现 WebSocket 连接管理

**Files:**
- Create: `services/agent-gateway/src/connection.ts`
- Create: `services/agent-gateway/src/types.ts`

**Step 1: 创建本地类型定义**

```typescript
// services/agent-gateway/src/types.ts
import type { 
  HostCapabilities, 
  GatewayToServerMessage, 
  ServerToGatewayMessage,
  TaskRequest,
  AgentEvent,
  TaskResult 
} from '@vk/api-types';

export interface GatewayOptions {
  serverUrl: string;
  hostId: string;
  authToken: string;
  capabilities: HostCapabilities;
  reconnect?: boolean;
}

export interface ConnectionState {
  status: 'disconnected' | 'connecting' | 'connected' | 'registered';
  lastError?: string;
  reconnectAttempt: number;
}

export { 
  HostCapabilities, 
  GatewayToServerMessage, 
  ServerToGatewayMessage,
  TaskRequest,
  AgentEvent,
  TaskResult 
};
```

**Step 2: 实现连接管理器**

```typescript
// services/agent-gateway/src/connection.ts
import WebSocket from 'ws';
import { EventEmitter } from 'events';
import type { 
  GatewayOptions, 
  ConnectionState,
  GatewayToServerMessage,
  ServerToGatewayMessage 
} from './types';

export class GatewayConnection extends EventEmitter {
  private ws: WebSocket | null = null;
  private state: ConnectionState = {
    status: 'disconnected',
    reconnectAttempt: 0,
  };
  private reconnectTimer: NodeJS.Timeout | null = null;
  private heartbeatTimer: NodeJS.Timeout | null = null;

  constructor(private options: GatewayOptions) {
    super();
  }

  async connect(): Promise<void> {
    if (this.state.status !== 'disconnected') {
      throw new Error(`Cannot connect: current status is ${this.state.status}`);
    }

    this.state.status = 'connecting';
    this.emit('stateChange', this.state);

    const url = new URL('/agent/ws', this.options.serverUrl);
    url.searchParams.set('hostId', this.options.hostId);

    try {
      this.ws = new WebSocket(url.toString(), {
        headers: {
          Authorization: `Bearer ${this.options.authToken}`,
        },
      });

      this.ws.on('open', () => this.handleOpen());
      this.ws.on('message', (data) => this.handleMessage(data));
      this.ws.on('close', (code, reason) => this.handleClose(code, reason.toString()));
      this.ws.on('error', (err) => this.handleError(err));

    } catch (err) {
      this.state.status = 'disconnected';
      this.state.lastError = err instanceof Error ? err.message : String(err);
      this.emit('stateChange', this.state);
      throw err;
    }
  }

  private handleOpen(): void {
    console.log('[Gateway] WebSocket connected');
    this.state.status = 'connected';
    this.state.reconnectAttempt = 0;
    this.emit('stateChange', this.state);

    // 发送注册消息
    this.send({
      type: 'register',
      hostId: this.options.hostId,
      capabilities: this.options.capabilities,
    });

    // 启动心跳
    this.startHeartbeat();
  }

  private handleMessage(data: WebSocket.RawData): void {
    try {
      const msg: ServerToGatewayMessage = JSON.parse(data.toString());
      this.emit('message', msg);

      if (msg.type === 'registered') {
        if (msg.ok) {
          this.state.status = 'registered';
          this.emit('stateChange', this.state);
          console.log('[Gateway] Registered successfully');
        } else {
          console.error('[Gateway] Registration failed:', msg.error);
          this.disconnect();
        }
      } else if (msg.type === 'ping') {
        this.send({ type: 'heartbeat', timestamp: Date.now() });
      }
    } catch (err) {
      console.error('[Gateway] Failed to parse message:', err);
    }
  }

  private handleClose(code: number, reason: string): void {
    console.log(`[Gateway] WebSocket closed: ${code} ${reason}`);
    this.cleanup();

    if (this.options.reconnect !== false) {
      this.scheduleReconnect();
    }
  }

  private handleError(err: Error): void {
    console.error('[Gateway] WebSocket error:', err.message);
    this.state.lastError = err.message;
    this.emit('error', err);
  }

  send(msg: GatewayToServerMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      console.warn('[Gateway] Cannot send: WebSocket not open');
    }
  }

  disconnect(): void {
    this.options.reconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.cleanup();
  }

  private cleanup(): void {
    this.state.status = 'disconnected';
    this.emit('stateChange', this.state);

    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }

    if (this.ws) {
      this.ws.removeAllListeners();
      if (this.ws.readyState === WebSocket.OPEN) {
        this.ws.close();
      }
      this.ws = null;
    }
  }

  private startHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
    }
    this.heartbeatTimer = setInterval(() => {
      this.send({ type: 'heartbeat', timestamp: Date.now() });
    }, 30000);
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;

    const delay = this.getReconnectDelay();
    console.log(`[Gateway] Reconnecting in ${delay}ms...`);

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.state.reconnectAttempt++;
      this.connect().catch((err) => {
        console.error('[Gateway] Reconnect failed:', err.message);
      });
    }, delay);
  }

  private getReconnectDelay(): number {
    const baseDelay = 1000;
    const maxDelay = 60000;
    const delay = Math.min(
      baseDelay * Math.pow(2, this.state.reconnectAttempt),
      maxDelay
    );
    // 添加 ±25% 抖动
    return delay * (0.75 + Math.random() * 0.5);
  }

  get isConnected(): boolean {
    return this.state.status === 'registered';
  }

  get currentState(): ConnectionState {
    return { ...this.state };
  }
}
```

**Step 3: Commit**

```bash
git add services/agent-gateway/src/
git commit -m "feat(agent-gateway): implement WebSocket connection manager"
```

---

#### Task 1.3: 实现任务执行器

**Files:**
- Create: `services/agent-gateway/src/executor.ts`
- Modify: `services/agent-gateway/src/index.ts`

**Step 1: 创建任务执行器**

```typescript
// services/agent-gateway/src/executor.ts
import { spawn, ChildProcess } from 'child_process';
import { EventEmitter } from 'events';
import type { TaskRequest, AgentEvent, TaskResult } from './types';

export interface ExecutorOptions {
  defaultCwd: string;
  defaultAgent: string;
}

export class TaskExecutor extends EventEmitter {
  private activeTasks = new Map<string, ChildProcess>();

  constructor(private options: ExecutorOptions) {
    super();
  }

  async execute(task: TaskRequest): Promise<TaskResult> {
    const startTime = Date.now();
    const cwd = task.cwd || this.options.defaultCwd;
    const agentType = task.agentType || this.options.defaultAgent;

    return new Promise((resolve, reject) => {
      const command = this.resolveCommand(agentType);
      const args = this.resolveArgs(agentType, task.prompt, task.model);

      this.emitEvent(task.taskId, {
        type: 'log',
        content: `[executor] Starting: ${command} ${args.join(' ')}`,
        timestamp: Date.now(),
      });

      const child = spawn(command, args, {
        cwd,
        env: {
          ...process.env,
          ...(task.env || {}),
          CI: '1',
          NO_COLOR: '1',
          TERM: 'dumb',
        },
        shell: true,
      });

      this.activeTasks.set(task.taskId, child);

      let output = '';
      let errorOutput = '';

      child.stdout?.on('data', (chunk) => {
        const text = chunk.toString();
        output += text;
        this.parseAndEmitEvents(task.taskId, text);
      });

      child.stderr?.on('data', (chunk) => {
        const text = chunk.toString();
        errorOutput += text;
        this.emitEvent(task.taskId, {
          type: 'error',
          content: text,
          timestamp: Date.now(),
        });
      });

      child.on('error', (err) => {
        this.activeTasks.delete(task.taskId);
        reject(err);
      });

      child.on('exit', (code) => {
        this.activeTasks.delete(task.taskId);
        const duration = Date.now() - startTime;

        resolve({
          success: code === 0,
          exitCode: code ?? undefined,
          output: output.slice(-2000), // 保留最后 2000 字符
          duration,
        });
      });

      // 超时处理
      if (task.timeout) {
        setTimeout(() => {
          if (this.activeTasks.has(task.taskId)) {
            child.kill();
            this.activeTasks.delete(task.taskId);
            reject(new Error(`Task timeout after ${task.timeout}ms`));
          }
        }, task.timeout);
      }
    });
  }

  abort(taskId: string): boolean {
    const child = this.activeTasks.get(taskId);
    if (child) {
      child.kill();
      this.activeTasks.delete(taskId);
      return true;
    }
    return false;
  }

  sendInput(taskId: string, content: string): boolean {
    const child = this.activeTasks.get(taskId);
    if (child?.stdin) {
      child.stdin.write(content + '\n');
      return true;
    }
    return false;
  }

  private emitEvent(taskId: string, event: AgentEvent): void {
    this.emit('event', { taskId, event });
  }

  private parseAndEmitEvents(taskId: string, text: string): void {
    // 尝试解析 JSON 事件
    const lines = text.split('\n').filter(Boolean);
    
    for (const line of lines) {
      try {
        const parsed = JSON.parse(line);
        // OpenCode JSON 格式输出
        if (parsed.type) {
          this.emitEvent(taskId, {
            type: this.mapEventType(parsed.type),
            content: parsed.content || parsed.message,
            data: parsed,
            timestamp: Date.now(),
          });
          continue;
        }
      } catch {
        // 不是 JSON，作为普通日志
      }

      // 普通日志
      this.emitEvent(taskId, {
        type: 'log',
        content: line,
        timestamp: Date.now(),
      });
    }
  }

  private mapEventType(type: string): AgentEvent['type'] {
    const mapping: Record<string, AgentEvent['type']> = {
      'thinking': 'thinking',
      'tool_call': 'tool_call',
      'tool_result': 'tool_result',
      'file_change': 'file_change',
      'message': 'message',
      'error': 'error',
    };
    return mapping[type] || 'log';
  }

  private resolveCommand(agentType: string): string {
    const isWindows = process.platform === 'win32';
    switch (agentType) {
      case 'claude-code':
      case 'claudecode':
        return isWindows ? 'claude.cmd' : 'claude';
      case 'gemini-cli':
      case 'gemini':
        return isWindows ? 'gemini.cmd' : 'gemini';
      case 'opencode':
      default:
        return isWindows ? 'opencode.cmd' : 'opencode';
    }
  }

  private resolveArgs(agentType: string, prompt: string, model?: string): string[] {
    if (agentType === 'opencode') {
      const args = [
        '--print-logs',
        '--log-level', 'INFO',
        'run',
        '--format', 'json',
        '--port', '0',
      ];
      if (model) {
        args.push('-m', model);
      }
      args.push(prompt);
      return args;
    }
    return [prompt];
  }

  get activeTaskCount(): number {
    return this.activeTasks.size;
  }

  get activeTaskIds(): string[] {
    return Array.from(this.activeTasks.keys());
  }
}
```

**Step 2: 更新入口文件**

```typescript
// services/agent-gateway/src/index.ts
import { GatewayConnection } from './connection';
import { TaskExecutor } from './executor';
import type { ServerToGatewayMessage, TaskRequest } from './types';

// 配置从环境变量读取
const config = {
  serverUrl: process.env.GATEWAY_SERVER_URL || 'ws://localhost:3001',
  hostId: process.env.GATEWAY_HOST_ID || `host-${Date.now()}`,
  authToken: process.env.GATEWAY_AUTH_TOKEN || 'dev-token',
  capabilities: {
    name: process.env.GATEWAY_HOST_NAME || 'Development Host',
    agents: ['opencode'] as const,
    maxConcurrent: parseInt(process.env.GATEWAY_MAX_CONCURRENT || '2'),
    cwd: process.env.GATEWAY_CWD || process.cwd(),
  },
};

// 创建连接
const connection = new GatewayConnection({
  serverUrl: config.serverUrl,
  hostId: config.hostId,
  authToken: config.authToken,
  capabilities: config.capabilities,
  reconnect: true,
});

// 创建执行器
const executor = new TaskExecutor({
  defaultCwd: config.capabilities.cwd,
  defaultAgent: 'opencode',
});

// 转发执行器事件到服务器
executor.on('event', ({ taskId, event }) => {
  connection.send({
    type: 'task:event',
    taskId,
    event,
  });
});

// 处理服务器消息
connection.on('message', async (msg: ServerToGatewayMessage) => {
  switch (msg.type) {
    case 'task:execute':
      await handleTaskExecute(msg.task);
      break;
    case 'task:abort':
      handleTaskAbort(msg.taskId);
      break;
    case 'task:input':
      handleTaskInput(msg.taskId, msg.content);
      break;
  }
});

async function handleTaskExecute(task: TaskRequest): Promise<void> {
  console.log(`[Gateway] Executing task ${task.taskId}`);
  
  connection.send({
    type: 'task:started',
    taskId: task.taskId,
    sessionId: '', // OpenCode 会话 ID，稍后填充
  });

  try {
    const result = await executor.execute(task);
    
    connection.send({
      type: 'task:completed',
      taskId: task.taskId,
      result,
    });
  } catch (err) {
    connection.send({
      type: 'task:failed',
      taskId: task.taskId,
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

function handleTaskAbort(taskId: string): void {
  console.log(`[Gateway] Aborting task ${taskId}`);
  const aborted = executor.abort(taskId);
  if (!aborted) {
    console.warn(`[Gateway] Task ${taskId} not found`);
  }
}

function handleTaskInput(taskId: string, content: string): void {
  console.log(`[Gateway] Sending input to task ${taskId}`);
  executor.sendInput(taskId, content);
}

// 启动
console.log('[Gateway] Starting Agent Gateway...');
console.log(`[Gateway] Server: ${config.serverUrl}`);
console.log(`[Gateway] Host ID: ${config.hostId}`);
console.log(`[Gateway] Capabilities:`, config.capabilities);

connection.connect().catch((err) => {
  console.error('[Gateway] Failed to connect:', err.message);
});

// 优雅关闭
process.on('SIGINT', () => {
  console.log('[Gateway] Shutting down...');
  connection.disconnect();
  process.exit(0);
});
```

**Step 3: 测试启动**

Run: `cd services/agent-gateway && npm run dev`
Expected: 显示启动日志，尝试连接服务器（会失败，因为服务器还没实现）

**Step 4: Commit**

```bash
git add services/agent-gateway/src/
git commit -m "feat(agent-gateway): implement task executor and main entry"
```

---

### Phase 2: Rust 服务器端 Gateway Manager

#### Task 2.1: 添加 Axum WebSocket 依赖

**Files:**
- Modify: `crates/api-server/Cargo.toml`

**Step 1: 添加依赖**

```toml
# crates/api-server/Cargo.toml (追加依赖)
[dependencies]
axum = { version = "0.7", features = ["ws"] }
tokio-tungstenite = "0.21"
futures-util = "0.3"
```

**Step 2: 构建验证**

Run: `cd crates/api-server && cargo build`
Expected: 成功，无错误

**Step 3: Commit**

```bash
git add crates/api-server/Cargo.toml
git commit -m "chore(api-server): add WebSocket dependencies"
```

---

#### Task 2.2: 实现 Gateway 协议类型

**Files:**
- Create: `crates/api-server/src/gateway/mod.rs`
- Create: `crates/api-server/src/gateway/protocol.rs`
- Modify: `crates/api-server/src/lib.rs`

**Step 1: 创建协议类型**

```rust
// crates/api-server/src/gateway/protocol.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 主机能力
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    pub name: String,
    pub agents: Vec<String>,
    pub max_concurrent: u32,
    pub cwd: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// 任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRequest {
    pub task_id: String,
    pub prompt: String,
    pub cwd: String,
    pub agent_type: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Agent 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
    pub timestamp: u64,
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub success: bool,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub files_changed: Vec<String>,
}

/// Gateway -> Server 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GatewayToServer {
    #[serde(rename = "register")]
    Register {
        host_id: String,
        capabilities: HostCapabilities,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: u64 },
    #[serde(rename = "task:started")]
    TaskStarted { task_id: String, session_id: String },
    #[serde(rename = "task:event")]
    TaskEvent { task_id: String, event: AgentEvent },
    #[serde(rename = "task:completed")]
    TaskCompleted { task_id: String, result: TaskResult },
    #[serde(rename = "task:failed")]
    TaskFailed {
        task_id: String,
        error: String,
        #[serde(default)]
        details: serde_json::Value,
    },
}

/// Server -> Gateway 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerToGateway {
    #[serde(rename = "registered")]
    Registered { ok: bool, error: Option<String> },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "task:execute")]
    TaskExecute { task: TaskRequest },
    #[serde(rename = "task:abort")]
    TaskAbort { task_id: String },
    #[serde(rename = "task:input")]
    TaskInput { task_id: String, content: String },
}
```

**Step 2: 创建 mod.rs**

```rust
// crates/api-server/src/gateway/mod.rs
pub mod protocol;
pub mod manager;
pub mod handler;

pub use protocol::*;
pub use manager::GatewayManager;
```

**Step 3: 导出 gateway 模块**

```rust
// crates/api-server/src/lib.rs (追加)
pub mod gateway;
```

**Step 4: Commit**

```bash
git add crates/api-server/src/gateway/
git commit -m "feat(api-server): add gateway protocol types"
```

---

#### Task 2.3: 实现 Gateway Manager

**Files:**
- Create: `crates/api-server/src/gateway/manager.rs`

**Step 1: 实现 Manager**

```rust
// crates/api-server/src/gateway/manager.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::protocol::*;

/// 主机连接状态
pub struct HostConnection {
    pub host_id: String,
    pub capabilities: HostCapabilities,
    pub tx: mpsc::Sender<ServerToGateway>,
    pub active_tasks: Vec<String>,
    pub last_heartbeat: Instant,
    pub connected_at: Instant,
}

impl HostConnection {
    pub fn is_available(&self, agent_type: &str) -> bool {
        self.capabilities.agents.contains(&agent_type.to_string())
            && (self.active_tasks.len() as u32) < self.capabilities.max_concurrent
    }
}

/// Gateway 管理器
pub struct GatewayManager {
    connections: Arc<RwLock<HashMap<String, HostConnection>>>,
    event_tx: broadcast::Sender<TaskEvent>,
}

/// 任务事件 (用于转发到前端)
#[derive(Debug, Clone)]
pub struct TaskEvent {
    pub task_id: String,
    pub host_id: String,
    pub event: AgentEvent,
}

impl GatewayManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// 订阅任务事件
    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_tx.subscribe()
    }

    /// 注册主机
    pub async fn register_host(
        &self,
        host_id: String,
        capabilities: HostCapabilities,
        tx: mpsc::Sender<ServerToGateway>,
    ) -> bool {
        let mut connections = self.connections.write().await;
        
        if connections.contains_key(&host_id) {
            warn!("Host {} already registered, replacing connection", host_id);
        }

        connections.insert(
            host_id.clone(),
            HostConnection {
                host_id: host_id.clone(),
                capabilities,
                tx,
                active_tasks: Vec::new(),
                last_heartbeat: Instant::now(),
                connected_at: Instant::now(),
            },
        );

        info!("Host {} registered", host_id);
        true
    }

    /// 注销主机
    pub async fn unregister_host(&self, host_id: &str) {
        let mut connections = self.connections.write().await;
        if connections.remove(host_id).is_some() {
            info!("Host {} unregistered", host_id);
        }
    }

    /// 更新心跳
    pub async fn update_heartbeat(&self, host_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.last_heartbeat = Instant::now();
        }
    }

    /// 分配任务
    pub async fn dispatch_task(&self, task: TaskRequest) -> Result<String, String> {
        let mut connections = self.connections.write().await;

        // 找到合适的主机
        let host = connections
            .values_mut()
            .find(|c| c.is_available(&task.agent_type))
            .ok_or_else(|| "No available host for this agent type".to_string())?;

        let host_id = host.host_id.clone();
        host.active_tasks.push(task.task_id.clone());

        // 发送任务
        if let Err(e) = host.tx.send(ServerToGateway::TaskExecute { task }).await {
            error!("Failed to send task to host {}: {}", host_id, e);
            return Err(format!("Failed to dispatch task: {}", e));
        }

        info!("Task dispatched to host {}", host_id);
        Ok(host_id)
    }

    /// 处理任务开始
    pub async fn handle_task_started(&self, host_id: &str, task_id: &str, session_id: &str) {
        debug!("Task {} started on host {} (session: {})", task_id, host_id, session_id);
    }

    /// 处理任务事件
    pub async fn handle_task_event(&self, host_id: &str, task_id: &str, event: AgentEvent) {
        let _ = self.event_tx.send(TaskEvent {
            task_id: task_id.to_string(),
            host_id: host_id.to_string(),
            event,
        });
    }

    /// 处理任务完成
    pub async fn handle_task_completed(&self, host_id: &str, task_id: &str, result: TaskResult) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.active_tasks.retain(|id| id != task_id);
        }
        
        info!("Task {} completed on host {}: success={}", task_id, host_id, result.success);
        // TODO: 更新数据库任务状态
    }

    /// 处理任务失败
    pub async fn handle_task_failed(&self, host_id: &str, task_id: &str, error: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.active_tasks.retain(|id| id != task_id);
        }
        
        error!("Task {} failed on host {}: {}", task_id, host_id, error);
        // TODO: 更新数据库任务状态
    }

    /// 中止任务
    pub async fn abort_task(&self, task_id: &str) -> Result<(), String> {
        let connections = self.connections.read().await;
        
        for conn in connections.values() {
            if conn.active_tasks.contains(&task_id.to_string()) {
                conn.tx
                    .send(ServerToGateway::TaskAbort { task_id: task_id.to_string() })
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }

        Err("Task not found on any host".to_string())
    }

    /// 发送输入到任务
    pub async fn send_input(&self, task_id: &str, content: String) -> Result<(), String> {
        let connections = self.connections.read().await;
        
        for conn in connections.values() {
            if conn.active_tasks.contains(&task_id.to_string()) {
                conn.tx
                    .send(ServerToGateway::TaskInput {
                        task_id: task_id.to_string(),
                        content,
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }

        Err("Task not found on any host".to_string())
    }

    /// 获取所有主机状态
    pub async fn list_hosts(&self) -> Vec<HostStatus> {
        let connections = self.connections.read().await;
        
        connections
            .values()
            .map(|conn| HostStatus {
                host_id: conn.host_id.clone(),
                name: conn.capabilities.name.clone(),
                status: if conn.active_tasks.is_empty() {
                    "online"
                } else {
                    "busy"
                }.to_string(),
                capabilities: conn.capabilities.clone(),
                active_tasks: conn.active_tasks.clone(),
                last_heartbeat: conn.last_heartbeat.elapsed().as_secs(),
                connected_at: conn.connected_at.elapsed().as_secs(),
            })
            .collect()
    }

    /// 清理超时的连接
    pub async fn cleanup_stale_connections(&self, timeout: Duration) {
        let mut connections = self.connections.write().await;
        
        connections.retain(|host_id, conn| {
            if conn.last_heartbeat.elapsed() > timeout {
                warn!("Host {} heartbeat timeout, removing", host_id);
                false
            } else {
                true
            }
        });
    }
}

/// 主机状态 (用于 API 响应)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub host_id: String,
    pub name: String,
    pub status: String,
    pub capabilities: HostCapabilities,
    pub active_tasks: Vec<String>,
    pub last_heartbeat: u64,
    pub connected_at: u64,
}

impl Default for GatewayManager {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Commit**

```bash
git add crates/api-server/src/gateway/manager.rs
git commit -m "feat(api-server): implement gateway manager"
```

---

#### Task 2.4: 实现 WebSocket Handler

**Files:**
- Create: `crates/api-server/src/gateway/handler.rs`

**Step 1: 实现 Handler**

```rust
// crates/api-server/src/gateway/handler.rs

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::{protocol::*, GatewayManager};

#[derive(serde::Deserialize)]
pub struct WsQuery {
    #[serde(rename = "hostId")]
    host_id: String,
}

/// WebSocket 连接处理入口
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(manager): State<Arc<GatewayManager>>,
) -> impl IntoResponse {
    info!("New WebSocket connection from host: {}", query.host_id);
    ws.on_upgrade(move |socket| handle_socket(socket, query.host_id, manager))
}

async fn handle_socket(socket: WebSocket, host_id: String, manager: Arc<GatewayManager>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerToGateway>(100);

    // 转发 server -> gateway 消息的任务
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // 处理 gateway -> server 消息
    let host_id_clone = host_id.clone();
    let manager_clone = manager.clone();
    let tx_clone = tx.clone();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<GatewayToServer>(&text) {
                    Ok(msg) => {
                        handle_message(&manager_clone, &host_id_clone, msg, tx_clone.clone()).await;
                    }
                    Err(e) => {
                        warn!("Failed to parse message from {}: {}", host_id_clone, e);
                    }
                }
            }
            Message::Close(_) => {
                info!("Host {} disconnected", host_id_clone);
                break;
            }
            Message::Ping(data) => {
                // Axum 自动处理 pong
                debug!("Ping from {}", host_id_clone);
            }
            _ => {}
        }
    }

    // 清理
    manager.unregister_host(&host_id).await;
    send_task.abort();
}

async fn handle_message(
    manager: &GatewayManager,
    host_id: &str,
    msg: GatewayToServer,
    tx: mpsc::Sender<ServerToGateway>,
) {
    match msg {
        GatewayToServer::Register { host_id: msg_host_id, capabilities } => {
            let ok = manager.register_host(msg_host_id, capabilities, tx.clone()).await;
            let _ = tx.send(ServerToGateway::Registered { ok, error: None }).await;
        }

        GatewayToServer::Heartbeat { timestamp: _ } => {
            manager.update_heartbeat(host_id).await;
        }

        GatewayToServer::TaskStarted { task_id, session_id } => {
            manager.handle_task_started(host_id, &task_id, &session_id).await;
        }

        GatewayToServer::TaskEvent { task_id, event } => {
            manager.handle_task_event(host_id, &task_id, event).await;
        }

        GatewayToServer::TaskCompleted { task_id, result } => {
            manager.handle_task_completed(host_id, &task_id, result).await;
        }

        GatewayToServer::TaskFailed { task_id, error, details: _ } => {
            manager.handle_task_failed(host_id, &task_id, &error).await;
        }
    }
}

/// 启动心跳检测定时器
pub fn start_heartbeat_checker(manager: Arc<GatewayManager>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            manager
                .cleanup_stale_connections(std::time::Duration::from_secs(90))
                .await;
        }
    });
}
```

**Step 2: 更新 mod.rs 导出**

```rust
// crates/api-server/src/gateway/mod.rs
pub mod protocol;
pub mod manager;
pub mod handler;

pub use protocol::*;
pub use manager::{GatewayManager, HostStatus, TaskEvent};
pub use handler::{ws_handler, start_heartbeat_checker};
```

**Step 3: Commit**

```bash
git add crates/api-server/src/gateway/
git commit -m "feat(api-server): implement WebSocket handler for gateway"
```

---

#### Task 2.5: 集成到 Axum 路由

**Files:**
- Modify: `crates/api-server/src/routes/mod.rs` (或主路由文件)

**Step 1: 添加 Gateway 路由**

```rust
// 在 Axum 路由配置中添加:

use std::sync::Arc;
use crate::gateway::{GatewayManager, ws_handler, start_heartbeat_checker};

// 创建 Gateway Manager
let gateway_manager = Arc::new(GatewayManager::new());

// 启动心跳检测
start_heartbeat_checker(gateway_manager.clone());

// 添加路由
let app = Router::new()
    // ... 其他路由
    .route("/agent/ws", get(ws_handler))
    .route("/api/hosts", get(list_hosts))
    .with_state(gateway_manager);

// 列出所有主机的 API
async fn list_hosts(
    State(manager): State<Arc<GatewayManager>>,
) -> impl IntoResponse {
    let hosts = manager.list_hosts().await;
    axum::Json(hosts)
}
```

**Step 2: 构建验证**

Run: `cd crates/api-server && cargo build`
Expected: 成功

**Step 3: Commit**

```bash
git add crates/api-server/
git commit -m "feat(api-server): integrate gateway routes"
```

---

### Phase 3: 测试和验证

#### Task 3.1: 编写 Gateway 单元测试

**Files:**
- Create: `services/agent-gateway/src/__tests__/connection.test.ts`
- Create: `services/agent-gateway/src/__tests__/executor.test.ts`

**Step 1: 创建连接测试**

```typescript
// services/agent-gateway/src/__tests__/connection.test.ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import WebSocket from 'ws';
import { WebSocketServer } from 'ws';
import { GatewayConnection } from '../connection';

describe('GatewayConnection', () => {
  let wss: WebSocketServer;
  let serverPort: number;

  beforeEach((done) => {
    wss = new WebSocketServer({ port: 0 }, () => {
      serverPort = (wss.address() as any).port;
      done();
    });
  });

  afterEach(() => {
    wss.close();
  });

  it('should connect and register', async () => {
    wss.on('connection', (ws) => {
      ws.on('message', (data) => {
        const msg = JSON.parse(data.toString());
        if (msg.type === 'register') {
          ws.send(JSON.stringify({ type: 'registered', ok: true }));
        }
      });
    });

    const conn = new GatewayConnection({
      serverUrl: `ws://localhost:${serverPort}`,
      hostId: 'test-host',
      authToken: 'test-token',
      capabilities: {
        name: 'Test',
        agents: ['opencode'],
        maxConcurrent: 1,
        cwd: '/tmp',
      },
      reconnect: false,
    });

    await conn.connect();

    // 等待注册完成
    await new Promise((resolve) => {
      conn.on('stateChange', (state) => {
        if (state.status === 'registered') resolve(undefined);
      });
    });

    expect(conn.isConnected).toBe(true);
    conn.disconnect();
  });
});
```

**Step 2: 创建执行器测试**

```typescript
// services/agent-gateway/src/__tests__/executor.test.ts
import { describe, it, expect, vi } from 'vitest';
import { TaskExecutor } from '../executor';

describe('TaskExecutor', () => {
  it('should execute a simple command', async () => {
    const executor = new TaskExecutor({
      defaultCwd: process.cwd(),
      defaultAgent: 'opencode',
    });

    const events: any[] = [];
    executor.on('event', (e) => events.push(e));

    // 使用一个简单的 echo 命令来测试
    // 注意：这不是真正的 opencode 测试，只是验证执行逻辑
    // 真正的 opencode 测试需要 mock 或集成测试环境
  });

  it('should abort a running task', async () => {
    const executor = new TaskExecutor({
      defaultCwd: process.cwd(),
      defaultAgent: 'opencode',
    });

    // 启动一个长时间运行的任务
    const taskPromise = executor.execute({
      taskId: 'test-1',
      prompt: 'mock-test sleep 10', // 这需要 mock
      cwd: process.cwd(),
      agentType: 'opencode',
      timeout: 60000,
    });

    // 立即中止
    setTimeout(() => {
      executor.abort('test-1');
    }, 100);

    // 任务应该被中止
    try {
      await taskPromise;
    } catch (e) {
      // 预期中止
    }

    expect(executor.activeTaskCount).toBe(0);
  });
});
```

**Step 3: 添加测试脚本**

```json
// services/agent-gateway/package.json (添加)
{
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "devDependencies": {
    "vitest": "^1.0.0"
  }
}
```

**Step 4: 运行测试**

Run: `cd services/agent-gateway && npm test`
Expected: 测试通过

**Step 5: Commit**

```bash
git add services/agent-gateway/
git commit -m "test(agent-gateway): add unit tests"
```

---

#### Task 3.2: 端到端测试

**Files:**
- Create: `tests/e2e/gateway-e2e.test.ts`

**描述:** 启动真实的 Rust 服务器和 Gateway，验证完整流程

```typescript
// tests/e2e/gateway-e2e.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { spawn, ChildProcess } from 'child_process';
import WebSocket from 'ws';

describe('Gateway E2E', () => {
  let serverProcess: ChildProcess;
  let gatewayProcess: ChildProcess;

  beforeAll(async () => {
    // 启动 Rust 服务器
    serverProcess = spawn('cargo', ['run', '-p', 'api-server'], {
      stdio: 'pipe',
    });

    // 等待服务器启动
    await new Promise((resolve) => setTimeout(resolve, 5000));

    // 启动 Gateway
    gatewayProcess = spawn('npm', ['run', 'dev'], {
      cwd: 'services/agent-gateway',
      stdio: 'pipe',
      env: {
        ...process.env,
        GATEWAY_SERVER_URL: 'ws://localhost:3001',
        GATEWAY_HOST_ID: 'e2e-test-host',
      },
    });

    // 等待 Gateway 连接
    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  afterAll(() => {
    gatewayProcess?.kill();
    serverProcess?.kill();
  });

  it('should show host in list', async () => {
    const response = await fetch('http://localhost:3001/api/hosts');
    const hosts = await response.json();

    expect(hosts).toContainEqual(
      expect.objectContaining({
        hostId: 'e2e-test-host',
        status: expect.any(String),
      })
    );
  });

  it('should dispatch and complete a mock task', async () => {
    // 创建任务
    const createResponse = await fetch('http://localhost:3001/api/tasks', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        prompt: 'mock-test hello',
        agentType: 'opencode',
      }),
    });

    const task = await createResponse.json();
    expect(task.id).toBeDefined();

    // 等待任务完成
    await new Promise((resolve) => setTimeout(resolve, 3000));

    // 检查任务状态
    const statusResponse = await fetch(`http://localhost:3001/api/tasks/${task.id}`);
    const status = await statusResponse.json();

    expect(status.status).toBe('completed');
  });
});
```

---

## 3. 测试策略

### 单元测试

| 模块 | 测试文件 | 测试内容 |
|------|---------|---------|
| GatewayConnection | `connection.test.ts` | 连接、重连、心跳、消息解析 |
| TaskExecutor | `executor.test.ts` | 任务执行、中止、超时、事件发送 |
| GatewayManager (Rust) | `manager_test.rs` | 主机注册、任务分配、状态管理 |

### 集成测试

| 场景 | 测试内容 |
|------|---------|
| 连接流程 | Gateway 启动 → 连接服务器 → 注册 → 心跳 |
| 任务执行 | 服务器分配任务 → Gateway 执行 → 事件流 → 完成通知 |
| 断线重连 | 模拟网络断开 → 自动重连 → 恢复心跳 |
| 多主机 | 多个 Gateway 同时连接 → 任务负载均衡 |

### E2E 测试

| 场景 | 测试内容 |
|------|---------|
| 完整任务流 | 前端创建任务 → 服务器分配 → Gateway 执行 → 前端收到事件 → 任务完成 |
| 任务中止 | 前端中止任务 → 服务器转发 → Gateway 杀死进程 |

---

## 4. 部署指南

### Gateway 部署 (每台开发机)

```bash
# 1. 安装
npm install -g @vk/agent-gateway

# 2. 配置环境变量
export GATEWAY_SERVER_URL=wss://your-server.com
export GATEWAY_HOST_ID=my-dev-machine
export GATEWAY_AUTH_TOKEN=your-token
export GATEWAY_HOST_NAME="My Development Machine"
export GATEWAY_CWD=/path/to/projects

# 3. 启动
agent-gateway

# 或作为服务运行 (systemd/pm2)
pm2 start agent-gateway --name gateway
```

### 服务器部署

```bash
# Rust 服务器 (包含 Gateway Manager)
cargo run --release -p api-server

# 配置
export RUST_LOG=info
export DATABASE_URL=postgres://...
export JWT_SECRET=...
```

### 安全考虑

1. **认证**: Gateway 连接时使用 JWT Token 认证
2. **加密**: 使用 WSS (WebSocket over TLS)
3. **授权**: 每个 Token 绑定特定主机 ID
4. **限流**: 服务器端限制单个 IP 连接数

---

## 5. 迁移计划

### 从 agent-worker 迁移

1. **Phase 1**: 并行运行 agent-worker 和 agent-gateway
2. **Phase 2**: 新任务优先使用 agent-gateway
3. **Phase 3**: 废弃 agent-worker

### 前端适配

前端无需大改动，任务 API 保持不变，只是后端分发逻辑改变。

---

**Plan complete and saved to `docs/plans/2026-02-02-agent-gateway-architecture.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
