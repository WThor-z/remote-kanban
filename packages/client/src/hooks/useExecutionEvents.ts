import { useState, useEffect } from 'react';
import { useOpencode } from './useOpencode';
import type { ExecutionEvent } from '@opencode-vibe/protocol';

export const useExecutionEvents = (taskId: string) => {
  const { socket } = useOpencode();
  const [events, setEvents] = useState<ExecutionEvent[]>([]);

  useEffect(() => {
    if (!socket || !taskId) return;

    const handleEvent = (event: ExecutionEvent) => {
      // Ensure we only process events for this task
      // Note: Rust sends snake_case task_id in ExecutionEventBase
      if (event.task_id === taskId) {
        setEvents(prev => [...prev, event]);
      }
    };

    socket.on('task:execution_event', handleEvent);

    return () => {
      socket.off('task:execution_event', handleEvent);
    };
  }, [socket, taskId]);

  return { events, clearEvents: () => setEvents([]) };
};
