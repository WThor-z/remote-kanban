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

  it('should write data to the terminal', () => {
    const manager = new PtyManager();
    const mockTerminal = {
      write: vi.fn(),
    } as unknown as pty.IPty;

    manager.write(mockTerminal, 'ls');

    expect(mockTerminal.write).toHaveBeenCalledWith('ls');
  });

  it('should subscribe to terminal output', () => {
    const manager = new PtyManager();
    const subscription = { dispose: vi.fn() };
    const mockTerminal = {
      onData: vi.fn().mockReturnValue(subscription),
    } as unknown as pty.IPty;

    const handler = vi.fn();
    const result = manager.onData(mockTerminal, handler);

    expect(mockTerminal.onData).toHaveBeenCalledWith(handler);
    expect(result).toBe(subscription);
  });

  it('should resize the terminal', () => {
    const manager = new PtyManager();
    const mockTerminal = {
      resize: vi.fn(),
    } as unknown as pty.IPty;

    manager.resize(mockTerminal, 120, 40);

    expect(mockTerminal.resize).toHaveBeenCalledWith(120, 40);
  });
});
