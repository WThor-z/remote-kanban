import { describe, it, expect, beforeAll, afterAll, afterEach, vi } from 'vitest';
import { io as ioc, Socket as ClientSocket } from 'socket.io-client';
import { startServer } from '../src/server';
import { AddressInfo } from 'net';

// Mock OpencodeClient to avoid actual OpenCode processes during tests
vi.mock('../src/agent/opencode-client', () => {
  return {
    OpencodeClient: vi.fn().mockImplementation(() => ({
      start: vi.fn().mockResolvedValue('http://127.0.0.1:12345'),
      stop: vi.fn(),
      waitForHealth: vi.fn().mockResolvedValue(true),
      createSession: vi.fn().mockResolvedValue('mock-session-id'),
      sendMessage: vi.fn().mockResolvedValue(undefined),
      connectEventStream: vi.fn().mockResolvedValue(undefined),
      on: vi.fn(),
      emit: vi.fn(),
    })),
  };
});

describe('Gateway Server', () => {
  let clientSocket: ClientSocket;
  let secondarySocket: ClientSocket | null = null;
  let httpServer: Awaited<ReturnType<typeof startServer>>['httpServer'];
  let port: number;
  let stopServer: () => void;

  beforeAll(async () => {
    // Start the server on port 0 for random available port
    const app = await startServer(0);
    httpServer = app.httpServer;
    stopServer = app.stop;
    
    await new Promise<void>((resolve) => {
        httpServer.on('listening', () => {
            port = (httpServer.address() as AddressInfo).port;
            resolve();
        });
        // If already listening
        if (httpServer.listening) {
            port = (httpServer.address() as AddressInfo).port;
            resolve();
        }
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

  it('should allow a client to connect', () => new Promise<void>((done) => {
    clientSocket = ioc(`http://localhost:${port}`);
    
    clientSocket.on('connect', () => {
      expect(clientSocket.connected).toBe(true);
      done();
    });
  }));

  it('should handle multiple client connections', async () => {
    clientSocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      clientSocket.on('connect', () => resolve());
    });

    secondarySocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      secondarySocket?.on('connect', () => resolve());
    });

    // Both clients should remain connected
    expect(clientSocket.connected).toBe(true);
    expect(secondarySocket?.connected).toBe(true);
  });

  it('should emit disconnect event when client disconnects', async () => {
    clientSocket = ioc(`http://localhost:${port}`);

    await new Promise<void>((resolve) => {
      clientSocket.on('connect', () => resolve());
    });

    expect(clientSocket.connected).toBe(true);
    
    clientSocket.disconnect();
    
    expect(clientSocket.connected).toBe(false);
  });
});
