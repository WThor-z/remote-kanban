import { describe, it, expect, beforeEach, afterEach, vi, Mock } from 'vitest';
import { AgentExecutor } from '../src/agent/executor';
import type {
  AgentSession,
  AgentConfig,
  AgentOutputEvent,
  AgentStatusEvent,
} from '@opencode-vibe/protocol';

// Define interface for mock
interface MockOpencodeClient {
  start: Mock;
  stop: Mock;
  waitForHealth: Mock;
  createSession: Mock;
  sendMessage: Mock;
  connectEventStream: Mock;
  on: Mock;
  emit: Mock;
  isRunning: boolean;
  serverUrl: string | null;
  _handlers: Record<string, ((...args: unknown[]) => void)[]>;
}

// Create mock OpencodeClient
const mockOpencodeClient: MockOpencodeClient = {
  start: vi.fn(),
  stop: vi.fn(),
  waitForHealth: vi.fn(),
  createSession: vi.fn(),
  sendMessage: vi.fn(),
  connectEventStream: vi.fn(),
  on: vi.fn(),
  emit: vi.fn(),
  isRunning: false,
  serverUrl: null,
  _handlers: {},
};

// Mock OpencodeClient module
vi.mock('../src/agent/opencode-client', () => {
  return {
    OpencodeClient: vi.fn().mockImplementation(() => ({
      ...mockOpencodeClient,
      on: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
        if (!mockOpencodeClient._handlers[event]) {
          mockOpencodeClient._handlers[event] = [];
        }
        mockOpencodeClient._handlers[event].push(handler);
      }),
    })),
  };
});

