import type { KanbanBoardState } from '@opencode-vibe/protocol';

export const filterBoardByVisibleTaskIds = (
  board: KanbanBoardState,
  visibleTaskIds: Set<string> | null,
): KanbanBoardState => {
  if (!visibleTaskIds) {
    return board;
  }

  const tasks = Object.fromEntries(
    Object.entries(board.tasks).filter(([taskId]) => visibleTaskIds.has(taskId)),
  );

  return {
    ...board,
    tasks,
    columns: {
      todo: {
        ...board.columns.todo,
        taskIds: board.columns.todo.taskIds.filter((taskId) => visibleTaskIds.has(taskId)),
      },
      doing: {
        ...board.columns.doing,
        taskIds: board.columns.doing.taskIds.filter((taskId) => visibleTaskIds.has(taskId)),
      },
      done: {
        ...board.columns.done,
        taskIds: board.columns.done.taskIds.filter((taskId) => visibleTaskIds.has(taskId)),
      },
    },
  };
};
