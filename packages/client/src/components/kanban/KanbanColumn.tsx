import { useDroppable } from '@dnd-kit/core';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { KanbanColumn as ColumnType, KanbanTask } from '@opencode-vibe/protocol';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';
import { TaskCard } from './TaskCard';

interface KanbanColumnProps {
  column: ColumnType;
  tasks: KanbanTask[];
  onDeleteTask: (taskId: string) => void;
  onTaskClick?: (task: KanbanTask) => void;
  executingTaskIds?: string[];
  language?: ConsoleLanguage;
}

const emptyColumnMessage: Record<ConsoleLanguage, string> = {
  en: 'No tasks',
  zh: '暂无任务',
};

export const KanbanColumn = ({
  column,
  tasks,
  onDeleteTask,
  onTaskClick,
  executingTaskIds = [],
  language = 'en',
}: KanbanColumnProps) => {
  const { setNodeRef, isOver } = useDroppable({
    id: column.id,
  });

  const columnColors = {
    todo: 'kanban-column--todo',
    doing: 'kanban-column--doing',
    done: 'kanban-column--done',
  } as const;

  return (
    <div
      ref={setNodeRef}
      className={`kanban-column ${columnColors[column.id]} ${isOver ? 'kanban-column--active' : ''}`}
    >
      <div className="kanban-column__header">
        <h3 className="kanban-column__title">{column.title}</h3>
        <span
          data-testid="column-count"
          className="column-count-badge"
        >
          {tasks.length}
        </span>
      </div>

      <div className="flex flex-col gap-2 flex-1">
        {tasks.length === 0 ? (
          <div className="kanban-empty">{emptyColumnMessage[language]}</div>
        ) : (
          tasks.map((task) => (
            <SortableTaskCard 
              key={task.id} 
              task={task} 
              onDelete={onDeleteTask}
              onClick={onTaskClick}
              isExecuting={executingTaskIds.includes(task.id)}
              language={language}
            />
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
  onClick?: (task: KanbanTask) => void;
  isExecuting?: boolean;
  language: ConsoleLanguage;
}

const SortableTaskCard = ({ task, onDelete, onClick, isExecuting, language }: SortableTaskCardProps) => {
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
      <TaskCard 
        task={task} 
        onDelete={onDelete} 
        onClick={onClick}
        isDragging={isDragging}
        isExecuting={isExecuting}
        language={language}
      />
    </div>
  );
};
