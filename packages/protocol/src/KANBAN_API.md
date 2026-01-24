# Kanban Protocol API 文档

## 概述

`kanban.ts` 模块定义了 Kanban 看板系统的核心类型、事件解析和工具函数。

---

## 类型定义

### KanbanTaskStatus

任务状态枚举。

```typescript
type KanbanTaskStatus = 'todo' | 'doing' | 'done';
```

### KanbanTask

单个任务的数据结构。

```typescript
interface KanbanTask {
  id: string;           // 唯一标识
  title: string;        // 任务标题
  status: KanbanTaskStatus;  // 当前状态
  description?: string; // 可选描述
  createdAt: number;    // 创建时间戳
}
```

### KanbanColumn

看板列的数据结构。

```typescript
interface KanbanColumn {
  id: KanbanTaskStatus;  // 列 ID (与状态一致)
  title: string;         // 显示标题
  taskIds: string[];     // 该列包含的任务 ID 列表
}
```

### KanbanBoardState

完整看板状态。

```typescript
interface KanbanBoardState {
  tasks: Record<string, KanbanTask>;           // 任务字典
  columns: Record<KanbanTaskStatus, KanbanColumn>; // 列字典
  columnOrder: KanbanTaskStatus[];             // 列顺序
}
```

### KanbanEvent

Socket 事件联合类型。

```typescript
type KanbanEvent = 
  | { type: 'kanban:sync'; payload: KanbanBoardState }
  | { type: 'kanban:create'; payload: { title: string; description?: string } }
  | { type: 'kanban:move'; payload: { taskId: string; targetStatus: KanbanTaskStatus; targetIndex?: number } }
  | { type: 'kanban:delete'; payload: { taskId: string } };
```

---

## 函数

### parseKanbanEvent

解析 JSON 字符串为 KanbanEvent。

**输入:**
| 参数 | 类型 | 说明 |
|------|------|------|
| raw | string | JSON 字符串 |

**输出:**
| 类型 | 说明 |
|------|------|
| KanbanEvent \| null | 有效事件或 null |

**示例:**
```typescript
const event = parseKanbanEvent('{"type":"kanban:create","payload":{"title":"新任务"}}');
// => { type: 'kanban:create', payload: { title: '新任务' } }

const invalid = parseKanbanEvent('not json');
// => null
```

---

### isKanbanEventType

类型守卫：检查字符串是否为有效事件类型。

**输入:** `value: string`

**输出:** `boolean` (同时收窄类型为 `KanbanEventType`)

**示例:**
```typescript
isKanbanEventType('kanban:sync');  // true
isKanbanEventType('unknown');      // false
```

---

### isKanbanTaskStatus

类型守卫：检查字符串是否为有效任务状态。

**输入:** `value: string`

**输出:** `boolean`

**示例:**
```typescript
isKanbanTaskStatus('todo');    // true
isKanbanTaskStatus('pending'); // false
```

---

### createEmptyBoardState

创建空的看板状态。

**输入:** 无

**输出:** `KanbanBoardState`

**示例:**
```typescript
const board = createEmptyBoardState();
// => {
//   tasks: {},
//   columns: {
//     todo: { id: 'todo', title: 'To Do', taskIds: [] },
//     doing: { id: 'doing', title: 'Doing', taskIds: [] },
//     done: { id: 'done', title: 'Done', taskIds: [] },
//   },
//   columnOrder: ['todo', 'doing', 'done']
// }
```

---

### generateTaskId

生成唯一任务 ID。

**输入:** 无

**输出:** `string` (格式: `task-{timestamp}-{random}`)

**示例:**
```typescript
const id = generateTaskId();
// => 'task-1706000000000-a3b2c1d'
```

---

## 内部逻辑

1. **parseKanbanEvent** 首先检查是否为 JSON 格式，然后验证 `type` 字段是否为已知事件类型，最后根据事件类型验证 `payload` 结构。
2. 验证函数 (`isValidBoardState`, `isValidCreatePayload` 等) 使用类型守卫模式确保运行时类型安全。
3. **generateTaskId** 结合时间戳和随机字符串确保唯一性。

---

## 测试覆盖

测试文件: `packages/protocol/tests/kanban.test.ts`

- 24 个测试用例
- 覆盖所有类型守卫、解析函数、工厂函数
- 覆盖边界情况 (无效 JSON、缺失字段、错误类型)
