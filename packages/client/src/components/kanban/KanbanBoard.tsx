import { useState } from 'react';
import {
  DndContext,
  DragOverlay,
  closestCorners,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import type { KanbanBoardState, KanbanTaskStatus, KanbanTask } from '@opencode-vibe/protocol';
import { KanbanColumn } from './KanbanColumn';
import { TaskCard } from './TaskCard';

interface KanbanBoardProps {
  board: KanbanBoardState;
  onMoveTask: (taskId: string, targetStatus: KanbanTaskStatus, targetIndex?: number) => void;
  onDeleteTask: (taskId: string) => void;
  onTaskClick?: (task: KanbanTask) => void;
  executingTaskIds?: string[];
}

export const KanbanBoard = ({ board, onMoveTask, onDeleteTask, onTaskClick, executingTaskIds = [] }: KanbanBoardProps) => {
  const [activeId, setActiveId] = useState<string | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(event.active.id as string);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveId(null);

    if (!over) return;

    const activeId = active.id as string;
    const overId = over.id as string;

    // 判断目标是列还是任务
    const targetStatus = getTargetStatus(overId, board);
    if (!targetStatus) return;

    const activeTask = board.tasks[activeId];
    if (!activeTask) return;

    // 计算目标索引
    const targetColumn = board.columns[targetStatus];
    let targetIndex = targetColumn.taskIds.indexOf(overId);
    if (targetIndex === -1) {
      targetIndex = targetColumn.taskIds.length;
    }

    // 只在状态或位置变化时触发
    if (activeTask.status !== targetStatus || activeId !== overId) {
      onMoveTask(activeId, targetStatus, targetIndex);
    }
  };

  const activeTask = activeId ? board.tasks[activeId] : null;

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div className="grid grid-cols-3 gap-4 w-full">
        {board.columnOrder.map((columnId) => {
          const column = board.columns[columnId];
          const tasks = column.taskIds.map((taskId) => board.tasks[taskId]).filter(Boolean);

          return (
            <SortableContext
              key={columnId}
              items={column.taskIds}
              strategy={verticalListSortingStrategy}
            >
<KanbanColumn
                column={column}
                tasks={tasks}
                onDeleteTask={onDeleteTask}
                onTaskClick={onTaskClick}
                executingTaskIds={executingTaskIds}
              />
            </SortableContext>
          );
        })}
      </div>

      <DragOverlay>
        {activeTask && <TaskCard task={activeTask} isDragging />}
      </DragOverlay>
    </DndContext>
  );
};

// 根据 overId 确定目标列
const getTargetStatus = (overId: string, board: KanbanBoardState): KanbanTaskStatus | null => {
  // 检查是否是列 ID
  if (overId in board.columns) {
    return overId as KanbanTaskStatus;
  }

  // 检查是否是任务 ID，返回其所在列
  const task = board.tasks[overId];
  if (task) {
    return task.status;
  }

  return null;
};
