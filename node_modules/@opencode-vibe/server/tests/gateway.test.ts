import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import { io as ioc, Socket as ClientSocket } from 'socket.io-client';
import { startServer } from '../src/server';
import { AddressInfo } from 'net';

// Mock PtyManager
vi.mock('@opencode-vibe/pty-manager', () => {
  return {
    PtyManager: vi.fn().mockImplementation(() => ({
      spawn: vi.fn().mockReturnValue({
        on: vi.fn(),
        write: vi.fn(),
        kill: vi.fn(),
      }),
    })),
  };
});

describe('Gateway Server', () => {
  let clientSocket: ClientSocket;
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
  });

  it('should allow a client to connect', () => new Promise<void>((done) => {
    clientSocket = ioc(`http://localhost:${port}`);
    
    clientSocket.on('connect', () => {
      expect(clientSocket.connected).toBe(true);
      done();
    });
  }));
});
