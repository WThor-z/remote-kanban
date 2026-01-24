import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { KanbanManager } from '../src/kanban/manager';
import type { KanbanBoardState } from '@opencode-vibe/protocol';
import * as fs from 'fs';
import * as path from 'path';

// Mock fs 模块
vi.mock('fs', () => ({
  existsSync: vi.fn(),
  readFileSync: vi.fn(),
  writeFileSync: vi.fn(),
  mkdirSync: vi.fn(),
  watch: vi.fn(() => ({ close: vi.fn() })),
}));

describe('KanbanManager', () => {
  const testDir = '/test/project';
  const kanbanPath = path.join(testDir, '.opencode', 'kanban.json');

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('初始化', () => {
    it('当文件不存在时创建默认看板', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false);

      const manager = new KanbanManager(testDir);
      const state = manager.getState();

      expect(fs.mkdirSync).toHaveBeenCalledWith(
        path.join(testDir, '.opencode'),
        { recursive: true }
      );
      expect(fs.writeFileSync).toHaveBeenCalled();
      expect(state.columnOrder).toEqual(['todo', 'doing', 'done']);
      expect(Object.keys(state.tasks)).toHaveLength(0);
    });

    it('当文件存在时加载已有看板', () => {
      const existingState: KanbanBoardState = {
        tasks: {
          'task-1': {
            id: 'task-1',
            title: '已存在的任务',
            status: 'doing',
            createdAt: 1234567890,
          },
        },
        columns: {
          todo: { id: 'todo', title: 'To Do', taskIds: [] },
          doing: { id: 'doing', title: 'Doing', taskIds: ['task-1'] },
          done: { id: 'done', title: 'Done', taskIds: [] },
        },
        columnOrder: ['todo', 'doing', 'done'],
      };

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(existingState));

      const manager = new KanbanManager(testDir);
      const state = manager.getState();

      expect(fs.readFileSync).toHaveBeenCalledWith(kanbanPath, 'utf-8');
      expect(state.tasks['task-1'].title).toBe('已存在的任务');
    });

    it('当文件损坏时创建默认看板', () => {
      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readFileSync).mockReturnValue('invalid json {{{');

      const manager = new KanbanManager(testDir);
      const state = manager.getState();

      expect(state.columnOrder).toEqual(['todo', 'doing', 'done']);
      expect(Object.keys(state.tasks)).toHaveLength(0);
    });
  });

  describe('持久化', () => {
    it('状态变更时自动保存到文件', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false);

      const manager = new KanbanManager(testDir);
      vi.clearAllMocks(); // 清除初始化的调用

      manager.createTask('新任务');

      expect(fs.writeFileSync).toHaveBeenCalledTimes(1);
      const [filePath, content] = vi.mocked(fs.writeFileSync).mock.calls[0];
      expect(filePath).toBe(kanbanPath);
      const saved = JSON.parse(content as string) as KanbanBoardState;
      expect(Object.values(saved.tasks)[0].title).toBe('新任务');
    });
  });

  describe('订阅', () => {
    it('支持外部订阅状态变更', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false);

      const manager = new KanbanManager(testDir);
      const callback = vi.fn();

      manager.subscribe(callback);
      manager.createTask('触发订阅');

      expect(callback).toHaveBeenCalledTimes(1);
    });
  });

  describe('文件监听', () => {
    it('启动时注册文件监听器', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false);

      new KanbanManager(testDir);

      expect(fs.watch).toHaveBeenCalledWith(kanbanPath, expect.any(Function));
    });

    it('销毁时关闭文件监听器', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false);
      const mockClose = vi.fn();
      vi.mocked(fs.watch).mockReturnValue({ close: mockClose } as unknown as fs.FSWatcher);

      const manager = new KanbanManager(testDir);
      manager.dispose();

      expect(mockClose).toHaveBeenCalled();
    });
  });
});
