import { useState, useEffect, useCallback } from 'react';
import type { Socket } from 'socket.io-client';
import type {
  TaskSessionHistory,
  ChatMessage,
  AgentSessionStatus,
} from '@opencode-vibe/protocol';

interface UseTaskSessionOptions {
  socket: Socket | undefined;
  isConnected: boolean;
}

interface UseTaskSessionReturn {
  /** 当前选中的任务 ID */
  selectedTaskId: string | null;
  /** 任务会话历史 */
  history: TaskSessionHistory | null;
  /** 任务执行状态 */
  status: AgentSessionStatus | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
  /** 选中任务 */
  selectTask: (taskId: string | null) => void;
  /** 执行任务 */
  executeTask: (taskId: string) => void;
  /** 停止任务 */
  stopTask: (taskId: string) => void;
  /** 发送消息 */
  sendMessage: (taskId: string, content: string) => void;
}

export function useTaskSession({ socket, isConnected }: UseTaskSessionOptions): UseTaskSessionReturn {
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [history, setHistory] = useState<TaskSessionHistory | null>(null);
  const [status, setStatus] = useState<AgentSessionStatus | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 监听任务事件
  useEffect(() => {
    if (!socket || !isConnected) return;

    const handleTaskStatus = (payload: { taskId: string; status: AgentSessionStatus }) => {
      if (payload.taskId === selectedTaskId) {
        setStatus(payload.status);
        setIsLoading(payload.status === 'starting' || payload.status === 'running');
      }
      // 更新历史中的状态
      setHistory(prev => {
        if (prev && prev.taskId === payload.taskId) {
          return { ...prev, status: payload.status };
        }
        return prev;
      });
    };

    const handleTaskMessage = (payload: { taskId: string; message: ChatMessage }) => {
      if (payload.taskId === selectedTaskId) {
        setHistory(prev => {
          if (!prev) return prev;
          // 如果是同一个 assistant 消息，更新它
          const lastMessage = prev.messages[prev.messages.length - 1];
          if (lastMessage?.id === payload.message.id) {
            const updatedMessages = [...prev.messages];
            updatedMessages[updatedMessages.length - 1] = payload.message;
            return { ...prev, messages: updatedMessages };
          }
          // 否则追加新消息
          return {
            ...prev,
            messages: [...prev.messages, payload.message],
          };
        });
      }
    };

    const handleTaskHistory = (payload: TaskSessionHistory) => {
      if (payload.taskId === selectedTaskId) {
        setHistory(payload);
        setStatus(payload.status);
        setIsLoading(false);
      }
    };

    const handleTaskError = (payload: { taskId: string; error: string }) => {
      if (payload.taskId === selectedTaskId) {
        setError(payload.error);
        setIsLoading(false);
      }
    };

    socket.on('task:status', handleTaskStatus);
    socket.on('task:message', handleTaskMessage);
    socket.on('task:history', handleTaskHistory);
    socket.on('task:error', handleTaskError);

    return () => {
      socket.off('task:status', handleTaskStatus);
      socket.off('task:message', handleTaskMessage);
      socket.off('task:history', handleTaskHistory);
      socket.off('task:error', handleTaskError);
    };
  }, [socket, isConnected, selectedTaskId]);

  // 选中任务时请求历史
  useEffect(() => {
    if (!socket || !isConnected || !selectedTaskId) {
      setHistory(null);
      setStatus(null);
      return;
    }

    setIsLoading(true);
    setError(null);
    socket.emit('task:history', { taskId: selectedTaskId });
  }, [socket, isConnected, selectedTaskId]);

  const selectTask = useCallback((taskId: string | null) => {
    setSelectedTaskId(taskId);
    if (!taskId) {
      setHistory(null);
      setStatus(null);
      setError(null);
    }
  }, []);

  const executeTask = useCallback((taskId: string) => {
    if (!socket || !isConnected) return;
    setIsLoading(true);
    setError(null);
    socket.emit('task:execute', { taskId });
  }, [socket, isConnected]);

  const stopTask = useCallback((taskId: string) => {
    if (!socket || !isConnected) return;
    socket.emit('task:stop', { taskId });
  }, [socket, isConnected]);

  const sendMessage = useCallback((taskId: string, content: string) => {
    if (!socket || !isConnected) return;
    socket.emit('task:message', { taskId, content });
  }, [socket, isConnected]);

  return {
    selectedTaskId,
    history,
    status,
    isLoading,
    error,
    selectTask,
    executeTask,
    stopTask,
    sendMessage,
  };
}
