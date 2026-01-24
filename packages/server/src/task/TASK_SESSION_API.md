# Task Session Manager API

## Overview

`TaskSessionManager` 负责管理 Kanban 任务与 AI Agent 会话之间的关联。当用户点击"开始执行"时，它会启动一个 Agent 会话来处理任务，并将执行历史持久化到文件系统。

## Inputs

### Constructor Options

```typescript
interface TaskSessionManagerOptions {
  /** 任务历史存储目录 (如 .opencode/tasks/) */
  tasksDir: string;
  /** Kanban Store 实例 - 用于更新任务状态 */
  kanbanStore: KanbanStore;
  /** Agent Executor 实例 - 用于执行 AI 会话 */
  agentExecutor: AgentExecutor;
}
```

### Methods

| Method | Parameters | Description |
|--------|------------|-------------|
| `initialize()` | - | 初始化，确保任务目录存在 |
| `executeTask(taskId)` | `taskId: string` | 启动任务的 Agent 会话 |
| `stopTask(taskId)` | `taskId: string` | 停止正在执行的任务 |
| `sendMessage(taskId, content)` | `taskId: string, content: string` | 发送用户消息到执行中的任务 |
| `getHistory(taskId)` | `taskId: string` | 获取任务的会话历史 |
| `on(event, callback)` | `event: EventType, callback: Function` | 注册事件监听器 |

## Outputs

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `status` | `{ taskId: string, status: AgentSessionStatus }` | 任务执行状态变更 |
| `message` | `{ taskId: string, message: ChatMessage }` | 新消息（用户或 AI） |
| `error` | `{ taskId: string, error: string }` | 执行错误 |

### Return Types

```typescript
// getHistory() 返回类型
interface TaskSessionHistory {
  taskId: string;
  sessionId: string;
  title: string;
  description: string;
  messages: ChatMessage[];
  status: AgentSessionStatus;
  createdAt: number;
  startedAt?: number;
  completedAt?: number;
  error?: string;
  stats?: { duration?: number; totalTokens?: number };
}
```

## Usage Examples

### 基本使用

```typescript
import { TaskSessionManager } from './session-manager';
import { KanbanStore } from '../kanban/store';
import { AgentExecutor } from '../agent/executor';

// 初始化
const manager = new TaskSessionManager({
  tasksDir: '.opencode/tasks',
  kanbanStore: new KanbanStore(),
  agentExecutor: new AgentExecutor(),
});

await manager.initialize();

// 注册事件监听
manager.on('status', ({ taskId, status }) => {
  console.log(`Task ${taskId} status: ${status}`);
});

manager.on('message', ({ taskId, message }) => {
  console.log(`[${message.role}]: ${message.content}`);
});

// 执行任务
await manager.executeTask('task-123');

// 停止任务
await manager.stopTask('task-123');
```

### 与 Socket.io 集成

```typescript
// server.ts
socket.on('task:execute', async ({ taskId }) => {
  await taskSessionManager.executeTask(taskId);
});

taskSessionManager.on('status', (payload) => {
  io.emit('task:status', payload);
});

taskSessionManager.on('message', (payload) => {
  io.emit('task:message', payload);
});
```

## Internal Logic

### 执行流程

```
executeTask(taskId)
    │
    ├─► 1. 从 KanbanStore 获取任务信息
    │
    ├─► 2. 创建/加载 TaskSessionHistory
    │
    ├─► 3. 添加用户消息 (任务描述)
    │
    ├─► 4. 更新 Kanban 状态为 'doing'
    │
    ├─► 5. 启动 AgentExecutor
    │       │
    │       ├─► 成功: 监听 Agent 输出，转换为 ChatMessage
    │       │
    │       └─► 失败: 恢复任务状态为 'todo'，发送错误事件
    │
    └─► 6. Agent 完成时，更新状态为 'done'
```

### 持久化

- 历史存储路径: `.opencode/tasks/{taskId}.json`
- 采用防抖保存 (1 秒) 避免频繁写入
- 内存缓存活跃任务的历史

### 状态转换

```
                ┌──────────────────────────┐
                │                          │
                ▼                          │
┌─────┐     ┌────────┐     ┌─────────┐     │
│ idle │ ──► │starting│ ──► │ running │ ────┤
└─────┘     └────────┘     └─────────┘     │
    ▲                           │          │
    │                           ▼          │
    │     ┌─────────┐     ┌──────────┐     │
    └──── │ failed  │ ◄── │completed │ ◄───┘
          └─────────┘     └──────────┘
                ▲              ▲
                │              │
          ┌─────────┐          │
          │ aborted │ ─────────┘
          └─────────┘
```

## Dependencies

- `@opencode-vibe/protocol`: 类型定义和工厂函数
- `../agent/executor`: Agent 执行器
- `../kanban/store`: Kanban 状态管理
