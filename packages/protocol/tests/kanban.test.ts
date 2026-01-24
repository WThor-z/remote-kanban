import { describe, it, expect } from 'vitest';
import {
  parseKanbanEvent,
  isKanbanEventType,
  isKanbanTaskStatus,
  createEmptyBoardState,
  generateTaskId,
  type KanbanTask,
  type KanbanBoardState,
} from '../src/index';

describe('Kanban Type Guards', () => {
  describe('isKanbanEventType', () => {
    it('returns true for valid event types', () => {
      expect(isKanbanEventType('kanban:sync')).toBe(true);
      expect(isKanbanEventType('kanban:create')).toBe(true);
      expect(isKanbanEventType('kanban:move')).toBe(true);
      expect(isKanbanEventType('kanban:delete')).toBe(true);
    });

    it('returns false for invalid event types', () => {
      expect(isKanbanEventType('kanban:invalid')).toBe(false);
      expect(isKanbanEventType('sync')).toBe(false);
      expect(isKanbanEventType('')).toBe(false);
    });
  });

  describe('isKanbanTaskStatus', () => {
    it('returns true for valid statuses', () => {
      expect(isKanbanTaskStatus('todo')).toBe(true);
      expect(isKanbanTaskStatus('doing')).toBe(true);
      expect(isKanbanTaskStatus('done')).toBe(true);
    });

    it('returns false for invalid statuses', () => {
      expect(isKanbanTaskStatus('pending')).toBe(false);
      expect(isKanbanTaskStatus('completed')).toBe(false);
      expect(isKanbanTaskStatus('')).toBe(false);
    });
  });
});

describe('parseKanbanEvent', () => {
  describe('kanban:sync', () => {
    it('parses valid sync event', () => {
      const boardState: KanbanBoardState = createEmptyBoardState();
      const raw = JSON.stringify({ type: 'kanban:sync', payload: boardState });

      const event = parseKanbanEvent(raw);
      expect(event).not.toBeNull();
      expect(event?.type).toBe('kanban:sync');
      expect(event?.payload).toEqual(boardState);
    });

    it('returns null for sync with invalid payload', () => {
      const raw = JSON.stringify({ type: 'kanban:sync', payload: { invalid: true } });
      expect(parseKanbanEvent(raw)).toBeNull();
    });
  });

  describe('kanban:create', () => {
    it('parses valid create event', () => {
      const raw = JSON.stringify({
        type: 'kanban:create',
        payload: { title: 'New Task', description: 'A description' },
      });

      const event = parseKanbanEvent(raw);
      expect(event).not.toBeNull();
      expect(event?.type).toBe('kanban:create');
      if (event?.type === 'kanban:create') {
        expect(event.payload.title).toBe('New Task');
        expect(event.payload.description).toBe('A description');
      }
    });

    it('parses create event without description', () => {
      const raw = JSON.stringify({
        type: 'kanban:create',
        payload: { title: 'New Task' },
      });

      const event = parseKanbanEvent(raw);
      expect(event).not.toBeNull();
      expect(event?.type).toBe('kanban:create');
    });

    it('returns null for create with empty title', () => {
      const raw = JSON.stringify({
        type: 'kanban:create',
        payload: { title: '' },
      });
      expect(parseKanbanEvent(raw)).toBeNull();
    });

    it('returns null for create without title', () => {
      const raw = JSON.stringify({
        type: 'kanban:create',
        payload: { description: 'No title' },
      });
      expect(parseKanbanEvent(raw)).toBeNull();
    });
  });

  describe('kanban:move', () => {
    it('parses valid move event', () => {
      const raw = JSON.stringify({
        type: 'kanban:move',
        payload: { taskId: 'task-1', targetStatus: 'doing', targetIndex: 0 },
      });

      const event = parseKanbanEvent(raw);
      expect(event).not.toBeNull();
      expect(event?.type).toBe('kanban:move');
      if (event?.type === 'kanban:move') {
        expect(event.payload.taskId).toBe('task-1');
        expect(event.payload.targetStatus).toBe('doing');
        expect(event.payload.targetIndex).toBe(0);
      }
    });

    it('parses move event without targetIndex', () => {
      const raw = JSON.stringify({
        type: 'kanban:move',
        payload: { taskId: 'task-1', targetStatus: 'done' },
      });

      const event = parseKanbanEvent(raw);
      expect(event).not.toBeNull();
      expect(event?.type).toBe('kanban:move');
    });

    it('returns null for move with invalid targetStatus', () => {
      const raw = JSON.stringify({
        type: 'kanban:move',
        payload: { taskId: 'task-1', targetStatus: 'invalid' },
      });
      expect(parseKanbanEvent(raw)).toBeNull();
    });
  });

  describe('kanban:delete', () => {
    it('parses valid delete event', () => {
      const raw = JSON.stringify({
        type: 'kanban:delete',
        payload: { taskId: 'task-1' },
      });

      const event = parseKanbanEvent(raw);
      expect(event).not.toBeNull();
      expect(event?.type).toBe('kanban:delete');
      if (event?.type === 'kanban:delete') {
        expect(event.payload.taskId).toBe('task-1');
      }
    });

    it('returns null for delete without taskId', () => {
      const raw = JSON.stringify({
        type: 'kanban:delete',
        payload: {},
      });
      expect(parseKanbanEvent(raw)).toBeNull();
    });
  });

  describe('edge cases', () => {
    it('returns null for non-JSON string', () => {
      expect(parseKanbanEvent('not json')).toBeNull();
    });

    it('returns null for invalid JSON', () => {
      expect(parseKanbanEvent('{invalid}')).toBeNull();
    });

    it('returns null for unknown event type', () => {
      const raw = JSON.stringify({ type: 'unknown:event', payload: {} });
      expect(parseKanbanEvent(raw)).toBeNull();
    });

    it('returns null for missing type', () => {
      const raw = JSON.stringify({ payload: {} });
      expect(parseKanbanEvent(raw)).toBeNull();
    });
  });
});

describe('createEmptyBoardState', () => {
  it('creates a board with three columns', () => {
    const board = createEmptyBoardState();

    expect(board.columnOrder).toEqual(['todo', 'doing', 'done']);
    expect(Object.keys(board.columns)).toHaveLength(3);
    expect(Object.keys(board.tasks)).toHaveLength(0);
  });

  it('creates columns with empty taskIds arrays', () => {
    const board = createEmptyBoardState();

    expect(board.columns.todo.taskIds).toEqual([]);
    expect(board.columns.doing.taskIds).toEqual([]);
    expect(board.columns.done.taskIds).toEqual([]);
  });

  it('creates columns with proper titles', () => {
    const board = createEmptyBoardState();

    expect(board.columns.todo.title).toBe('To Do');
    expect(board.columns.doing.title).toBe('Doing');
    expect(board.columns.done.title).toBe('Done');
  });
});

describe('generateTaskId', () => {
  it('generates unique IDs', () => {
    const ids = new Set<string>();
    for (let i = 0; i < 100; i++) {
      ids.add(generateTaskId());
    }
    expect(ids.size).toBe(100);
  });

  it('generates IDs starting with task-', () => {
    const id = generateTaskId();
    expect(id.startsWith('task-')).toBe(true);
  });
});
