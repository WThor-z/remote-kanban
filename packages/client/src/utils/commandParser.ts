import type { KanbanTaskStatus } from '@opencode-vibe/protocol';

export type KanbanCommand =
  | { type: 'kanban:create'; payload: { title: string; description?: string } }
  | { type: 'kanban:move'; payload: { taskId: string; targetStatus: KanbanTaskStatus } }
  | { type: 'kanban:delete'; payload: { taskId: string } };

const VALID_STATUSES: KanbanTaskStatus[] = ['todo', 'doing', 'done'];

/**
 * 解析用户输入，识别 Kanban 指令
 * 支持的指令:
 *   /task <title> [| <description>]     - 简洁创建
 *   /task add <title> [-- <description>] - 完整格式
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

    // 检查是否是子命令 (add, move, done, delete, rm)
    const knownSubCommands = ['add', 'move', 'done', 'delete', 'rm'];
    
    if (knownSubCommands.includes(subCommand)) {
      // 使用现有的子命令逻辑
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
    } else {
      // /task <title> [| <description>] - 简洁创建格式
      const rest = trimmed.slice('/task'.length).trim();
      if (!rest) return null;

      // 支持 | 或 -- 作为分隔符
      let titlePart: string;
      let descPart: string | undefined;

      if (rest.includes('|')) {
        [titlePart, descPart] = rest.split('|').map((s) => s.trim());
      } else if (rest.includes('--')) {
        [titlePart, descPart] = rest.split('--').map((s) => s.trim());
      } else {
        titlePart = rest;
      }

      if (!titlePart) return null;

      return {
        type: 'kanban:create',
        payload: {
          title: titlePart,
          ...(descPart && { description: descPart }),
        },
      };
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
