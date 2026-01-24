import { useEffect, useState, useCallback } from 'react';
import { io, Socket } from 'socket.io-client';
import type { KanbanBoardState, KanbanTaskStatus } from '@opencode-vibe/protocol';

let kanbanSocket: Socket | undefined;
let kanbanSocketUrl: string | undefined;

const resolveSocketUrl = () => {
  const envUrl = typeof import.meta !== 'undefined'
    ? import.meta.env?.VITE_OPENCODE_SOCKET_URL
    : undefined;

  if (envUrl) {
    return envUrl;
  }

  if (typeof process !== 'undefined' && process.env?.OPENCODE_SOCKET_URL) {
    return process.env.OPENCODE_SOCKET_URL;
  }

  return 'http://localhost:3000';
};

const defaultBoardState: KanbanBoardState = {
  tasks: {},
  columns: {
    todo: { id: 'todo', title: 'To Do', taskIds: [] },
    doing: { id: 'doing', title: 'Doing', taskIds: [] },
    done: { id: 'done', title: 'Done', taskIds: [] },
  },
  columnOrder: ['todo', 'doing', 'done'],
};

export const useKanban = () => {
  const url = resolveSocketUrl();
  const [board, setBoard] = useState<KanbanBoardState>(defaultBoardState);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    if (!kanbanSocket || kanbanSocketUrl !== url) {
      kanbanSocket?.disconnect();
      kanbanSocket = io(url);
      kanbanSocketUrl = url;
    }

    const handleSync = (state: KanbanBoardState) => {
      setBoard(state);
      setIsLoading(false);
    };

    const handleError = (error: { message: string }) => {
      console.error('Kanban error:', error.message);
    };

    kanbanSocket.on('kanban:sync', handleSync);
    kanbanSocket.on('kanban:error', handleError);

    // 请求初始同步
    kanbanSocket.emit('kanban:request-sync');

    return () => {
      kanbanSocket?.off('kanban:sync', handleSync);
      kanbanSocket?.off('kanban:error', handleError);
    };
  }, [url]);

  const createTask = useCallback((title: string, description?: string) => {
    kanbanSocket?.emit('kanban:create', { title, description });
  }, []);

  const moveTask = useCallback((taskId: string, targetStatus: KanbanTaskStatus, targetIndex?: number) => {
    kanbanSocket?.emit('kanban:move', { taskId, targetStatus, targetIndex });
  }, []);

  const deleteTask = useCallback((taskId: string) => {
    kanbanSocket?.emit('kanban:delete', { taskId });
  }, []);

  return {
    board,
    isLoading,
    createTask,
    moveTask,
    deleteTask,
  };
};
