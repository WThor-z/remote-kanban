# Agent Module

AI 代理执行模块，通过 OpenCode HTTP API 与 AI 编码助手通信。

## 模块结构

```
packages/server/src/agent/
├── index.ts              # 模块导出
├── executor.ts           # AgentExecutor - 会话管理
├── opencode-client.ts    # OpencodeClient - HTTP API 客户端
└── parser.ts             # AgentOutputParser - 输出解析
```

## 组件说明

### OpencodeClient

与 OpenCode CLI 的 HTTP API 通信的客户端。

#### 接受参数 (Inputs)

```typescript
interface OpencodeServerConfig {
  cwd?: string;                    // 工作目录，默认 process.cwd()
  env?: Record<string, string>;    // 环境变量
}
```

#### 输出参数 (Outputs)

事件驱动，通过 EventEmitter 发出以下事件：

| 事件 | 数据 | 说明 |
|------|------|------|
| `ready` | `string` (URL) | 服务器启动完成 |
| `output` | `{ type: 'stdout' \| 'stderr', data: string }` | 进程输出 |
| `event` | `OpencodeEvent` | OpenCode SSE 事件 |
| `session` | `{ id: string, baseUrl: string }` | 会话创建完成 |
| `idle` | - | 会话空闲 |
| `done` | - | 执行完成 |
| `error` | `Error` | 错误发生 |
| `exit` | `number` (exit code) | 进程退出 |

#### 使用示例

```typescript
import { OpencodeClient } from './opencode-client';

const client = new OpencodeClient({ cwd: '/path/to/project' });

// 监听事件
client.on('event', (event) => {
  if (event.type === 'message.part.updated') {
    const part = event.properties?.part as { text?: string };
    console.log('AI:', part?.text);
  }
});

client.on('idle', () => {
  console.log('Session completed');
  client.stop();
});

// 运行完整会话
await client.run('帮我写一个 Hello World');
```

#### 内部逻辑

1. **启动服务器**: 执行 `opencode serve --hostname 127.0.0.1 --port 0`
2. **等待就绪**: 解析 stdout 中的 `opencode server listening on <url>`
3. **健康检查**: 轮询 `GET /global/health` 直到返回 `{ healthy: true }`
4. **创建会话**: `POST /session?directory=<dir>`
5. **连接事件流**: `GET /event` (SSE)
6. **发送消息**: `POST /session/{id}/message`
7. **处理事件**: 解析 SSE 数据流，发出对应事件
8. **会话结束**: 收到 `session.idle` 事件时完成

---

### AgentExecutor

管理多个 AI 代理会话的执行器。

#### 接受参数 (Inputs)

```typescript
interface AgentSession {
  id: string;
  agentType: AgentType;
  status: AgentSessionStatus;
  prompt: string;
  taskId?: string;
  startedAt?: number;
  endedAt?: number;
  error?: string;
}

interface AgentConfig {
  type: string;
  name: string;
  command: string;
  args: string[];
  cwd?: string;
  env?: Record<string, string>;
}
```

#### 输出参数 (Outputs)

事件回调：

```typescript
// 输出事件
executor.onOutput((event: AgentOutputEvent) => {
  // { sessionId, type, data, timestamp }
});

// 状态变更事件
executor.onStatus((event: AgentStatusEvent) => {
  // { sessionId, previousStatus, currentStatus, timestamp, error? }
});
```

#### 使用示例

```typescript
import { AgentExecutor } from './executor';
import { createAgentSession, AGENT_PRESETS } from '@opencode-vibe/protocol';

const executor = new AgentExecutor();

// 注册事件处理
executor.onOutput((event) => {
  console.log(`[${event.sessionId}] ${event.data}`);
});

executor.onStatus((event) => {
  console.log(`Session ${event.sessionId}: ${event.previousStatus} -> ${event.currentStatus}`);
});

// 启动会话
const session = createAgentSession('opencode', '帮我写一个 React 组件');
await executor.start(session, { ...AGENT_PRESETS.opencode, cwd: process.cwd() });

// 停止会话
await executor.stop(session.id);
```

#### 内部逻辑

1. **创建 OpencodeClient**: 根据配置初始化客户端
2. **注册事件监听**: 转发 client 事件到 executor 事件
3. **状态管理**: 跟踪会话状态 (idle → starting → running → completed/failed/aborted)
4. **多会话隔离**: 每个会话独立的 OpencodeClient 实例

---

### AgentOutputParser

解析 AI 输出中的任务指令。

#### 接受参数 (Inputs)

```typescript
parseChunk(chunk: string): ParseResult[]
```

#### 输出参数 (Outputs)

```typescript
interface ParseResult {
  raw: string;
  taskDetected?: {
    action: 'create' | 'move' | 'delete';
    taskTitle: string;
    targetStatus?: KanbanTaskStatus;
  };
}
```

#### 使用示例

```typescript
import { AgentOutputParser } from './parser';

const parser = new AgentOutputParser();

const results = parser.parseChunk('[TASK:create] 实现用户登录功能');
// results[0].taskDetected = { action: 'create', taskTitle: '实现用户登录功能' }
```

---

## Socket 事件

服务器通过 Socket.io 暴露以下事件：

### 客户端 → 服务器

| 事件 | 数据 | 说明 |
|------|------|------|
| `agent:start` | `{ agentType, prompt, taskId? }` | 启动代理会话 |
| `agent:stop` | `{ sessionId }` | 停止会话 |
| `agent:input` | `{ sessionId, data }` | 发送输入 (不支持) |
| `agent:list` | - | 获取所有会话 |

### 服务器 → 客户端

| 事件 | 数据 | 说明 |
|------|------|------|
| `agent:session` | `AgentSession` | 会话状态更新 |
| `agent:output` | `AgentOutputEvent` | 输出数据 |
| `agent:status` | `AgentStatusEvent` | 状态变更 |
| `agent:error` | `{ message, sessionId? }` | 错误信息 |
| `agent:sessions` | `AgentSession[]` | 所有会话列表 |
| `agent:task-detected` | `{ sessionId, action, taskTitle }` | 检测到任务指令 |

---

## OpenCode 事件格式

OpenCode 1.1.x 发送的 SSE 事件类型：

| 事件类型 | 说明 | 提取内容 |
|----------|------|----------|
| `message.part.updated` | 消息片段更新 | `properties.part.text` (文本内容) |
| `session.idle` | 会话空闲 | 表示会话完成 |
| `session.error` | 会话错误 | `properties.error.message` |
| `session.status` | 状态变更 | `properties.status.type` |
| `tool.start` | 工具开始 | `properties.name` |

---

## 测试

```bash
# 运行 agent 模块测试
npm test --workspace=packages/server -- --run tests/agent-*.test.ts tests/opencode-client.test.ts

# 测试覆盖
# - agent-executor.test.ts: 24 tests
# - agent-parser.test.ts: 27 tests
# - opencode-client.test.ts: 28 tests
```
