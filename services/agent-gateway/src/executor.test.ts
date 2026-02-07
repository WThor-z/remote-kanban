import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { GatewayAgentEvent } from './types.js';
import { EventEmitter } from 'events';
import path from 'path';

// Create mock functions that will be shared - use vi.hoisted for proper hoisting
const { mockSession, mockEvent, mockClient, mockCreateOpencodeClient, mockSpawn } = vi.hoisted(() => {
  const mockSession = {
    create: vi.fn(),
    prompt: vi.fn(),
    promptAsync: vi.fn(),
    messages: vi.fn(),
    abort: vi.fn(),
    list: vi.fn(),
  };

  const mockEvent = {
    subscribe: vi.fn(),
  };

  const mockClient = {
    session: mockSession,
    event: mockEvent,
  };

  const mockCreateOpencodeClient = vi.fn().mockReturnValue(mockClient);
  const mockSpawn = vi.fn();

  return { mockSession, mockEvent, mockClient, mockCreateOpencodeClient, mockSpawn };
});

// Mock the SDK
vi.mock('@opencode-ai/sdk', () => ({
  createOpencodeClient: mockCreateOpencodeClient,
}));

// Mock child_process
vi.mock('child_process', () => ({
  spawn: mockSpawn,
}));

import { TaskExecutor } from './executor.js';

// Create the mock server process outside of vi.hoisted (EventEmitter can't be hoisted)
let mockServerProcess: EventEmitter & {
  stdout: EventEmitter;
  stderr: EventEmitter;
  stdin: null;
  pid: number;
  kill: ReturnType<typeof vi.fn>;
  killed: boolean;
};

function createMockServerProcess() {
  const proc = Object.assign(new EventEmitter(), {
    stdout: new EventEmitter(),
    stderr: new EventEmitter(),
    stdin: null as null,
    pid: 12345,
    kill: vi.fn(),
    killed: false,
  });
  return proc;
}

