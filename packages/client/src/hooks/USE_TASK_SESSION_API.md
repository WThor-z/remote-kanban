# useTaskSession Hook API

## Overview

`useTaskSession` 是一个 React Hook，用于管理任务详情面板的状态和与服务端的通信。它处理任务选择、历史加载、执行控制和消息发送。

## Inputs

### Options

```typescript
interface UseTaskSessionOptions {
  /** Socket.io 客户端实例 */
  socket: Socket | undefined;
  /** 是否已连接到服务器 */
  isConnected: boolean;
}
```

## Outputs

### Return Value

```typescript
interface UseTaskSessionReturn {
  /** 当前选中的任务 ID */
  selectedTaskId: string | null;
  /** 任务会话历史 */
  history: TaskSessionHistory | null;
  /** 任务执行状态 */
  status: AgentSessionStatus | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 选中任务 - 传入 null 关闭面板 */
  selectTask: (taskId: string | null) => void;
  /** 执行任务 - 启动 AI Agent */
  executeTask: (taskId: string) => void;
  /** 停止任务 */
  stopTask: (taskId: string) => void;
  /** 发送消息到执行中的任务 */
  sendMessage: (taskId: string, content: string) => void;
}
```

### Types

```typescript
type AgentSessionStatus = 
  | 'idle'      // 等待执行
  | 'starting'  // 启动中
  | 'running'   // 执行中
  | 'paused'    // 已暂停
  | 'completed' // 已完成
  | 'failed'    // 失败
  | 'aborted';  // 已中止

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
}

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
}
```

## Usage Examples

### 基本使用

```tsx
import { useTaskSession } from '../hooks/useTaskSession';
import { useOpencode } from '../hooks/useOpencode';

const TaskManager = () => {
  const { socket, isConnected } = useOpencode();
  const {
    selectedTaskId,
    history,
    status,
    isLoading,
    error,
    selectTask,
    executeTask,
    stopTask,
  } = useTaskSession({ socket, isConnected });

  return (
    <div>
      {/* 点击任务卡片选中 */}
      <TaskCard onClick={() => selectTask('task-123')} />

      {/* 任务详情面板 */}
      {selectedTaskId && (
        <TaskDetailPanel
          history={history}
          status={status}
          isLoading={isLoading}
          error={error}
          onClose={() => selectTask(null)}
          onExecute={executeTask}
          onStop={stopTask}
        />
      )}
    </div>
  );
};
```

### 与 App.tsx 集成

```tsx
// App.tsx
const App = () => {
  const { socket, isConnected } = useOpencode();
  const { tasks } = useKanban(socket, isConnected);
  const taskSession = useTaskSession({ socket, isConnected });

  const selectedTask = taskSession.selectedTaskId
    ? tasks[taskSession.selectedTaskId]
    : null;

  return (
    <>
      <KanbanBoard
        tasks={tasks}
        onTaskClick={(task) => taskSession.selectTask(task.id)}
      />

      {selectedTask && (
        <TaskDetailPanel
          task={selectedTask}
          history={taskSession.history}
          status={taskSession.status}
          isLoading={taskSession.isLoading}
          error={taskSession.error}
          onClose={() => taskSession.selectTask(null)}
          onExecute={taskSession.executeTask}
          onStop={taskSession.stopTask}
          onSendMessage={taskSession.sendMessage}
        />
      )}
    </>
  );
};
```

## Internal Logic

### Socket Events

| Event | Direction | Description |
|-------|-----------|-------------|
| `task:history` | emit | 请求任务历史 |
| `task:execute` | emit | 开始执行任务 |
| `task:stop` | emit | 停止任务执行 |
| `task:message` | emit | 发送用户消息 |
| `task:status` | receive | 状态更新 |
| `task:message` | receive | 新消息 |
| `task:history` | receive | 历史数据 |
| `task:error` | receive | 错误信息 |

### 状态管理流程

```
selectTask(taskId)
    │
    ├─► 设置 selectedTaskId
    │
    ├─► 重置 history, status, error
    │
    └─► emit 'task:history' 请求历史
           │
           ▼
    收到 'task:history'
           │
           ├─► 设置 history
           │
           └─► 设置 isLoading = false

executeTask(taskId)
    │
    └─► emit 'task:execute'
           │
           ▼
    收到 'task:status' (starting)
           │
           └─► 设置 status, isLoading
                  │
                  ▼
           收到 'task:message' (AI 输出)
                  │
                  └─► 追加到 history.messages
```

### 消息更新策略

- 新消息：追加到 `history.messages` 数组
- 同 ID 消息：更新最后一条（用于流式输出）

## Dependencies

- `socket.io-client`: WebSocket 通信
- `@opencode-vibe/protocol`: 类型定义