describe('AgentExecutor', () => {
  let executor: AgentExecutor;
  let mockSession: AgentSession;
  let mockConfig: AgentConfig;

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Reset mock functions
    mockOpencodeClient.start.mockReset();
    mockOpencodeClient.stop.mockReset();
    mockOpencodeClient.waitForHealth.mockReset();
    mockOpencodeClient.createSession.mockReset();
    mockOpencodeClient.sendMessage.mockReset();
    mockOpencodeClient.connectEventStream.mockReset();
    
    // Default mock implementations
    mockOpencodeClient.start.mockResolvedValue('http://127.0.0.1:12345');
    mockOpencodeClient.waitForHealth.mockResolvedValue(true);
    mockOpencodeClient.createSession.mockResolvedValue('opencode-session-1');
    mockOpencodeClient.sendMessage.mockResolvedValue(undefined);
    mockOpencodeClient.connectEventStream.mockResolvedValue(undefined);
    
    // Clear handlers
    mockOpencodeClient._handlers = {};
    
    executor = new AgentExecutor();

    mockSession = {
      id: 'test-session-1',
      agentType: 'opencode',
      status: 'idle',
      prompt: 'Test prompt',
    };

    mockConfig = {
      type: 'opencode',
      name: 'OpenCode',
      command: 'opencode',
      args: ['serve'],
      cwd: '/test/dir',
    };
  });

  afterEach(async () => {
    // Clean up any active sessions
    for (const session of executor.getAllSessions()) {
      try {
        await executor.stop(session.id);
      } catch {
        // Ignore errors during cleanup
      }
    }
  });

  describe('start', () => {
    it('should start a new agent session and emit starting status', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      // Start returns a promise, but we need to test immediate behavior
      const startPromise = executor.start(mockSession, mockConfig);

      // Session should be in starting status immediately
      const session = executor.getSession(mockSession.id);
      expect(session).toBeDefined();
      expect(session?.status).toBe('starting');
      expect(session?.startedAt).toBeDefined();
      
      // Status callback should have been called with 'starting'
      expect(statusCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          sessionId: mockSession.id,
          previousStatus: 'idle',
          currentStatus: 'starting',
        })
      );

      await startPromise;
    });

    it('should emit running status after successful start', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      await executor.start(mockSession, mockConfig);

      // Should have received 'starting' then 'running' status
      expect(statusCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          currentStatus: 'starting',
        })
      );
      expect(statusCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          previousStatus: 'starting',
          currentStatus: 'running',
        })
      );
    });

    it('should throw error when starting already active session', async () => {
      // Start first session
      const startPromise = executor.start(mockSession, mockConfig);
      
      // Try to start same session again (while first is still starting)
      await expect(executor.start(mockSession, mockConfig)).rejects.toThrow(
        'Session test-session-1 is already active'
      );

      await startPromise;
    });

    it('should handle startup failure gracefully', async () => {
      mockOpencodeClient.start.mockRejectedValue(new Error('Failed to start'));
      
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      await executor.start(mockSession, mockConfig);

      const session = executor.getSession(mockSession.id);
      expect(session?.status).toBe('failed');
      expect(session?.error).toBe('Failed to start');
    });

    it('should handle health check failure', async () => {
      mockOpencodeClient.waitForHealth.mockResolvedValue(false);
      
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      await executor.start(mockSession, mockConfig);

      const session = executor.getSession(mockSession.id);
      expect(session?.status).toBe('failed');
      expect(session?.error).toContain('health check');
    });
  });

  describe('stop', () => {
    it('should stop an active session', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      const startPromise = executor.start(mockSession, mockConfig);
      
      // Wait a tick for session to be registered
      await new Promise(r => setTimeout(r, 0));
      
      await executor.stop(mockSession.id);

      const session = executor.getSession(mockSession.id);
      expect(session?.status).toBe('aborted');
      expect(session?.endedAt).toBeDefined();
      expect(mockOpencodeClient.stop).toHaveBeenCalled();
    });

    it('should emit status event when session stops', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      const startPromise = executor.start(mockSession, mockConfig);
      await new Promise(r => setTimeout(r, 0));
      
      statusCallback.mockClear();

      await executor.stop(mockSession.id);

      expect(statusCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          sessionId: mockSession.id,
          currentStatus: 'aborted',
        })
      );
    });

    it('should throw error when stopping non-existent session', async () => {
      await expect(executor.stop('non-existent')).rejects.toThrow(
        'Session non-existent not found'
      );
    });

    it('should throw error when stopping inactive session', async () => {
      await executor.start(mockSession, mockConfig);
      await executor.stop(mockSession.id);

      await expect(executor.stop(mockSession.id)).rejects.toThrow(
        'Session test-session-1 is not active'
      );
    });
  });

  describe('write', () => {
    it('should warn when writing to session (not supported with HTTP API)', async () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      
      await executor.start(mockSession, mockConfig);
      executor.write(mockSession.id, 'test input');

      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('not supported')
      );
      
      consoleSpy.mockRestore();
    });

    it('should throw error when writing to non-existent session', () => {
      expect(() => executor.write('non-existent', 'data')).toThrow(
        'Session non-existent not found'
      );
    });

    it('should throw error when writing to inactive session', async () => {
      await executor.start(mockSession, mockConfig);
      await executor.stop(mockSession.id);

      expect(() => executor.write(mockSession.id, 'data')).toThrow(
        'Session test-session-1 is not active'
      );
    });
  });

  describe('onOutput', () => {
    it('should register output callback', async () => {
      const outputCallback = vi.fn();
      executor.onOutput(outputCallback);

      await executor.start(mockSession, mockConfig);

      // Callback should be registered (no output yet)
      expect(outputCallback).not.toHaveBeenCalled();
    });
  });

  describe('onStatus', () => {
    it('should register status callback and receive events', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      await executor.start(mockSession, mockConfig);

      // Should receive at least 'starting' status
      expect(statusCallback).toHaveBeenCalled();
    });

    it('should support multiple status callbacks', async () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();

      executor.onStatus(callback1);
      executor.onStatus(callback2);

      await executor.start(mockSession, mockConfig);

      expect(callback1).toHaveBeenCalled();
      expect(callback2).toHaveBeenCalled();
    });
  });

  describe('getSession', () => {
    it('should return undefined for non-existent session', () => {
      expect(executor.getSession('non-existent')).toBeUndefined();
    });

    it('should return session after start', async () => {
      const startPromise = executor.start(mockSession, mockConfig);
      
      // Wait a tick for session to be registered
      await new Promise(r => setTimeout(r, 0));

      const session = executor.getSession(mockSession.id);
      expect(session).toBeDefined();
      expect(session?.id).toBe(mockSession.id);

      await startPromise;
    });
  });

  describe('getAllSessions', () => {
    it('should return empty array initially', () => {
      expect(executor.getAllSessions()).toEqual([]);
    });

    it('should return all sessions', async () => {
      const session2: AgentSession = {
        id: 'test-session-2',
        agentType: 'claude-code',
        status: 'idle',
        prompt: 'Another prompt',
      };

      const p1 = executor.start(mockSession, mockConfig);
      await new Promise(r => setTimeout(r, 0));
      
      const p2 = executor.start(session2, { ...mockConfig, type: 'claude-code' });
      await new Promise(r => setTimeout(r, 0));

      const sessions = executor.getAllSessions();
      expect(sessions).toHaveLength(2);
      expect(sessions.map((s) => s.id)).toContain(mockSession.id);
      expect(sessions.map((s) => s.id)).toContain(session2.id);

      await Promise.all([p1, p2]);
    });
  });

  describe('multi-session management', () => {
    it('should handle multiple concurrent sessions', async () => {
      const session2: AgentSession = {
        id: 'test-session-2',
        agentType: 'claude-code',
        status: 'idle',
        prompt: 'Prompt 2',
      };

      const session3: AgentSession = {
        id: 'test-session-3',
        agentType: 'codex',
        status: 'idle',
        prompt: 'Prompt 3',
      };

      const promises = [
        executor.start(mockSession, mockConfig),
        executor.start(session2, { ...mockConfig, type: 'claude-code' }),
        executor.start(session3, { ...mockConfig, type: 'codex' }),
      ];

      // Wait for sessions to be registered
      await new Promise(r => setTimeout(r, 0));
      
      expect(executor.getAllSessions()).toHaveLength(3);

      await executor.stop(session2.id);

      const runningSessions = executor
        .getAllSessions()
        .filter((s) => s.status === 'running' || s.status === 'starting');
      expect(runningSessions.length).toBeLessThanOrEqual(2);

      await Promise.all(promises);
    });

    it('should isolate sessions from each other', async () => {
      const session2: AgentSession = {
        id: 'test-session-2',
        agentType: 'claude-code',
        status: 'idle',
        prompt: 'Different prompt',
      };

      const p1 = executor.start(mockSession, mockConfig);
      await new Promise(r => setTimeout(r, 0));
      
      const p2 = executor.start(session2, mockConfig);
      await new Promise(r => setTimeout(r, 0));

      await executor.stop(mockSession.id);

      expect(executor.getSession(mockSession.id)?.status).toBe('aborted');
      expect(['starting', 'running']).toContain(executor.getSession(session2.id)?.status);

      await Promise.all([p1, p2]);
    });
  });

  describe('status transitions', () => {
    it('should transition from idle to starting on start', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      const startPromise = executor.start(mockSession, mockConfig);

      expect(statusCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          previousStatus: 'idle',
          currentStatus: 'starting',
        })
      );

      await startPromise;
    });

    it('should transition to aborted on stop', async () => {
      const statusCallback = vi.fn();
      executor.onStatus(statusCallback);

      const startPromise = executor.start(mockSession, mockConfig);
      await new Promise(r => setTimeout(r, 0));
      
      statusCallback.mockClear();

      await executor.stop(mockSession.id);

      expect(statusCallback).toHaveBeenCalledWith(
        expect.objectContaining({
          currentStatus: 'aborted',
        })
      );

      await startPromise;
    });
  });

  describe('environment variables', () => {
    it('should pass environment variables to OpencodeClient', async () => {
      const { OpencodeClient } = await import('../src/agent/opencode-client');
      
      const configWithEnv: AgentConfig = {
        ...mockConfig,
        env: {
          CUSTOM_VAR: 'custom_value',
          API_KEY: 'secret',
        },
      };

      await executor.start(mockSession, configWithEnv);

      expect(OpencodeClient).toHaveBeenCalledWith(
        expect.objectContaining({
          cwd: '/test/dir',
          env: expect.objectContaining({
            CUSTOM_VAR: 'custom_value',
            API_KEY: 'secret',
          }),
        })
      );
    });
  });
});
