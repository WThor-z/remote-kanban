# Kanban Client UI 文档

## 概述

`packages/client/src/components/kanban/` 提供可拖拽的看板 UI 组件。

---

## 组件

### TaskCard

单个任务卡片组件。

```tsx
import { TaskCard } from './components/kanban';

<TaskCard
  task={task}
  onDelete={(taskId) => console.log('删除:', taskId)}
  isDragging={false}
/>
```

**Props:**

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task` | `KanbanTask` | ✅ | 任务数据 |
| `onDelete` | `(taskId: string) => void` | - | 删除回调 |
| `isDragging` | `boolean` | - | 是否正在拖拽 |

---

### KanbanColumn

单列组件（内部使用，支持拖放区域）。

---

### KanbanBoard

完整的三列看板，支持拖拽排序。

```tsx
import { KanbanBoard } from './components/kanban';

<KanbanBoard
  board={boardState}
  onMoveTask={(taskId, targetStatus, targetIndex) => {
    // 处理任务移动
  }}
  onDeleteTask={(taskId) => {
    // 处理任务删除
  }}
/>
```

**Props:**

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `board` | `KanbanBoardState` | ✅ | 看板状态 |
| `onMoveTask` | `(taskId, targetStatus, targetIndex?) => void` | ✅ | 移动任务回调 |
| `onDeleteTask` | `(taskId: string) => void` | ✅ | 删除任务回调 |

---

## Hook

### useKanban

管理看板状态和 Socket 通信。

```tsx
import { useKanban } from './hooks/useKanban';

const { board, isLoading, createTask, moveTask, deleteTask } = useKanban();
```

**返回值:**

| 属性 | 类型 | 说明 |
|------|------|------|
| `board` | `KanbanBoardState` | 当前看板状态 |
| `isLoading` | `boolean` | 是否正在加载 |
| `createTask` | `(title, description?) => void` | 创建任务 |
| `moveTask` | `(taskId, targetStatus, targetIndex?) => void` | 移动任务 |
| `deleteTask` | `(taskId) => void` | 删除任务 |

---

## 完整使用示例

```tsx
import { useKanban } from './hooks/useKanban';
import { KanbanBoard } from './components/kanban';

const KanbanPage = () => {
  const { board, isLoading, createTask, moveTask, deleteTask } = useKanban();

  if (isLoading) {
    return <div>加载中...</div>;
  }

  return (
    <div>
      <button onClick={() => createTask('新任务')}>
        添加任务
      </button>
      <KanbanBoard
        board={board}
        onMoveTask={moveTask}
        onDeleteTask={deleteTask}
      />
    </div>
  );
};
```

---

## 拖拽行为

- 使用 `@dnd-kit/core` 和 `@dnd-kit/sortable` 实现
- 支持鼠标和键盘拖拽
- 拖拽时显示半透明预览
- 释放后自动计算目标列和索引
- 通过 Socket 同步到服务器

---

## 测试覆盖

| 文件 | 测试数 |
|------|--------|
| `TaskCard.test.tsx` | 5 |
| `KanbanBoard.test.tsx` | 4 |
| `useKanban.test.ts` | 5 |
| **总计** | **14** |
