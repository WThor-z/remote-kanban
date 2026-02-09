import { describe, expect, it } from 'vitest';
import type { KanbanBoardState } from '@opencode-vibe/protocol';

import { filterBoardByVisibleTaskIds } from '../kanbanBoardFilter';

const boardFixture: KanbanBoardState = {
  tasks: {
    'task-1': {
      id: 'task-1',
      title: 'Task One',
      status: 'todo',
      createdAt: 1,
    },
    'task-2': {
      id: 'task-2',
      title: 'Task Two',
      status: 'doing',
      createdAt: 2,
    },
    'task-3': {
      id: 'task-3',
      title: 'Task Three',
      status: 'done',
      createdAt: 3,
    },
  },
  columns: {
    todo: { id: 'todo', title: 'To Do', taskIds: ['task-1'] },
    doing: { id: 'doing', title: 'Doing', taskIds: ['task-2'] },
    done: { id: 'done', title: 'Done', taskIds: ['task-3'] },
  },
  columnOrder: ['todo', 'doing', 'done'],
};

describe('filterBoardByVisibleTaskIds', () => {
  it('returns original board when visible set is null', () => {
    const result = filterBoardByVisibleTaskIds(boardFixture, null);
    expect(result).toBe(boardFixture);
  });

  it('keeps only tasks present in visible set', () => {
    const result = filterBoardByVisibleTaskIds(boardFixture, new Set(['task-2']));

    expect(Object.keys(result.tasks)).toEqual(['task-2']);
    expect(result.columns.todo.taskIds).toEqual([]);
    expect(result.columns.doing.taskIds).toEqual(['task-2']);
    expect(result.columns.done.taskIds).toEqual([]);
  });
});
