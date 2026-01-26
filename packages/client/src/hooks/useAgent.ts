import { useEffect, useState, useCallback, useRef } from 'react';
import { io, Socket } from 'socket.io-client';
import type {
  AgentSession,
  AgentType,
  AgentOutputEvent,
  AgentStatusEvent,
  AgentTaskDetectedEvent,
} from '@opencode-vibe/protocol';

// Singleton socket (shared with useOpencode)
let socket: Socket | undefined;
let socketUrl: string | undefined;

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

const getSocket = (): Socket => {
  const url = resolveSocketUrl();
  if (!socket || socketUrl !== url) {
    socket?.disconnect();
    // Use WebSocket transport only to avoid CORS issues with polling
    socket = io(url, {
      transports: ['websocket'],
    });
    socketUrl = url;
  }
  return socket;
};

export interface AgentOutput {
  type: 'stdout' | 'stderr' | 'system';
  data: string;
  timestamp: number;
}

export interface UseAgentReturn {
  /** Current agent sessions */
  sessions: AgentSession[];
  /** Current active session (most recent running) */
  activeSession: AgentSession | undefined;
  /** Agent output log */
  outputLog: AgentOutput[];
  /** Start a new agent session */
  startAgent: (agentType: AgentType, prompt: string, taskId?: string) => void;
  /** Stop an agent session */
  stopAgent: (sessionId: string) => void;
  /** Send input to an agent */
  writeToAgent: (sessionId: string, data: string) => void;
  /** Request session list refresh */
  refreshSessions: () => void;
  /** Clear output log */
  clearOutput: () => void;
  /** Is any agent currently running */
  isRunning: boolean;
}

export const useAgent = (): UseAgentReturn => {
  const [sessions, setSessions] = useState<AgentSession[]>([]);
  const [outputLog, setOutputLog] = useState<AgentOutput[]>([]);
  const outputLogRef = useRef(outputLog);
  
  // Keep ref in sync
  useEffect(() => {
    outputLogRef.current = outputLog;
  }, [outputLog]);

  useEffect(() => {
    const s = getSocket();

    // Handle session updates
    const handleSession = (session: AgentSession) => {
      setSessions(prev => {
        const idx = prev.findIndex(s => s.id === session.id);
        if (idx >= 0) {
          const updated = [...prev];
          updated[idx] = session;
          return updated;
        }
        return [...prev, session];
      });
    };

    // Handle sessions list
    const handleSessions = (sessionsList: AgentSession[]) => {
      setSessions(sessionsList);
    };

    // Handle output events
    const handleOutput = (event: AgentOutputEvent) => {
      const newOutput: AgentOutput = {
        type: event.type,
        data: event.data,
        timestamp: event.timestamp,
      };
      setOutputLog(prev => [...prev, newOutput]);
    };

    // Handle status events
    const handleStatus = (event: AgentStatusEvent) => {
      setSessions(prev => prev.map(session => {
        if (session.id === event.sessionId) {
          return {
            ...session,
            status: event.currentStatus,
            error: event.error,
          };
        }
        return session;
      }));
    };

    // Handle task detection (for integration with Kanban)
    const handleTaskDetected = (event: AgentTaskDetectedEvent) => {
      // This could be used to auto-create/update Kanban tasks
      console.log('[Agent] Task detected:', event);
    };

    // Handle errors
    const handleError = (error: { sessionId?: string; message: string }) => {
      const systemOutput: AgentOutput = {
        type: 'system',
        data: `[Error] ${error.message}`,
        timestamp: Date.now(),
      };
      setOutputLog(prev => [...prev, systemOutput]);
    };

    s.on('agent:session', handleSession);
    s.on('agent:sessions', handleSessions);
    s.on('agent:output', handleOutput);
    s.on('agent:status', handleStatus);
    s.on('agent:task-detected', handleTaskDetected);
    s.on('agent:error', handleError);

    // Request initial session list
    s.emit('agent:list');

    return () => {
      s.off('agent:session', handleSession);
      s.off('agent:sessions', handleSessions);
      s.off('agent:output', handleOutput);
      s.off('agent:status', handleStatus);
      s.off('agent:task-detected', handleTaskDetected);
      s.off('agent:error', handleError);
    };
  }, []);

  const startAgent = useCallback((agentType: AgentType, prompt: string, taskId?: string) => {
    const s = getSocket();
    s.emit('agent:start', { agentType, prompt, taskId });
  }, []);

  const stopAgent = useCallback((sessionId: string) => {
    const s = getSocket();
    s.emit('agent:stop', { sessionId });
  }, []);

  const writeToAgent = useCallback((sessionId: string, data: string) => {
    const s = getSocket();
    s.emit('agent:input', { sessionId, data });
  }, []);

  const refreshSessions = useCallback(() => {
    const s = getSocket();
    s.emit('agent:list');
  }, []);

  const clearOutput = useCallback(() => {
    setOutputLog([]);
  }, []);

  // Derived state
  const activeSession = sessions.find(s => 
    s.status === 'running' || s.status === 'starting'
  );
  
  const isRunning = sessions.some(s => 
    s.status === 'running' || s.status === 'starting'
  );

  return {
    sessions,
    activeSession,
    outputLog,
    startAgent,
    stopAgent,
    writeToAgent,
    refreshSessions,
    clearOutput,
    isRunning,
  };
};

export default useAgent;
