import { useEffect, useRef, useState } from 'react';
import type { KanbanTask } from '@opencode-vibe/protocol';
import { Trash2, Play, Loader2 } from 'lucide-react';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';

interface TaskCardProps {
  task: KanbanTask;
  onDelete?: (taskId: string) => void;
  onClick?: (task: KanbanTask) => void;
  isDragging?: boolean;
  isExecuting?: boolean;
  language?: ConsoleLanguage;
}

const statusColors = {
  todo: 'border-l-slate-400',
  doing: 'border-l-amber-400',
  done: 'border-l-emerald-400',
} as const;

const statusLabels = {
  en: {
    todo: 'todo',
    doing: 'doing',
    done: 'done',
    delete: 'Delete task',
  },
  zh: {
    todo: '待办',
    doing: '进行中',
    done: '已完成',
    delete: '删除任务',
  },
} as const;

export const TaskCard = ({
  task,
  onDelete,
  onClick,
  isDragging = false,
  isExecuting = false,
  language = 'en',
}: TaskCardProps) => {
  const [flowClass, setFlowClass] = useState('');
  const previousStatusRef = useRef(task.status);

  useEffect(() => {
    const previousStatus = previousStatusRef.current;
    if (previousStatus !== task.status) {
      if (task.status === 'doing') {
        setFlowClass('task-card--flow-doing');
      } else if (task.status === 'done') {
        setFlowClass('task-card--flow-done');
      } else {
        setFlowClass('');
      }
    }

    previousStatusRef.current = task.status;
  }, [task.status]);

  useEffect(() => {
    if (!flowClass) {
      return;
    }

    const timer = window.setTimeout(() => {
      setFlowClass('');
    }, 760);

    return () => window.clearTimeout(timer);
  }, [flowClass]);

  const handleClick = (e: React.MouseEvent) => {
    // 阻止删除按钮触发卡片点击
    if ((e.target as HTMLElement).closest('button')) return;
    onClick?.(task);
  };

  const createdAtLabel = new Date(task.createdAt).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });

  return (
    <div
      data-testid="task-card"
      onClick={handleClick}
      className={`
        task-card bg-slate-800 rounded-lg p-3 border-l-4 shadow-md cursor-pointer
        ${statusColors[task.status]}
        ${isDragging ? 'task-card--dragging opacity-50 shadow-lg ring-2 ring-indigo-500' : ''}
        ${flowClass}
        ${onClick ? 'hover:bg-slate-750 hover:ring-1 hover:ring-indigo-500/50' : ''}
        transition-all duration-150
      `}
    >
      <div className="task-card__head">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            {isExecuting && (
              <Loader2 size={14} className="animate-spin text-indigo-400 flex-shrink-0" />
            )}
            <h4 className="task-card__title truncate">
              {task.title}
            </h4>
          </div>

          {task.description && (
            <p
              data-testid="task-description"
              className="task-card__description line-clamp-2"
            >
              {task.description}
            </p>
          )}
        </div>

        <div className="flex items-center gap-1">
          {onClick && task.status === 'todo' && !isExecuting && (
            <span className="text-cyan-300">
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
              aria-label={statusLabels[language].delete}
              className="task-card__icon-btn text-slate-500 hover:text-rose-400 transition-colors p-1 -m-1"
            >
              <Trash2 size={14} />
            </button>
          )}
        </div>
      </div>

      <div className="task-card__meta">
        <span data-testid="task-status" className={`task-card__status task-card__status--${task.status}`}>
          {statusLabels[language][task.status]}
        </span>
        <span className="task-card__time">{createdAtLabel}</span>
      </div>
    </div>
  );
};
