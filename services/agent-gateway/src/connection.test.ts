import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { WebSocketServer, WebSocket as WsWebSocket } from 'ws';
import { GatewayConnection } from './connection.js';
import type { GatewayOptions, HostCapabilities, ServerToGatewayMessage } from './types.js';

describe('GatewayConnection', () => {
  let wss: WebSocketServer;
  let serverPort: number;
  let serverMessages: any[];
  let serverSocket: WsWebSocket | null;
  let activeConnections: GatewayConnection[] = [];

  const testCapabilities: HostCapabilities = {
    name: 'Test Host',
    agents: ['opencode'],
    maxConcurrent: 2,
    cwd: '/tmp/test',
  };

  // Helper to create connection with proper cleanup tracking
  function createConnection(overrides: Partial<GatewayOptions> = {}) {
    const options: GatewayOptions = {
      serverUrl: `ws://localhost:${serverPort}`,
      hostId: 'test-host',
      authToken: 'test-token',
      capabilities: testCapabilities,
      reconnect: false,
      ...overrides,
    };
    
    const conn = new GatewayConnection(options);
    
    // Always add error handler to prevent unhandled errors
    conn.on('error', () => {});
    
    activeConnections.push(conn);
    return conn;
  }

  beforeEach(async () => {
    serverMessages = [];
    serverSocket = null;
    activeConnections = [];
    
    // Create test WebSocket server
    await new Promise<void>((resolve) => {
      wss = new WebSocketServer({ port: 0 }, () => {
        const address = wss.address();
        serverPort = typeof address === 'object' && address !== null ? address.port : 0;
        resolve();
      });
    });

    wss.on('connection', (ws) => {
      serverSocket = ws;
      ws.on('message', (data) => {
        const msg = JSON.parse(data.toString());
        serverMessages.push(msg);
        
        // Auto-respond to register
        if (msg.type === 'register') {
          ws.send(JSON.stringify({ type: 'registered', ok: true }));
        }
      });
    });
  });

  afterEach(async () => {
    // Disconnect all active connections first
    for (const conn of activeConnections) {
      conn.disconnect();
    }
    activeConnections = [];
    
    // Give time for any pending operations to complete
    await new Promise((r) => setTimeout(r, 50));
    
    wss.close();
  });

  describe('connect', () => {
    it('should connect and send register message', async () => {
      const conn = createConnection();

      await conn.connect();
      
      // Wait for registration
      await new Promise((r) => setTimeout(r, 100));

      expect(conn.isConnected).toBe(true);
      expect(serverMessages.length).toBeGreaterThanOrEqual(1);
      expect(serverMessages[0].type).toBe('register');
      expect(serverMessages[0].hostId).toBe('test-host');
    });

    it('should throw if already connecting', async () => {
      const conn = createConnection();

      const promise1 = conn.connect();
      
      await expect(conn.connect()).rejects.toThrow('Cannot connect');
      
      await promise1;
    });
  });

  describe('send', () => {
    it('should send messages to server', async () => {
      const conn = createConnection();

      await conn.connect();
      await new Promise((r) => setTimeout(r, 100));

      conn.send({ type: 'heartbeat', timestamp: 12345 });
      
      await new Promise((r) => setTimeout(r, 50));
      
      const heartbeat = serverMessages.find((m) => m.type === 'heartbeat');
      expect(heartbeat).toBeDefined();
      expect(heartbeat.timestamp).toBe(12345);
    });
  });

  describe('message handling', () => {
    it('should emit message events', async () => {
      const conn = createConnection();

      const messages: ServerToGatewayMessage[] = [];
      conn.on('message', (msg) => messages.push(msg));

      await conn.connect();
      await new Promise((r) => setTimeout(r, 100));

      // Server sends a ping
      serverSocket?.send(JSON.stringify({ type: 'ping' }));
      
      await new Promise((r) => setTimeout(r, 50));
      
      expect(messages.some((m) => m.type === 'ping')).toBe(true);
    });

    it('should respond to ping with heartbeat', async () => {
      const conn = createConnection();

      await conn.connect();
      await new Promise((r) => setTimeout(r, 100));

      serverMessages.length = 0; // Clear previous messages
      
      serverSocket?.send(JSON.stringify({ type: 'ping' }));
      
      await new Promise((r) => setTimeout(r, 100));
      
      const heartbeat = serverMessages.find((m) => m.type === 'heartbeat');
      expect(heartbeat).toBeDefined();
    });
  });

  describe('disconnect', () => {
    it('should cleanup on disconnect', async () => {
      const conn = createConnection();

      await conn.connect();
      await new Promise((r) => setTimeout(r, 100));
      
      expect(conn.isConnected).toBe(true);
      
      conn.disconnect();
      
      expect(conn.isConnected).toBe(false);
    });
  });

  describe('reconnection', () => {
    it('should attempt reconnect when enabled', async () => {
      const conn = createConnection({ reconnect: true });

      await conn.connect();
      await new Promise((r) => setTimeout(r, 100));

      const stateChanges: any[] = [];
      conn.on('stateChange', (state) => stateChanges.push(state));

      // Close server connection
      serverSocket?.close();
      
      await new Promise((r) => setTimeout(r, 200));
      
      // Should have disconnected state
      expect(stateChanges.some((s) => s.status === 'disconnected')).toBe(true);
      
      // Disconnect before reconnect timer fires (delay is ~1000ms)
      conn.disconnect();
    });
  });
});
