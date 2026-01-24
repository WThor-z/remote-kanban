# Command Parser API

## 概述

`commandParser.ts` 提供 Kanban 指令解析功能，用于拦截用户在 InputBar 中输入的 `/task` 和 `/todo` 指令。

## 导出

### 类型

```typescript
interface ParsedCommand {
  type: 'task' | 'todo';
  action: 'add' | 'move' | 'delete' | 'list';
  args: {
    title?: string;
    taskId?: string;
    targetColumn?: string;
  };
}
```

### 函数

#### `isKanbanCommand(input: string): boolean`

判断输入是否为 Kanban 指令。

**参数:**
- `input` - 用户输入字符串

**返回:** `true` 如果输入以 `/task` 或 `/todo` 开头

**示例:**
```typescript
isKanbanCommand('/task add Buy milk')  // true
isKanbanCommand('/todo Fix bug')       // true
isKanbanCommand('echo hello')          // false
```

---

#### `parseKanbanCommand(input: string): ParsedCommand | null`

解析 Kanban 指令，提取动作和参数。

**参数:**
- `input` - 用户输入字符串

**返回:** `ParsedCommand` 对象，或 `null`（如果解析失败）

**支持的指令格式:**

| 指令 | 说明 | 示例 |
|------|------|------|
| `/task add <title>` | 添加新任务 | `/task add 修复登录 bug` |
| `/todo <title>` | 添加新任务（简写） | `/todo 写单元测试` |
| `/task move <id> <column>` | 移动任务到指定列 | `/task move abc123 done` |
| `/task delete <id>` | 删除任务 | `/task delete abc123` |
| `/task list` | 列出所有任务 | `/task list` |

**列名映射:**

| 输入 | 映射到 |
|------|--------|
| `todo`, `待办` | `todo` |
| `progress`, `进行中`, `doing` | `in-progress` |
| `done`, `完成`, `已完成` | `done` |

---

## 使用示例

### 在 InputBar 中拦截指令

```typescript
import { isKanbanCommand, parseKanbanCommand } from '../utils/commandParser';
import { useKanban } from '../hooks/useKanban';

function InputBar() {
  const { createTask, moveTask, deleteTask } = useKanban();

  const handleSubmit = (input: string) => {
    if (isKanbanCommand(input)) {
      const cmd = parseKanbanCommand(input);
      if (!cmd) return;

      switch (cmd.action) {
        case 'add':
          if (cmd.args.title) {
            createTask({ title: cmd.args.title, column: 'todo' });
          }
          break;
        case 'move':
          if (cmd.args.taskId && cmd.args.targetColumn) {
            moveTask(cmd.args.taskId, cmd.args.targetColumn);
          }
          break;
        case 'delete':
          if (cmd.args.taskId) {
            deleteTask(cmd.args.taskId);
          }
          break;
      }
      return; // 不发送到 PTY
    }

    // 正常发送到 PTY
    sendToPty(input);
  };
}
```

---

## 测试覆盖

- 12 个单元测试覆盖所有指令格式
- 测试文件: `src/utils/__tests__/commandParser.test.ts`

## 相关文件

- `src/components/InputBar.tsx` - 指令拦截实现
- `src/hooks/useKanban.ts` - Kanban 操作 hook
- `packages/protocol/src/kanban.ts` - 类型定义
