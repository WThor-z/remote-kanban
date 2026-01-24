import { describe, it, expect, beforeEach, vi } from 'vitest';
import { KanbanStore } from '../src/kanban/store';
import type { KanbanBoardState, KanbanTask } from '@opencode-vibe/protocol';

describe('KanbanStore', () => {
  let store: KanbanStore;

  beforeEach(() => {
    store = new KanbanStore();
  });

  describe('getState', () => {
    it('返回初始的空看板状态', () => {
      const state = store.getState();

      expect(state.columnOrder).toEqual(['todo', 'doing', 'done']);
      expect(Object.keys(state.tasks)).toHaveLength(0);
      expect(state.columns.todo.taskIds).toEqual([]);
      expect(state.columns.doing.taskIds).toEqual([]);
      expect(state.columns.done.taskIds).toEqual([]);
    });
  });

  describe('createTask', () => {
    it('创建新任务并添加到 todo 列', () => {
      const task = store.createTask('测试任务', '这是描述');

      expect(task.title).toBe('测试任务');
      expect(task.description).toBe('这是描述');
      expect(task.status).toBe('todo');
      expect(task.id).toMatch(/^task-/);

      const state = store.getState();
      expect(state.tasks[task.id]).toBeDefined();
      expect(state.columns.todo.taskIds).toContain(task.id);
    });

    it('创建无描述的任务', () => {
      const task = store.createTask('仅标题');

      expect(task.title).toBe('仅标题');
      expect(task.description).toBeUndefined();
    });

    it('抛出错误当标题为空', () => {
      expect(() => store.createTask('')).toThrow('任务标题不能为空');
    });
  });

  describe('moveTask', () => {
    it('将任务从 todo 移动到 doing', () => {
      const task = store.createTask('待移动');

      store.moveTask(task.id, 'doing');

      const state = store.getState();
      expect(state.tasks[task.id].status).toBe('doing');
      expect(state.columns.todo.taskIds).not.toContain(task.id);
      expect(state.columns.doing.taskIds).toContain(task.id);
    });

    it('将任务移动到指定索引位置', () => {
      const task1 = store.createTask('任务1');
      const task2 = store.createTask('任务2');
      const task3 = store.createTask('任务3');

      // 将 task3 移动到 todo 列的索引 0 位置
      store.moveTask(task3.id, 'todo', 0);

      const state = store.getState();
      expect(state.columns.todo.taskIds[0]).toBe(task3.id);
      expect(state.columns.todo.taskIds[1]).toBe(task1.id);
      expect(state.columns.todo.taskIds[2]).toBe(task2.id);
    });

    it('抛出错误当任务不存在', () => {
      expect(() => store.moveTask('不存在的ID', 'doing')).toThrow('任务不存在');
    });
  });

  describe('deleteTask', () => {
    it('删除任务', () => {
      const task = store.createTask('待删除');

      store.deleteTask(task.id);

      const state = store.getState();
      expect(state.tasks[task.id]).toBeUndefined();
      expect(state.columns.todo.taskIds).not.toContain(task.id);
    });

    it('抛出错误当任务不存在', () => {
      expect(() => store.deleteTask('不存在的ID')).toThrow('任务不存在');
    });
  });

  describe('loadState', () => {
    it('加载外部状态', () => {
      const externalState: KanbanBoardState = {
        tasks: {
          'task-1': {
            id: 'task-1',
            title: '外部任务',
            status: 'doing',
            createdAt: Date.now(),
          },
        },
        columns: {
          todo: { id: 'todo', title: 'To Do', taskIds: [] },
          doing: { id: 'doing', title: 'Doing', taskIds: ['task-1'] },
          done: { id: 'done', title: 'Done', taskIds: [] },
        },
        columnOrder: ['todo', 'doing', 'done'],
      };

      store.loadState(externalState);

      const state = store.getState();
      expect(state.tasks['task-1'].title).toBe('外部任务');
      expect(state.columns.doing.taskIds).toContain('task-1');
    });
  });

  describe('subscribe', () => {
    it('状态变更时触发回调', () => {
      const callback = vi.fn();
      store.subscribe(callback);

      store.createTask('触发回调');

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(store.getState());
    });

    it('取消订阅后不再触发回调', () => {
      const callback = vi.fn();
      const unsubscribe = store.subscribe(callback);

      unsubscribe();
      store.createTask('不应触发');

      expect(callback).not.toHaveBeenCalled();
    });
  });
});
