import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as pty from 'node-pty';
import { PtyManager } from '../src/index';

// Mock node-pty
vi.mock('node-pty', () => {
  return {
    spawn: vi.fn(),
  };
});

describe('PtyManager', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should spawn a process using node-pty', () => {
    const manager = new PtyManager();
    const mockTerminal = {
        onData: vi.fn(),
        write: vi.fn(),
        resize: vi.fn(),
        kill: vi.fn(),
        onExit: vi.fn(),
        pid: 123
    };
    
    // Setup the mock return value
    vi.mocked(pty.spawn).mockReturnValue(mockTerminal as any);

    const shell = 'bash';
    const args = ['-c', 'echo hello'];
    
    // Call the method
    const term = manager.spawn(shell, args);

    // Assertions
    expect(pty.spawn).toHaveBeenCalledTimes(1);
    expect(pty.spawn).toHaveBeenCalledWith(shell, args, expect.any(Object));
    expect(term).toBe(mockTerminal);
  });
});
