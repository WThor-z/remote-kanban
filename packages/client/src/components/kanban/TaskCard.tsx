import type { KanbanTask } from '@opencode-vibe/protocol';
import { Trash2 } from 'lucide-react';

interface TaskCardProps {
  task: KanbanTask;
  onDelete?: (taskId: string) => void;
  isDragging?: boolean;
}

const statusColors = {
  todo: 'border-l-slate-400',
  doing: 'border-l-amber-400',
  done: 'border-l-emerald-400',
} as const;

export const TaskCard = ({ task, onDelete, isDragging = false }: TaskCardProps) => {
  return (
    <div
      data-testid="task-card"
      className={`
        bg-slate-800 rounded-lg p-3 border-l-4 shadow-md
        ${statusColors[task.status]}
        ${isDragging ? 'opacity-50 shadow-lg ring-2 ring-indigo-500' : ''}
        transition-all duration-150
      `}
    >
      <div className="flex justify-between items-start gap-2">
        <h4 className="text-sm font-medium text-slate-100 flex-1">
          {task.title}
        </h4>
        {onDelete && (
          <button
            type="button"
            onClick={() => onDelete(task.id)}
            aria-label="删除"
            className="text-slate-500 hover:text-rose-400 transition-colors p-1 -m-1"
          >
            <Trash2 size={14} />
          </button>
        )}
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
