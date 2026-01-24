import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach, vi } from 'vitest';
import { io as ioc, Socket as ClientSocket } from 'socket.io-client';
import { startServer } from '../src/server';
import { AddressInfo } from 'net';

type OnDataHandler = (data: string) => void;

let onDataHandler: OnDataHandler | null = null;
const writeMock = vi.fn();
const killMock = vi.fn();
const disposeMock = vi.fn();
const spawnMock = vi.fn();

const waitForCondition = (condition: () => boolean, timeoutMs: number = 1000) => {
  return new Promise<void>((resolve, reject) => {
    const interval = setInterval(() => {
      if (condition()) {
        clearInterval(interval);
        resolve();
      }
    }, 10);

    setTimeout(() => {
      clearInterval(interval);
      reject(new Error('Timed out waiting for condition'));
    }, timeoutMs);
  });
};

vi.mock('@opencode-vibe/pty-manager', () => {
  return {
    PtyManager: vi.fn().mockImplementation(() => ({
      spawn: spawnMock.mockImplementation(() => ({
        onData: vi.fn((handler: OnDataHandler) => {
          onDataHandler = handler;
          return { dispose: disposeMock };
        }),
        write: writeMock,
        kill: killMock,
      })),
    })),
  };
});

describe('Gateway Server', () => {
  let clientSocket: ClientSocket;
  let secondarySocket: ClientSocket | null = null;
  let httpServer: any;
  let port: number;
  let stopServer: () => void;

  beforeAll(async () => {
    // Start the server on port 0 for random available port
    const app = startServer(0);
    httpServer = app.httpServer;
    stopServer = app.stop;
    
    await new Promise<void>((resolve) => {
        httpServer.on('listening', () => {
            port = (httpServer.address() as AddressInfo).port;
            resolve();
        });
    });
  });

  afterAll(() => {
    if (stopServer) stopServer();
    if (clientSocket) clientSocket.disconnect();
    if (secondarySocket) secondarySocket.disconnect();
  });

  afterEach(() => {
    if (clientSocket && clientSocket.connected) {
      clientSocket.disconnect();
    }
    if (secondarySocket && secondarySocket.connected) {
      secondarySocket.disconnect();
    }
    secondarySocket = null;
  });

  beforeEach(() => {
    onDataHandler = null;
    vi.clearAllMocks();
  });

  it('should allow a client to connect', () => new Promise<void>((done) => {
    clientSocket = ioc(`http://localhost:${port}`);
    
    clientSocket.on('connect', () => {
      expect(clientSocket.connected).toBe(true);
      done();
    });
  }));

  it('should spawn a shell and forward input to PTY', async () => {
    clientSocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      clientSocket.on('connect', () => resolve());
    });

    await waitForCondition(() => spawnMock.mock.calls.length > 0);

    clientSocket.emit('input', 'echo test');

    await waitForCondition(() => writeMock.mock.calls.length > 0);
    expect(writeMock).toHaveBeenCalledWith('echo test');
  });

  it('should forward PTY output to client', async () => {
    clientSocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      clientSocket.on('connect', () => resolve());
    });

    await waitForCondition(() => onDataHandler !== null);

    await new Promise<void>((resolve) => {
      clientSocket.on('output', (data: string) => {
        expect(data).toBe('hello');
        resolve();
      });

      onDataHandler?.('hello');
    });
  });

  it('should spawn separate shells for multiple clients', async () => {
    clientSocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      clientSocket.on('connect', () => resolve());
    });

    const initialSpawnCount = spawnMock.mock.calls.length;

    secondarySocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      secondarySocket?.on('connect', () => resolve());
    });

    // Wait for second shell to spawn
    await waitForCondition(() => spawnMock.mock.calls.length > initialSpawnCount);
    
    // Both clients should remain connected
    expect(clientSocket.connected).toBe(true);
    expect(secondarySocket?.connected).toBe(true);
    
    // Should have spawned shells for both
    expect(spawnMock.mock.calls.length).toBeGreaterThanOrEqual(2);
  });
});
