import { describe, it, expect, vi, beforeEach, afterEach, Mock } from 'vitest';
import { EventEmitter } from 'events';
import { TaskExecutor } from './executor.js';
import type { TaskRequest, GatewayAgentEvent } from './types.js';

// Mock child_process spawn
vi.mock('child_process', () => {
  return {
    spawn: vi.fn(),
  };
});

import { spawn } from 'child_process';

// Helper to create a mock child process
function createMockChildProcess(options: {
  exitCode?: number;
  stdout?: string;
  stderr?: string;
  exitDelay?: number;
  noExitOnKill?: boolean;
} = {}) {
  const { exitCode = 0, stdout = '', stderr = '', exitDelay = 10, noExitOnKill = false } = options;
  
  const mockProcess = new EventEmitter() as any;
  mockProcess.stdout = new EventEmitter();
  mockProcess.stderr = new EventEmitter();
  mockProcess.stdin = {
    write: vi.fn(),
    end: vi.fn(),
  };
  mockProcess.killed = false;
  mockProcess.kill = vi.fn(() => {
    mockProcess.killed = true;
    // Only emit exit if noExitOnKill is false
    if (!noExitOnKill) {
      mockProcess.emit('exit', null);
    }
  });

  // Simulate async execution
  setTimeout(() => {
    if (stdout) {
      mockProcess.stdout.emit('data', Buffer.from(stdout));
    }
    if (stderr) {
      mockProcess.stderr.emit('data', Buffer.from(stderr));
    }
    setTimeout(() => {
      // Don't emit exit if already killed
      if (!mockProcess.killed) {
        mockProcess.emit('exit', exitCode);
      }
    }, exitDelay);
  }, 1);

  return mockProcess;
}

