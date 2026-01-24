import { useDroppable } from '@dnd-kit/core';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { KanbanColumn as ColumnType, KanbanTask } from '@opencode-vibe/protocol';
import { TaskCard } from './TaskCard';

interface KanbanColumnProps {
  column: ColumnType;
  tasks: KanbanTask[];
  onDeleteTask: (taskId: string) => void;
}

export const KanbanColumn = ({ column, tasks, onDeleteTask }: KanbanColumnProps) => {
  const { setNodeRef, isOver } = useDroppable({
    id: column.id,
  });

  const columnColors = {
    todo: 'bg-slate-500/20 border-slate-500/30',
    doing: 'bg-amber-500/20 border-amber-500/30',
    done: 'bg-emerald-500/20 border-emerald-500/30',
  } as const;

  return (
    <div
      ref={setNodeRef}
      className={`
        flex flex-col rounded-xl border p-4 min-h-[400px]
        ${columnColors[column.id]}
        ${isOver ? 'ring-2 ring-indigo-500/50' : ''}
        transition-all duration-200
      `}
    >
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-slate-200">{column.title}</h3>
        <span
          data-testid="column-count"
          className="text-xs px-2 py-0.5 rounded-full bg-slate-700 text-slate-300"
        >
          {tasks.length}
        </span>
      </div>

      <div className="flex flex-col gap-2 flex-1">
        {tasks.length === 0 ? (
          <div className="text-xs text-slate-500 text-center py-8">暂无任务</div>
        ) : (
          tasks.map((task) => (
            <SortableTaskCard key={task.id} task={task} onDelete={onDeleteTask} />
          ))
        )}
      </div>
    </div>
  );
};

// 可拖拽的任务卡片包装器
interface SortableTaskCardProps {
  task: KanbanTask;
  onDelete: (taskId: string) => void;
}

const SortableTaskCard = ({ task, onDelete }: SortableTaskCardProps) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: task.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <TaskCard task={task} onDelete={onDelete} isDragging={isDragging} />
    </div>
  );
};
