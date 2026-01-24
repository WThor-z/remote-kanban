# Kanban Server API 文档

## 概述

`packages/server/src/kanban/` 模块提供 Kanban 看板的服务端状态管理和持久化功能。

---

## 模块结构

```
packages/server/src/kanban/
├── index.ts      # 模块导出
├── store.ts      # KanbanStore - 内存状态管理
└── manager.ts    # KanbanManager - 文件持久化管理
```

---

## KanbanStore

纯内存的 Kanban 状态管理器，不涉及 I/O。

### 构造函数

```typescript
const store = new KanbanStore(initialState?: KanbanBoardState);
```

### 方法

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `getState()` | - | `KanbanBoardState` | 获取当前状态 |
| `loadState(state)` | `KanbanBoardState` | `void` | 加载外部状态 |
| `createTask(title, description?)` | `string`, `string?` | `KanbanTask` | 创建任务到 todo 列 |
| `moveTask(taskId, targetStatus, targetIndex?)` | `string`, `KanbanTaskStatus`, `number?` | `void` | 移动任务 |
| `deleteTask(taskId)` | `string` | `void` | 删除任务 |
| `subscribe(callback)` | `(state) => void` | `() => void` | 订阅变更，返回取消订阅函数 |

### 使用示例

```typescript
import { KanbanStore } from './kanban';

const store = new KanbanStore();

// 订阅变更
const unsubscribe = store.subscribe((state) => {
  console.log('状态更新:', state);
});

// 创建任务
const task = store.createTask('实现登录功能', '用户名密码登录');

// 移动到进行中
store.moveTask(task.id, 'doing');

// 完成任务
store.moveTask(task.id, 'done');

// 取消订阅
unsubscribe();
```

---

## KanbanManager

带文件持久化的 Kanban 管理器，封装 `KanbanStore` 并自动同步到 `.opencode/kanban.json`。

### 构造函数

```typescript
const manager = new KanbanManager(projectDir: string);
```

- `projectDir`: 项目根目录路径
- 自动创建 `.opencode/kanban.json` 如果不存在
- 自动监听文件变更并同步

### 方法

与 `KanbanStore` 相同，额外提供：

| 方法 | 说明 |
|------|------|
| `dispose()` | 关闭文件监听器，释放资源 |

### 文件格式

`.opencode/kanban.json`:

```json
{
  "tasks": {
    "task-1234567890-abc123": {
      "id": "task-1234567890-abc123",
      "title": "任务标题",
      "status": "todo",
      "description": "任务描述",
      "createdAt": 1234567890000
    }
  },
  "columns": {
    "todo": { "id": "todo", "title": "To Do", "taskIds": ["task-1234567890-abc123"] },
    "doing": { "id": "doing", "title": "Doing", "taskIds": [] },
    "done": { "id": "done", "title": "Done", "taskIds": [] }
  },
  "columnOrder": ["todo", "doing", "done"]
}
```

---

## Socket 事件

服务器支持以下 Kanban 相关的 Socket.io 事件：

### 客户端 → 服务器

| 事件 | Payload | 说明 |
|------|---------|------|
| `kanban:request-sync` | - | 请求当前看板状态 |
| `kanban:create` | `{ title: string; description?: string }` | 创建新任务 |
| `kanban:move` | `{ taskId: string; targetStatus: KanbanTaskStatus; targetIndex?: number }` | 移动任务 |
| `kanban:delete` | `{ taskId: string }` | 删除任务 |

### 服务器 → 客户端

| 事件 | Payload | 说明 |
|------|---------|------|
| `kanban:sync` | `KanbanBoardState` | 广播完整状态（任何变更后触发） |
| `kanban:error` | `{ message: string }` | 操作错误 |

### 使用示例

```typescript
import { io } from 'socket.io-client';

const socket = io('http://localhost:3000');

// 请求同步
socket.emit('kanban:request-sync');

// 监听状态更新
socket.on('kanban:sync', (state) => {
  console.log('看板状态:', state);
});

// 创建任务
socket.emit('kanban:create', { title: '新任务' });

// 移动任务
socket.emit('kanban:move', { taskId: 'task-xxx', targetStatus: 'doing' });

// 删除任务
socket.emit('kanban:delete', { taskId: 'task-xxx' });

// 错误处理
socket.on('kanban:error', (err) => {
  console.error('Kanban 错误:', err.message);
});
```

---

## 内部逻辑

1. **KanbanStore** 使用不可变数据模式更新状态，每次操作创建新对象
2. **KanbanManager** 在构造时：
   - 检查 `.opencode/kanban.json` 是否存在
   - 存在则加载，不存在则创建默认状态
   - 启动 `fs.watch` 监听外部修改
3. 状态变更时：
   - 触发所有订阅回调
   - 自动保存到文件（带防抖）
4. Socket 集成：
   - 每次状态变更自动广播 `kanban:sync` 到所有客户端

---

## 测试覆盖

| 文件 | 测试数 | 说明 |
|------|--------|------|
| `kanban-store.test.ts` | 12 | Store 单元测试 |
| `kanban-manager.test.ts` | 7 | Manager 持久化测试 (Mocked FS) |
| `kanban-socket.test.ts` | 2 | Socket 集成测试 |

总计：**21 个测试**