describe('TaskExecutor', () => {
  let executor: TaskExecutor;

  beforeEach(() => {
    vi.clearAllMocks();
    executor = new TaskExecutor({
      defaultCwd: process.cwd(),
      defaultAgent: 'opencode',
    });
  });

  afterEach(() => {
    // Abort any running tasks
    for (const taskId of executor.activeTaskIds) {
      executor.abort(taskId);
    }
    vi.restoreAllMocks();
  });

  describe('execute', () => {
    it('should emit events during execution', async () => {
      const mockChild = createMockChildProcess({ stdout: 'hello world\n', exitCode: 0 });
      (spawn as Mock).mockReturnValue(mockChild);

      const events: Array<{ taskId: string; event: GatewayAgentEvent }> = [];
      executor.on('event', (e) => events.push(e));

      await executor.execute({
        taskId: 'test-1',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      // Executor should have emitted events
      expect(events.length).toBeGreaterThan(0);
      expect(events[0].taskId).toBe('test-1');
      
      // First event should be the executor startup log
      expect(events[0].event.type).toBe('log');
      expect(events[0].event.content).toContain('[executor]');
    });

    it('should track active tasks', async () => {
      expect(executor.activeTaskCount).toBe(0);
      
      // Create a long-running mock process
      const mockChild = createMockChildProcess({ exitDelay: 5000 });
      (spawn as Mock).mockReturnValue(mockChild);

      const promise = executor.execute({
        taskId: 'long-task',
        prompt: 'long running',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 30000,
      });

      // Give it time to start
      await new Promise((r) => setTimeout(r, 50));
      
      expect(executor.activeTaskCount).toBe(1);
      expect(executor.activeTaskIds).toContain('long-task');
      
      // Abort it
      executor.abort('long-task');
      
      await new Promise((r) => setTimeout(r, 50));
      
      expect(executor.activeTaskCount).toBe(0);
      
      // Wait for promise to settle
      try {
        await promise;
      } catch {
        // Expected to fail due to abort
      }
    });

    it('should return success for successful command', async () => {
      const mockChild = createMockChildProcess({ 
        stdout: 'success output\n', 
        exitCode: 0 
      });
      (spawn as Mock).mockReturnValue(mockChild);

      const result = await executor.execute({
        taskId: 'success-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      expect(result.success).toBe(true);
      expect(result.exitCode).toBe(0);
    });

    it('should return failure for failing command', async () => {
      const mockChild = createMockChildProcess({ exitCode: 1 });
      (spawn as Mock).mockReturnValue(mockChild);

      const result = await executor.execute({
        taskId: 'fail-test',
        prompt: 'failing command',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      expect(result.success).toBe(false);
      expect(result.exitCode).toBe(1);
    });

    it('should emit error events for stderr output', async () => {
      const mockChild = createMockChildProcess({ 
        stderr: 'error output\n', 
        exitCode: 1 
      });
      (spawn as Mock).mockReturnValue(mockChild);

      const events: Array<{ taskId: string; event: GatewayAgentEvent }> = [];
      executor.on('event', (e) => events.push(e));

      await executor.execute({
        taskId: 'stderr-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      const errorEvent = events.find(e => e.event.type === 'error');
      expect(errorEvent).toBeDefined();
      expect(errorEvent?.event.content).toContain('error output');
    });

    it('should parse JSON output events', async () => {
      const jsonEvent = JSON.stringify({ type: 'message', content: 'Hello from agent' });
      const mockChild = createMockChildProcess({ 
        stdout: jsonEvent + '\n', 
        exitCode: 0 
      });
      (spawn as Mock).mockReturnValue(mockChild);

      const events: Array<{ taskId: string; event: GatewayAgentEvent }> = [];
      executor.on('event', (e) => events.push(e));

      await executor.execute({
        taskId: 'json-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      const messageEvent = events.find(e => e.event.type === 'message');
      expect(messageEvent).toBeDefined();
    });

    it('should use correct command for different agent types', async () => {
      const mockChild = createMockChildProcess({ exitCode: 0 });
      (spawn as Mock).mockReturnValue(mockChild);

      await executor.execute({
        taskId: 'agent-type-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'claude-code',
        timeout: 5000,
      });

      expect(spawn).toHaveBeenCalledWith(
        process.platform === 'win32' ? 'claude.cmd' : 'claude',
        expect.arrayContaining(['test prompt']),
        expect.any(Object)
      );
    });

    it('should handle timeout', async () => {
      // Use noExitOnKill so timeout rejection happens before exit event
      const mockChild = createMockChildProcess({ exitDelay: 10000, noExitOnKill: true });
      (spawn as Mock).mockReturnValue(mockChild);

      await expect(
        executor.execute({
          taskId: 'timeout-test',
          prompt: 'test',
          cwd: process.cwd(),
          agentType: 'opencode',
          timeout: 50,
        })
      ).rejects.toThrow('timeout');
    });

    it('should pass environment variables', async () => {
      const mockChild = createMockChildProcess({ exitCode: 0 });
      (spawn as Mock).mockReturnValue(mockChild);

      await executor.execute({
        taskId: 'env-test',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'opencode',
        env: { CUSTOM_VAR: 'custom_value' },
        timeout: 5000,
      });

      expect(spawn).toHaveBeenCalledWith(
        expect.any(String),
        expect.any(Array),
        expect.objectContaining({
          env: expect.objectContaining({
            CUSTOM_VAR: 'custom_value',
            CI: '1',
            NO_COLOR: '1',
          }),
        })
      );
    });
  });

  describe('abort', () => {
    it('should abort running task', async () => {
      const mockChild = createMockChildProcess({ exitDelay: 5000 });
      (spawn as Mock).mockReturnValue(mockChild);

      const promise = executor.execute({
        taskId: 'abort-test',
        prompt: 'long running',
        cwd: process.cwd(),
        agentType: 'opencode',
      });

      await new Promise((r) => setTimeout(r, 50));
      
      const aborted = executor.abort('abort-test');
      expect(aborted).toBe(true);
      expect(mockChild.kill).toHaveBeenCalled();
      
      // Should no longer be active
      await new Promise((r) => setTimeout(r, 50));
      expect(executor.activeTaskIds).not.toContain('abort-test');
      
      try {
        await promise;
      } catch {
        // Expected
      }
    });

    it('should return false for non-existent task', () => {
      const aborted = executor.abort('non-existent');
      expect(aborted).toBe(false);
    });
  });

  describe('sendInput', () => {
    it('should write to stdin of running task', async () => {
      const mockChild = createMockChildProcess({ exitDelay: 5000 });
      (spawn as Mock).mockReturnValue(mockChild);

      const promise = executor.execute({
        taskId: 'input-test',
        prompt: 'interactive',
        cwd: process.cwd(),
        agentType: 'opencode',
      });

      await new Promise((r) => setTimeout(r, 50));

      const sent = executor.sendInput('input-test', 'user input');
      expect(sent).toBe(true);
      expect(mockChild.stdin.write).toHaveBeenCalledWith('user input\n');

      executor.abort('input-test');
      try { await promise; } catch { /* expected */ }
    });

    it('should return false for non-existent task', () => {
      const sent = executor.sendInput('non-existent', 'input');
      expect(sent).toBe(false);
    });
  });

  describe('activeTaskCount', () => {
    it('should return correct count', async () => {
      expect(executor.activeTaskCount).toBe(0);
      
      const mockChild1 = createMockChildProcess({ exitDelay: 5000 });
      const mockChild2 = createMockChildProcess({ exitDelay: 5000 });
      (spawn as Mock)
        .mockReturnValueOnce(mockChild1)
        .mockReturnValueOnce(mockChild2);

      const p1 = executor.execute({
        taskId: 'task-1',
        prompt: 'task 1',
        cwd: process.cwd(),
        agentType: 'opencode',
      });
      
      const p2 = executor.execute({
        taskId: 'task-2',
        prompt: 'task 2',
        cwd: process.cwd(),
        agentType: 'opencode',
      });

      await new Promise((r) => setTimeout(r, 50));
      
      expect(executor.activeTaskCount).toBe(2);
      expect(executor.activeTaskIds).toContain('task-1');
      expect(executor.activeTaskIds).toContain('task-2');
      
      // Abort both
      executor.abort('task-1');
      executor.abort('task-2');
      
      try { await p1; } catch { /* expected */ }
      try { await p2; } catch { /* expected */ }
    });
  });

  describe('resolveCommand', () => {
    it('should emit log event with command info', async () => {
      const mockChild = createMockChildProcess({ exitCode: 0 });
      (spawn as Mock).mockReturnValue(mockChild);

      const events: Array<{ taskId: string; event: GatewayAgentEvent }> = [];
      executor.on('event', (e) => events.push(e));

      await executor.execute({
        taskId: 'cmd-test',
        prompt: 'test prompt',
        cwd: process.cwd(),
        agentType: 'opencode',
        timeout: 5000,
      });

      // Check that an event mentions the executor log
      const logEvent = events.find(
        (e) => e.event.type === 'log' && e.event.content?.includes('[executor]')
      );
      expect(logEvent).toBeDefined();
      expect(logEvent?.event.content).toContain('opencode');
    });

    it('should use gemini command for gemini agent type', async () => {
      const mockChild = createMockChildProcess({ exitCode: 0 });
      (spawn as Mock).mockReturnValue(mockChild);

      await executor.execute({
        taskId: 'gemini-test',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'gemini',
        timeout: 5000,
      });

      expect(spawn).toHaveBeenCalledWith(
        process.platform === 'win32' ? 'gemini.cmd' : 'gemini',
        expect.any(Array),
        expect.any(Object)
      );
    });
  });

  describe('resolveArgs', () => {
    it('should add model flag when model is provided', async () => {
      const mockChild = createMockChildProcess({ exitCode: 0 });
      (spawn as Mock).mockReturnValue(mockChild);

      await executor.execute({
        taskId: 'model-test',
        prompt: 'test',
        cwd: process.cwd(),
        agentType: 'opencode',
        model: 'claude-3-sonnet',
        timeout: 5000,
      });

      expect(spawn).toHaveBeenCalledWith(
        expect.any(String),
        expect.arrayContaining(['-m', 'claude-3-sonnet']),
        expect.any(Object)
      );
    });
  });
});
