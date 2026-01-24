import type { KanbanTask } from '@opencode-vibe/protocol';
import { Trash2, Play, Loader2 } from 'lucide-react';

interface TaskCardProps {
  task: KanbanTask;
  onDelete?: (taskId: string) => void;
  onClick?: (task: KanbanTask) => void;
  isDragging?: boolean;
  isExecuting?: boolean;
}

const statusColors = {
  todo: 'border-l-slate-400',
  doing: 'border-l-amber-400',
  done: 'border-l-emerald-400',
} as const;

export const TaskCard = ({ task, onDelete, onClick, isDragging = false, isExecuting = false }: TaskCardProps) => {
  const handleClick = (e: React.MouseEvent) => {
    // 阻止删除按钮触发卡片点击
    if ((e.target as HTMLElement).closest('button')) return;
    onClick?.(task);
  };

  return (
    <div
      data-testid="task-card"
      onClick={handleClick}
      className={`
        bg-slate-800 rounded-lg p-3 border-l-4 shadow-md cursor-pointer
        ${statusColors[task.status]}
        ${isDragging ? 'opacity-50 shadow-lg ring-2 ring-indigo-500' : ''}
        ${onClick ? 'hover:bg-slate-750 hover:ring-1 hover:ring-indigo-500/50' : ''}
        transition-all duration-150
      `}
    >
      <div className="flex justify-between items-start gap-2">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            {isExecuting && (
              <Loader2 size={14} className="animate-spin text-indigo-400 flex-shrink-0" />
            )}
            <h4 className="text-sm font-medium text-slate-100 truncate">
              {task.title}
            </h4>
          </div>
        </div>
        <div className="flex items-center gap-1">
          {onClick && task.status === 'todo' && !isExecuting && (
            <span className="text-indigo-400 opacity-0 group-hover:opacity-100 transition-opacity">
              <Play size={14} />
            </span>
          )}
          {onDelete && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onDelete(task.id);
              }}
              aria-label="删除"
              className="text-slate-500 hover:text-rose-400 transition-colors p-1 -m-1"
            >
              <Trash2 size={14} />
            </button>
          )}
        </div>
      </div>
      {task.description && (
        <p
          data-testid="task-description"
          className="text-xs text-slate-400 mt-2 line-clamp-2"
        >
          {task.description}
        </p>
      )}
    </div>
  );
};
