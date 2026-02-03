import { useState, useEffect } from 'react';
import { useOpencode } from './useOpencode';
import type { ExecutionEvent } from '@opencode-vibe/protocol';

/**
 * Gateway event format (simpler than full ExecutionEvent)
 * Sent by Rust executor.rs for Gateway tasks
 */
interface GatewayExecutionEvent {
  task_id: string;
  event_type: string;
  content?: string;
  timestamp: number;
}

/**
 * Normalize event to common format for display
 */
function normalizeEvent(event: ExecutionEvent | GatewayExecutionEvent): ExecutionEvent {
  // Check if it's a Gateway event (has simpler structure)
  if ('event_type' in event && typeof event.event_type === 'string' && !('event' in event)) {
    const gatewayEvent = event as GatewayExecutionEvent;
    return {
      id: `gateway-${gatewayEvent.timestamp}`,
      session_id: '',
      task_id: gatewayEvent.task_id,
      timestamp: new Date(gatewayEvent.timestamp).toISOString(),
      event_type: 'agent_event',
      event: {
        type: 'raw_output',
        stream: 'stdout',
        content: gatewayEvent.content || '',
      },
    } as ExecutionEvent;
  }
  return event as ExecutionEvent;
}

export const useExecutionEvents = (taskId: string) => {
  const { socket } = useOpencode();
  const [events, setEvents] = useState<ExecutionEvent[]>([]);

  useEffect(() => {
    if (!socket || !taskId) return;

    const handleEvent = (event: ExecutionEvent | GatewayExecutionEvent) => {
      // Ensure we only process events for this task
      // Note: Rust sends snake_case task_id in ExecutionEventBase
      if (event.task_id === taskId) {
        setEvents(prev => [...prev, normalizeEvent(event)]);
      }
    };

    socket.on('task:execution_event', handleEvent);

    return () => {
      socket.off('task:execution_event', handleEvent);
    };
  }, [socket, taskId]);

  return { events, clearEvents: () => setEvents([]) };
};
