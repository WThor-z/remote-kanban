import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useKanban } from '../useKanban';
import type { KanbanBoardState } from '@opencode-vibe/protocol';

// Mock socket.io-client
const mockSocket = {
  on: vi.fn(),
  off: vi.fn(),
  emit: vi.fn(),
  connected: true,
};

vi.mock('socket.io-client', () => ({
  io: vi.fn(() => mockSocket),
}));

describe('useKanban', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockBoardState: KanbanBoardState = {
    tasks: {
      'task-1': {
        id: 'task-1',
        title: '测试任务',
        status: 'todo',
        createdAt: Date.now(),
      },
    },
    columns: {
      todo: { id: 'todo', title: 'To Do', taskIds: ['task-1'] },
      doing: { id: 'doing', title: 'Doing', taskIds: [] },
      done: { id: 'done', title: 'Done', taskIds: [] },
    },
    columnOrder: ['todo', 'doing', 'done'],
  };

  it('初始化时请求同步', () => {
    renderHook(() => useKanban());

    expect(mockSocket.emit).toHaveBeenCalledWith('kanban:request-sync');
  });

  it('监听 kanban:sync 事件并更新状态', async () => {
    // 捕获 kanban:sync 回调
    let syncCallback: ((state: KanbanBoardState) => void) | undefined;
    mockSocket.on.mockImplementation((event: string, cb: (state: KanbanBoardState) => void) => {
      if (event === 'kanban:sync') {
        syncCallback = cb;
      }
    });

    const { result } = renderHook(() => useKanban());

    // 模拟服务器发送同步事件
    act(() => {
      syncCallback?.(mockBoardState);
    });

    await waitFor(() => {
      expect(result.current.board).toEqual(mockBoardState);
    });
  });

  it('createTask 发送 kanban:create 事件', () => {
    const { result } = renderHook(() => useKanban());

    act(() => {
      result.current.createTask('新任务', '描述');
    });

    expect(mockSocket.emit).toHaveBeenCalledWith('kanban:create', {
      title: '新任务',
      description: '描述',
    });
  });

  it('moveTask 发送 kanban:move 事件', () => {
    const { result } = renderHook(() => useKanban());

    act(() => {
      result.current.moveTask('task-1', 'doing', 0);
    });

    expect(mockSocket.emit).toHaveBeenCalledWith('kanban:move', {
      taskId: 'task-1',
      targetStatus: 'doing',
      targetIndex: 0,
    });
  });

  it('deleteTask 发送 kanban:delete 事件', () => {
    const { result } = renderHook(() => useKanban());

    act(() => {
      result.current.deleteTask('task-1');
    });

    expect(mockSocket.emit).toHaveBeenCalledWith('kanban:delete', {
      taskId: 'task-1',
    });
  });
});