describe('TaskExecutor (SDK Mode)', () => {
  let executor: TaskExecutor;

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Create fresh mock server process for each test
    mockServerProcess = createMockServerProcess();
    mockSpawn.mockReturnValue(mockServerProcess);
    
    // Reset mocks with default successful behavior
    mockSession.list.mockResolvedValue({
      data: [],
    });
    mockSession.create.mockResolvedValue({
      data: { id: 'session-123', title: 'Test Session' },
    });
    mockSession.prompt.mockResolvedValue({
      data: { info: { id: 'msg-1' }, parts: [{ type: 'text', text: 'Hello!' }] },
    });
    mockSession.promptAsync.mockResolvedValue({
      data: { info: { id: 'msg-1' }, parts: [{ type: 'text', text: 'Hello!' }] },
    });
    mockSession.messages.mockResolvedValue({
      data: [{ parts: [{ type: 'text', text: 'Response' }] }],
    });
    mockSession.abort.mockResolvedValue({ data: true });
    mockEvent.subscribe.mockResolvedValue({
      stream: (async function* () {})(),
    });
    
    executor = new TaskExecutor({
      defaultCwd: process.cwd(),
      defaultAgent: 'opencode',
    });
  });

  afterEach(async () => {
    await executor.shutdown();
  });

  // Helper to simulate server startup
  const simulateServerStart = (url = 'http://127.0.0.1:4096') => {
    // Simulate server starting and outputting URL
    setTimeout(() => {
      mockServerProcess.stdout.emit('data', `opencode server listening on ${url}\n`);
    }, 10);
  };

  describe('execute', () => {
    it('should emit events during execution', async () => {
      simulateServerStart();
      
      const events: Array<{ taskId: string; event: GatewayAgentEvent }> = [];
      executor.on('event', (e) => events.push(e));

      const result = await executor.execute({
        taskId: 'test-1',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      // Should have emitted events
      expect(events.length).toBeGreaterThan(0);
      expect(events[0].taskId).toBe('test-1');
      expect(result.success).toBe(true);
    });

    it('should return success for successful execution', async () => {
      simulateServerStart();
      
      const result = await executor.execute({
        taskId: 'success-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      expect(result.success).toBe(true);
      expect(result.duration).toBeDefined();
      expect(mockSession.create).toHaveBeenCalled();
      expect(mockSession.promptAsync).toHaveBeenCalled();
    });

    it('should return failure when session creation fails', async () => {
      simulateServerStart();
      mockSession.create.mockResolvedValue({ data: null });

      const events: Array<{ taskId: string; event: GatewayAgentEvent }> = [];
      executor.on('event', (e) => events.push(e));

      const result = await executor.execute({
        taskId: 'fail-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      expect(result.success).toBe(false);
      expect(result.output).toContain('Failed to create session');
    });

    it('should parse model string correctly', async () => {
      simulateServerStart();
      
      await executor.execute({
        taskId: 'model-test',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'opencode',
        model: 'anthropic/claude-3-sonnet',
        timeout: 5000,
      });

      // Verify prompt was called with parsed model
      expect(mockSession.promptAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          body: expect.objectContaining({
            model: { providerID: 'anthropic', modelID: 'claude-3-sonnet' },
          }),
        })
      );
    });
    
    it('should handle complex model strings with slashes', async () => {
      simulateServerStart();
      
      await executor.execute({
        taskId: 'model-test-2',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'opencode',
        model: 'google/gemini-2.0-flash-preview',
        timeout: 5000,
      });

      expect(mockSession.promptAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          body: expect.objectContaining({
            model: { providerID: 'google', modelID: 'gemini-2.0-flash-preview' },
          }),
        })
      );
    });

    it('should spawn opencode serve command', async () => {
      simulateServerStart();
      
      await executor.execute({
        taskId: 'spawn-test',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      // Check spawn was called with correct arguments
      expect(mockSpawn).toHaveBeenCalledWith(
        expect.stringMatching(/opencode(\.cmd)?/),
        ['serve', '--hostname=127.0.0.1', '--port=0'],
        expect.objectContaining({
          cwd: process.cwd(),
        })
      );
    });

    it('should connect client with server URL', async () => {
      simulateServerStart('http://127.0.0.1:9999');
      
      await executor.execute({
        taskId: 'client-test',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      // Check client was created with correct URL
      expect(mockCreateOpencodeClient).toHaveBeenCalledWith({
        baseUrl: 'http://127.0.0.1:9999',
      });
    });

    it('should reject cwd outside allowlist', async () => {
      const safeRoot = path.join(process.cwd(), 'projects');
      executor = new TaskExecutor({
        defaultCwd: process.cwd(),
        defaultAgent: 'opencode',
        allowedRoots: [safeRoot],
      });

      const result = await executor.execute({
        taskId: 'blocked-cwd',
        prompt: 'test',
        cwd: path.join(process.cwd(), 'outside'),
        agentType: 'opencode',
      });

      expect(result.success).toBe(false);
      expect(result.output).toContain('outside allowed project roots');
      expect(mockSpawn).not.toHaveBeenCalled();
    });

    it('should allow cwd inside allowlist', async () => {
      simulateServerStart();
      const safeRoot = path.join(process.cwd(), 'projects');
      executor = new TaskExecutor({
        defaultCwd: process.cwd(),
        defaultAgent: 'opencode',
        allowedRoots: [safeRoot],
      });

      const result = await executor.execute({
        taskId: 'allowed-cwd',
        prompt: 'test',
        cwd: path.join(safeRoot, 'repo-a'),
        agentType: 'opencode',
      });

      expect(result.success).toBe(true);
      expect(mockSpawn).toHaveBeenCalled();
    });
  });

  describe('abort', () => {
    it('should return false for non-existent task', () => {
      const aborted = executor.abort('non-existent');
      expect(aborted).toBe(false);
    });
  });

  describe('sendInput', () => {
    it('should return false (not supported in SDK mode)', () => {
      const sent = executor.sendInput('any-task', 'input');
      expect(sent).toBe(false);
    });
  });

  describe('activeTaskCount', () => {
    it('should return 0 initially', () => {
      expect(executor.activeTaskCount).toBe(0);
    });

    it('should return 0 when no tasks running', () => {
      expect(executor.activeTaskIds).toEqual([]);
    });
  });

  describe('shutdown', () => {
    it('should kill server process', async () => {
      simulateServerStart();
      
      // Execute something to initialize
      await executor.execute({
        taskId: 'init-test',
        prompt: 'init',
        cwd: process.cwd(),
        agentType: 'opencode',
      });

      await executor.shutdown();

      expect(mockServerProcess.kill).toHaveBeenCalled();
    });
  });
});
