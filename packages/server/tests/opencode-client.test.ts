import { describe, it, expect, beforeEach, afterEach, vi, Mock } from 'vitest';
import { EventEmitter } from 'events';

// Mock child_process
const mockProcess = {
  stdout: new EventEmitter(),
  stderr: new EventEmitter(),
  kill: vi.fn(),
  on: vi.fn(),
};

vi.mock('child_process', () => ({
  spawn: vi.fn(() => mockProcess),
}));

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch;

// Import after mocks
import { OpencodeClient } from '../src/agent/opencode-client';
import { spawn } from 'child_process';

describe('OpencodeClient', () => {
  let client: OpencodeClient;

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Reset mock process
    mockProcess.stdout = new EventEmitter();
    mockProcess.stderr = new EventEmitter();
    mockProcess.kill.mockReset();
    mockProcess.on.mockReset();
    
    // Reset fetch mock
    mockFetch.mockReset();
    
    client = new OpencodeClient({ cwd: '/test/dir' });
  });

  afterEach(() => {
    client.stop();
  });

  describe('constructor', () => {
    it('should create client with default config', () => {
      const defaultClient = new OpencodeClient();
      expect(defaultClient.isRunning).toBe(false);
      expect(defaultClient.serverUrl).toBeNull();
    });

    it('should create client with custom config', () => {
      const customClient = new OpencodeClient({
        cwd: '/custom/path',
        env: { CUSTOM_VAR: 'value' },
      });
      expect(customClient.isRunning).toBe(false);
    });
  });

  describe('start', () => {
    it('should spawn opencode serve process', async () => {
      // Simulate server startup output
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      const url = await client.start();

      expect(spawn).toHaveBeenCalledWith(
        'opencode',
        ['serve', '--hostname', '127.0.0.1', '--port', '0'],
        expect.objectContaining({
          cwd: '/test/dir',
          shell: true,
        })
      );
      expect(url).toBe('http://127.0.0.1:4096');
      expect(client.isRunning).toBe(true);
      expect(client.serverUrl).toBe('http://127.0.0.1:4096');
    });

    it('should emit ready event when server starts', async () => {
      const readyHandler = vi.fn();
      client.on('ready', readyHandler);

      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:5000\n'));
      }, 10);

      await client.start();

      expect(readyHandler).toHaveBeenCalledWith('http://127.0.0.1:5000');
    });

    it('should emit output events for stdout', async () => {
      const outputHandler = vi.fn();
      client.on('output', outputHandler);

      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('some output\n'));
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();

      expect(outputHandler).toHaveBeenCalledWith({ type: 'stdout', data: 'some output\n' });
    });

    it('should emit output events for stderr', async () => {
      const outputHandler = vi.fn();
      client.on('output', outputHandler);

      setTimeout(() => {
        mockProcess.stderr.emit('data', Buffer.from('error output\n'));
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();

      expect(outputHandler).toHaveBeenCalledWith({ type: 'stderr', data: 'error output\n' });
    });

    it('should throw error if already running', async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();

      await expect(client.start()).rejects.toThrow('OpenCode server already running');
    });

    it('should handle process error', async () => {
      const error = new Error('spawn failed');
      
      setTimeout(() => {
        mockProcess.on.mock.calls.find(
          (call: [string, (...args: unknown[]) => void]) => call[0] === 'error'
        )?.[1](error);
      }, 10);

      // Setup the on handler
      mockProcess.on.mockImplementation((event: string, handler: (...args: unknown[]) => void) => {
        if (event === 'error') {
          setTimeout(() => handler(error), 10);
        }
      });

      await expect(client.start()).rejects.toThrow('spawn failed');
    });
  });

  describe('stop', () => {
    it('should kill the process and reset state', async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();
      expect(client.isRunning).toBe(true);

      client.stop();

      expect(mockProcess.kill).toHaveBeenCalled();
      expect(client.isRunning).toBe(false);
      expect(client.serverUrl).toBeNull();
    });

    it('should handle stop when not running', () => {
      // Should not throw
      expect(() => client.stop()).not.toThrow();
    });
  });

  describe('waitForHealth', () => {
    beforeEach(async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);
      await client.start();
    });

    it('should return true when server is healthy', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ healthy: true }),
      });

      const result = await client.waitForHealth();

      expect(result).toBe(true);
      expect(mockFetch).toHaveBeenCalledWith(
        'http://127.0.0.1:4096/global/health',
        expect.objectContaining({
          headers: expect.any(Object),
        })
      );
    });

    it('should retry until healthy', async () => {
      mockFetch
        .mockRejectedValueOnce(new Error('connection refused'))
        .mockResolvedValueOnce({
          ok: true,
          json: () => Promise.resolve({ healthy: true }),
        });

      const result = await client.waitForHealth();

      expect(result).toBe(true);
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it('should return false on timeout', async () => {
      mockFetch.mockRejectedValue(new Error('connection refused'));

      // Override timeout for faster test
      const originalDateNow = Date.now;
      let callCount = 0;
      Date.now = vi.fn(() => {
        callCount++;
        // Simulate timeout after a few calls
        return callCount > 5 ? originalDateNow() + 25000 : originalDateNow();
      });

      const result = await client.waitForHealth();

      expect(result).toBe(false);
      
      Date.now = originalDateNow;
    });

    it('should throw if server not started', async () => {
      const newClient = new OpencodeClient();
      await expect(newClient.waitForHealth()).rejects.toThrow('Server not started');
    });
  });

  describe('createSession', () => {
    beforeEach(async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);
      await client.start();
    });

    it('should create a session and return session ID', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ id: 'session-123' }),
      });

      const sessionId = await client.createSession();

      expect(sessionId).toBe('session-123');
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/session?directory='),
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
            'Authorization': expect.stringMatching(/^Basic /),
          }),
        })
      );
    });

    it('should throw on HTTP error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
      });

      await expect(client.createSession()).rejects.toThrow('Failed to create session: HTTP 500');
    });

    it('should throw if server not started', async () => {
      const newClient = new OpencodeClient();
      await expect(newClient.createSession()).rejects.toThrow('Server not started');
    });
  });

  describe('sendMessage', () => {
    beforeEach(async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);
      await client.start();
    });

    it('should send a message to the session', async () => {
      mockFetch.mockResolvedValueOnce({ ok: true });

      await client.sendMessage('session-123', 'Hello, OpenCode!');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/session/session-123/message?directory='),
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({
            parts: [{ type: 'text', text: 'Hello, OpenCode!' }],
          }),
        })
      );
    });

    it('should throw on HTTP error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        text: () => Promise.resolve('Bad request'),
      });

      await expect(client.sendMessage('session-123', 'test')).rejects.toThrow(
        'Failed to send message: HTTP 400 Bad request'
      );
    });

    it('should throw if server not started', async () => {
      const newClient = new OpencodeClient();
      await expect(newClient.sendMessage('session-123', 'test')).rejects.toThrow('Server not started');
    });
  });

  describe('abort', () => {
    beforeEach(async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);
      await client.start();
    });

    it('should send abort request', async () => {
      mockFetch.mockResolvedValueOnce({ ok: true });

      await client.abort('session-123');

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/session/session-123/abort?directory='),
        expect.objectContaining({
          method: 'POST',
        })
      );
    });

    it('should not throw on abort error', async () => {
      mockFetch.mockRejectedValueOnce(new Error('network error'));

      // Should not throw
      await expect(client.abort('session-123')).resolves.toBeUndefined();
    });

    it('should do nothing if server not started', async () => {
      const newClient = new OpencodeClient();
      await expect(newClient.abort('session-123')).resolves.toBeUndefined();
      expect(mockFetch).not.toHaveBeenCalled();
    });
  });

  describe('isRunning', () => {
    it('should return false when not started', () => {
      expect(client.isRunning).toBe(false);
    });

    it('should return true when running', async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();

      expect(client.isRunning).toBe(true);
    });

    it('should return false after stop', async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();
      client.stop();

      expect(client.isRunning).toBe(false);
    });
  });

  describe('serverUrl', () => {
    it('should return null when not started', () => {
      expect(client.serverUrl).toBeNull();
    });

    it('should return URL when running', async () => {
      setTimeout(() => {
        mockProcess.stdout.emit('data', Buffer.from('opencode server listening on http://127.0.0.1:4096\n'));
      }, 10);

      await client.start();

      expect(client.serverUrl).toBe('http://127.0.0.1:4096');
    });
  });
});
