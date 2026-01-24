import type { KanbanTaskStatus } from '@opencode-vibe/protocol';

export type KanbanCommand =
  | { type: 'kanban:create'; payload: { title: string; description?: string } }
  | { type: 'kanban:move'; payload: { taskId: string; targetStatus: KanbanTaskStatus } }
  | { type: 'kanban:delete'; payload: { taskId: string } };

const VALID_STATUSES: KanbanTaskStatus[] = ['todo', 'doing', 'done'];

/**
 * 解析用户输入，识别 Kanban 指令
 * 支持的指令:
 *   /task add <title> [-- <description>]
 *   /todo <title> (别名)
 *   /task move <taskId> <status>
 *   /task done <taskId> (快捷方式)
 *   /task delete <taskId>
 *   /task rm <taskId> (别名)
 * 
 * @returns KanbanCommand 如果是有效的 Kanban 指令，否则返回 null
 */
export const parseCommand = (input: string): KanbanCommand | null => {
  const trimmed = input.trim();
  
  // 检查是否以 / 开头
  if (!trimmed.startsWith('/')) {
    return null;
  }

  // 分割并规范化
  const parts = trimmed.split(/\s+/).filter(Boolean);
  const command = parts[0]?.toLowerCase();

  // /todo <title> - 快捷创建
  if (command === '/todo') {
    const title = parts.slice(1).join(' ').trim();
    if (!title) return null;
    return { type: 'kanban:create', payload: { title } };
  }

  // /task 指令
  if (command === '/task') {
    const subCommand = parts[1]?.toLowerCase();
    
    if (!subCommand) return null;

    switch (subCommand) {
      case 'add': {
        // /task add <title> [-- <description>]
        const rest = parts.slice(2).join(' ');
        if (!rest.trim()) return null;

        const [titlePart, descPart] = rest.split('--').map((s) => s.trim());
        if (!titlePart) return null;

        return {
          type: 'kanban:create',
          payload: {
            title: titlePart,
            ...(descPart && { description: descPart }),
          },
        };
      }

      case 'move': {
        // /task move <taskId> <status>
        const taskId = parts[2];
        const targetStatus = parts[3]?.toLowerCase() as KanbanTaskStatus;
        
        if (!taskId || !targetStatus) return null;
        if (!VALID_STATUSES.includes(targetStatus)) return null;

        return {
          type: 'kanban:move',
          payload: { taskId, targetStatus },
        };
      }

      case 'done': {
        // /task done <taskId> - 快捷移动到 done
        const taskId = parts[2];
        if (!taskId) return null;

        return {
          type: 'kanban:move',
          payload: { taskId, targetStatus: 'done' },
        };
      }

      case 'delete':
      case 'rm': {
        // /task delete <taskId> 或 /task rm <taskId>
        const taskId = parts[2];
        if (!taskId) return null;

        return {
          type: 'kanban:delete',
          payload: { taskId },
        };
      }

      default:
        return null;
    }
  }

  return null;
};

/**
 * 检查输入是否为 Kanban 指令
 */
export const isKanbanCommand = (input: string): boolean => {
  return parseCommand(input) !== null;
};
