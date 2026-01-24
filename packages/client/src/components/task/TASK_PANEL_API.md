# TaskDetailPanel Component API

## Overview

`TaskDetailPanel` 是一个模态对话框组件，用于显示任务详情、执行历史和 AI 对话。用户可以在此启动 AI 执行任务、查看执行进度和与 AI 交互。

## Inputs

### Props

```typescript
interface TaskDetailPanelProps {
  /** 任务对象 */
  task: KanbanTask;
  /** 任务会话历史 */
  history: TaskSessionHistory | null;
  /** 当前执行状态 */
  status: AgentSessionStatus | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 关闭面板回调 */
  onClose: () => void;
  /** 开始执行回调 */
  onExecute: (taskId: string) => void;
  /** 停止执行回调 */
  onStop: (taskId: string) => void;
  /** 发送消息回调 */
  onSendMessage: (taskId: string, content: string) => void;
}
```

### Types

```typescript
interface KanbanTask {
  id: string;
  title: string;
  description?: string;
  status: 'todo' | 'doing' | 'done';
  createdAt: number;
  updatedAt: number;
  sessionId?: string;
}

type AgentSessionStatus = 
  | 'idle'      // 等待执行
  | 'starting'  // 启动中
  | 'running'   // 执行中
  | 'paused'    // 已暂停
  | 'completed' // 已完成
  | 'failed'    // 失败
  | 'aborted';  // 已中止
```

## Outputs

该组件为纯展示组件，通过回调函数与父组件通信：

| Callback | Trigger | Description |
|----------|---------|-------------|
| `onClose` | 点击关闭按钮或背景 | 关闭面板 |
| `onExecute` | 点击"开始执行"按钮 | 启动 AI 执行 |
| `onStop` | 点击"停止"按钮 | 中止执行 |
| `onSendMessage` | 提交消息表单 | 发送用户消息 |

## Usage Examples

### 基本使用

```tsx
import { TaskDetailPanel } from './task/TaskDetailPanel';

const ParentComponent = () => {
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);
  const { history, status, isLoading, error, executeTask, stopTask, sendMessage } = useTaskSession();

  return (
    <>
      {selectedTask && (
        <TaskDetailPanel
          task={selectedTask}
          history={history}
          status={status}
          isLoading={isLoading}
          error={error}
          onClose={() => setSelectedTask(null)}
          onExecute={executeTask}
          onStop={stopTask}
          onSendMessage={sendMessage}
        />
      )}
    </>
  );
};
```

### 完整集成示例

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
    <div className="min-h-screen bg-slate-900">
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
    </div>
  );
};
```

## Internal Logic

### 组件结构

```
TaskDetailPanel
├── Header
│   ├── 任务标题
│   ├── 状态指示器 (图标 + 文字)
│   └── 关闭按钮
│
├── Messages Area (可滚动)
│   ├── 任务描述卡片
│   ├── 加载指示器 (可选)
│   ├── 消息气泡列表
│   │   └── MessageBubble (user/assistant/system)
│   └── 错误提示 (可选)
│
└── Actions Footer
    ├── 控制按钮
    │   ├── "开始执行" (idle/completed/failed/aborted 时显示)
    │   └── "停止" (starting/running 时显示)
    │
    └── 消息输入框 (仅 running 时显示)
```

### 状态配置

```typescript
const statusConfig = {
  idle:      { icon: Clock,       label: '等待执行', color: 'text-slate-400' },
  starting:  { icon: Loader2,     label: '启动中',   color: 'text-amber-400' },
  running:   { icon: Loader2,     label: '执行中',   color: 'text-indigo-400' },
  paused:    { icon: Clock,       label: '已暂停',   color: 'text-amber-400' },
  completed: { icon: CheckCircle, label: '已完成',   color: 'text-emerald-400' },
  failed:    { icon: XCircle,     label: '失败',     color: 'text-rose-400' },
  aborted:   { icon: XCircle,     label: '已中止',   color: 'text-slate-400' },
};
```

### 自动滚动

- 当 `history.messages` 变化时，自动滚动到底部
- 使用 `scrollIntoView({ behavior: 'smooth' })`

### MessageBubble 子组件

```typescript
function MessageBubble({ message }: { message: ChatMessage }) {
  // 用户消息: 右对齐，紫色背景
  // AI 消息: 左对齐，深色背景
  // 系统消息: 左对齐，琥珀色边框
}
```

## Styling

- 使用 TailwindCSS
- 深色主题 (slate-800/900)
- 响应式设计 (max-w-2xl)
- 毛玻璃背景遮罩

## Dependencies

- `lucide-react`: 图标库
- `@opencode-vibe/protocol`: 类型定义
