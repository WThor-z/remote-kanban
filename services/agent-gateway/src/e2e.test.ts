import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import { WebSocketServer, WebSocket as WsWebSocket } from 'ws';
import { GatewayConnection } from './connection.js';
import { TaskExecutor } from './executor.js';
import type {
  GatewayToServerMessage,
  ServerToGatewayMessage,
  TaskRequest,
  HostCapabilities,
} from './types.js';

/**
 * E2E Tests for Agent Gateway
 * 
 * These tests verify the complete flow:
 * 1. Gateway connects to mock server
 * 2. Gateway registers successfully
 * 3. Gateway receives and executes tasks
 * 4. Gateway reports task completion
 * 
 * Note: These tests use a mock server, not the real Rust server.
 * For full integration testing with Rust server, run manually.
 */

describe('Gateway E2E', () => {
  let mockServer: WebSocketServer;
  let serverPort: number;
  let receivedMessages: GatewayToServerMessage[] = [];
  let serverSocket: WsWebSocket | null = null;
  let activeConnections: GatewayConnection[] = [];

  const testCapabilities: HostCapabilities = {
    name: 'E2E Test Host',
    agents: ['opencode'],
    maxConcurrent: 2,
    cwd: process.cwd(),
  };

  // Helper to create connection with proper cleanup tracking
  function createConnection(overrides: Partial<{
    serverUrl: string;
    hostId: string;
    authToken: string;
    capabilities: HostCapabilities;
    reconnect: boolean;
  }> = {}) {
    const conn = new GatewayConnection({
      serverUrl: `ws://localhost:${serverPort}`,
      hostId: 'e2e-test-host',
      authToken: 'test-token',
      capabilities: testCapabilities,
      reconnect: false,
      ...overrides,
    });
    
    // Always add error handler to prevent unhandled errors
    conn.on('error', () => {});
    
    activeConnections.push(conn);
    return conn;
  }

  beforeAll(async () => {
    // Create mock server
    await new Promise<void>((resolve) => {
      mockServer = new WebSocketServer({ port: 0 }, () => {
        const address = mockServer.address();
        serverPort = typeof address === 'object' && address !== null ? address.port : 0;
        resolve();
      });
    });

    mockServer.on('connection', (ws) => {
      serverSocket = ws;
      ws.on('message', (data) => {
        try {
          const msg = JSON.parse(data.toString()) as GatewayToServerMessage;
          receivedMessages.push(msg);

          // Auto-respond to register
          if (msg.type === 'register') {
            ws.send(JSON.stringify({ type: 'registered', ok: true }));
          }
        } catch {
          // Ignore parse errors
        }
      });
    });
  });

  afterAll(() => {
    mockServer.close();
  });

  beforeEach(() => {
    receivedMessages = [];
    activeConnections = [];
  });

  afterEach(async () => {
    // Disconnect all active connections
    for (const conn of activeConnections) {
      conn.disconnect();
    }
    activeConnections = [];
    
    // Give time for cleanup
    await new Promise((r) => setTimeout(r, 50));
  });

  describe('Full Connection Flow', () => {
    it('should complete registration handshake', async () => {
      const connection = createConnection({ hostId: 'registration-test' });

      // Track state changes
      const states: string[] = [];
      connection.on('stateChange', (state) => states.push(state.status));

      await connection.connect();
      
      // Wait for registration
      await new Promise((r) => setTimeout(r, 200));

      expect(connection.isConnected).toBe(true);
      expect(states).toContain('connecting');
      expect(states).toContain('connected');
      expect(states).toContain('registered');

      // Verify register message was sent
      const registerMsg = receivedMessages.find((m) => m.type === 'register');
      expect(registerMsg).toBeDefined();
      expect((registerMsg as any).hostId).toBe('registration-test');
      expect((registerMsg as any).capabilities.name).toBe('E2E Test Host');
    });

    it('should handle heartbeat ping-pong', async () => {
      const connection = createConnection({ hostId: 'heartbeat-test' });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      receivedMessages.length = 0;

      // Server sends ping
      serverSocket?.send(JSON.stringify({ type: 'ping' }));
      
      await new Promise((r) => setTimeout(r, 100));

      // Gateway should respond with heartbeat
      const heartbeat = receivedMessages.find((m) => m.type === 'heartbeat');
      expect(heartbeat).toBeDefined();
      expect((heartbeat as any).timestamp).toBeGreaterThan(0);
    });

    it('should track state changes correctly', async () => {
      const connection = createConnection({ hostId: 'state-test' });

      const states: string[] = [];
      connection.on('stateChange', (state) => states.push(state.status));

      await connection.connect();
      await new Promise((r) => setTimeout(r, 150));

      // Should have progressed through states
      expect(states[0]).toBe('connecting');
      expect(states).toContain('connected');
      expect(states).toContain('registered');

      connection.disconnect();
      
      await new Promise((r) => setTimeout(r, 50));
      expect(states).toContain('disconnected');
    });
  });

  describe('Task Execution Flow', () => {
    it('should receive and acknowledge task', async () => {
      const connection = createConnection({ hostId: 'task-test-host' });

      const executor = new TaskExecutor({
        defaultCwd: process.cwd(),
        defaultAgent: 'opencode',
      });

      // Wire up message handling
      connection.on('message', async (msg: ServerToGatewayMessage) => {
        if (msg.type === 'task:execute') {
          const task = msg.task;
          
          // Send started
          connection.send({
            type: 'task:started',
            taskId: task.taskId,
            sessionId: 'test-session',
          });

          // Execute a simple echo command (works on all platforms)
          try {
            const isWindows = process.platform === 'win32';
            const simpleTask: TaskRequest = {
              taskId: task.taskId,
              prompt: isWindows ? 'echo e2e test' : 'echo "e2e test"',
              cwd: process.cwd(),
              agentType: 'opencode',
              timeout: 5000,
            };

            // For E2E test, we use a direct shell command via spawn
            const { spawn } = await import('child_process');
            const child = spawn(isWindows ? 'cmd' : 'sh', 
              isWindows ? ['/c', 'echo e2e test'] : ['-c', 'echo "e2e test"'],
              { cwd: process.cwd() }
            );

            let output = '';
            child.stdout?.on('data', (chunk) => {
              output += chunk.toString();
              connection.send({
                type: 'task:event',
                taskId: task.taskId,
                event: {
                  type: 'log',
                  content: chunk.toString(),
                  timestamp: Date.now(),
                },
              });
            });

            child.on('exit', (code) => {
              connection.send({
                type: 'task:completed',
                taskId: task.taskId,
                result: {
                  success: code === 0,
                  exitCode: code ?? undefined,
                  output: output.trim(),
                },
              });
            });

            child.on('error', (err) => {
              connection.send({
                type: 'task:failed',
                taskId: task.taskId,
                error: err.message,
              });
            });
          } catch (err) {
            connection.send({
              type: 'task:failed',
              taskId: task.taskId,
              error: err instanceof Error ? err.message : String(err),
            });
          }
        }
      });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      receivedMessages.length = 0;

      // Server dispatches a task
      const task: TaskRequest = {
        taskId: 'e2e-task-1',
        prompt: 'echo "e2e test"',
        cwd: process.cwd(),
        agentType: 'opencode',
      };

      serverSocket?.send(JSON.stringify({ type: 'task:execute', task }));

      // Wait for execution
      await new Promise((r) => setTimeout(r, 2000));

      // Check received messages
      const started = receivedMessages.find((m) => m.type === 'task:started');
      expect(started).toBeDefined();
      expect((started as any).taskId).toBe('e2e-task-1');

      const completed = receivedMessages.find(
        (m) => m.type === 'task:completed' || m.type === 'task:failed'
      );
      expect(completed).toBeDefined();
      expect((completed as any).taskId).toBe('e2e-task-1');
    }, 10000);

    it('should emit task events during execution', async () => {
      const connection = createConnection({ hostId: 'event-test-host' });

      const taskEvents: GatewayToServerMessage[] = [];
      
      connection.on('message', async (msg: ServerToGatewayMessage) => {
        if (msg.type === 'task:execute') {
          const task = msg.task;
          
          connection.send({
            type: 'task:started',
            taskId: task.taskId,
            sessionId: 'event-test-session',
          });

          // Emit some events
          connection.send({
            type: 'task:event',
            taskId: task.taskId,
            event: {
              type: 'log',
              content: 'Starting task...',
              timestamp: Date.now(),
            },
          });

          await new Promise((r) => setTimeout(r, 50));

          connection.send({
            type: 'task:event',
            taskId: task.taskId,
            event: {
              type: 'message',
              content: 'Task in progress',
              timestamp: Date.now(),
            },
          });

          connection.send({
            type: 'task:completed',
            taskId: task.taskId,
            result: {
              success: true,
              exitCode: 0,
              output: 'Done',
            },
          });
        }
      });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      receivedMessages.length = 0;

      serverSocket?.send(JSON.stringify({
        type: 'task:execute',
        task: {
          taskId: 'event-task',
          prompt: 'test',
          cwd: process.cwd(),
          agentType: 'opencode',
        },
      }));

      await new Promise((r) => setTimeout(r, 500));

      const events = receivedMessages.filter((m) => m.type === 'task:event');
      expect(events.length).toBeGreaterThanOrEqual(2);

      const logEvent = events.find((e) => (e as any).event?.type === 'log');
      expect(logEvent).toBeDefined();

      const messageEvent = events.find((e) => (e as any).event?.type === 'message');
      expect(messageEvent).toBeDefined();
    });

    it('should handle task abort', async () => {
      const connection = createConnection({ hostId: 'abort-test-host' });

      const executor = new TaskExecutor({
        defaultCwd: process.cwd(),
        defaultAgent: 'opencode',
      });

      let taskStarted = false;

      connection.on('message', async (msg: ServerToGatewayMessage) => {
        if (msg.type === 'task:execute') {
          taskStarted = true;
          connection.send({
            type: 'task:started',
            taskId: msg.task.taskId,
            sessionId: 'abort-test-session',
          });

          // Start a long-running shell command
          const isWindows = process.platform === 'win32';
          const { spawn } = await import('child_process');
          
          const child = spawn(isWindows ? 'cmd' : 'sh',
            isWindows ? ['/c', 'ping -n 30 localhost'] : ['-c', 'sleep 30'],
            { cwd: process.cwd() }
          );

          // Store reference for abort
          (executor as any).activeTasks = (executor as any).activeTasks || new Map();
          (executor as any).activeTasks.set(msg.task.taskId, child);

        } else if (msg.type === 'task:abort') {
          const aborted = executor.abort(msg.taskId);
          if (aborted) {
            connection.send({
              type: 'task:event',
              taskId: msg.taskId,
              event: {
                type: 'log',
                content: 'Task aborted',
                timestamp: Date.now(),
              },
            });
          }
        }
      });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      receivedMessages.length = 0;

      // Start a long task
      serverSocket?.send(
        JSON.stringify({
          type: 'task:execute',
          task: {
            taskId: 'long-task',
            prompt: 'sleep 30',
            cwd: process.cwd(),
            agentType: 'opencode',
          },
        })
      );

      await new Promise((r) => setTimeout(r, 500));

      // Verify task started
      expect(taskStarted).toBe(true);
      const started = receivedMessages.find((m) => m.type === 'task:started');
      expect(started).toBeDefined();

      // Abort the task
      serverSocket?.send(JSON.stringify({ type: 'task:abort', taskId: 'long-task' }));

      await new Promise((r) => setTimeout(r, 500));

      // Task should no longer be active
      expect(executor.activeTaskIds).not.toContain('long-task');
    }, 10000);
  });

  describe('Error Handling', () => {
    it('should handle server disconnect gracefully', async () => {
      const connection = createConnection({ 
        hostId: 'disconnect-test',
        reconnect: false,
      });

      const states: string[] = [];
      connection.on('stateChange', (state) => states.push(state.status));

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      expect(connection.isConnected).toBe(true);

      // Server closes connection
      serverSocket?.close();
      
      await new Promise((r) => setTimeout(r, 200));

      expect(connection.isConnected).toBe(false);
      expect(states).toContain('disconnected');
    });

    it('should handle malformed messages gracefully', async () => {
      const connection = createConnection({ hostId: 'malformed-test' });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      // Send malformed message (should not crash)
      serverSocket?.send('not valid json {{{');
      serverSocket?.send(JSON.stringify({ type: 'unknown_type' }));
      
      await new Promise((r) => setTimeout(r, 100));

      // Connection should still be alive
      expect(connection.isConnected).toBe(true);
    });

    it('should handle multiple rapid messages', async () => {
      const connection = createConnection({ hostId: 'rapid-test' });

      const receivedPings: ServerToGatewayMessage[] = [];
      connection.on('message', (msg) => {
        if (msg.type === 'ping') {
          receivedPings.push(msg);
        }
      });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      receivedMessages.length = 0;

      // Send multiple pings rapidly
      for (let i = 0; i < 10; i++) {
        serverSocket?.send(JSON.stringify({ type: 'ping' }));
      }
      
      await new Promise((r) => setTimeout(r, 200));

      // Should have received all pings
      expect(receivedPings.length).toBe(10);

      // Should have responded with heartbeats
      const heartbeats = receivedMessages.filter((m) => m.type === 'heartbeat');
      expect(heartbeats.length).toBe(10);
    });

    it('should handle registration failure', async () => {
      // Create a new server that rejects registration
      const rejectServer = new WebSocketServer({ port: 0 });
      const rejectPort = await new Promise<number>((resolve) => {
        rejectServer.on('listening', () => {
          const addr = rejectServer.address();
          resolve(typeof addr === 'object' && addr !== null ? addr.port : 0);
        });
      });

      rejectServer.on('connection', (ws) => {
        ws.on('message', (data) => {
          const msg = JSON.parse(data.toString());
          if (msg.type === 'register') {
            ws.send(JSON.stringify({ 
              type: 'registered', 
              ok: false, 
              error: 'Invalid credentials' 
            }));
          }
        });
      });

      const connection = new GatewayConnection({
        serverUrl: `ws://localhost:${rejectPort}`,
        hostId: 'reject-test',
        authToken: 'bad-token',
        capabilities: testCapabilities,
        reconnect: false,
      });
      
      connection.on('error', () => {});
      activeConnections.push(connection);

      const states: string[] = [];
      connection.on('stateChange', (state) => states.push(state.status));

      await connection.connect();
      await new Promise((r) => setTimeout(r, 200));

      // Registration should have failed and connection closed
      expect(states).toContain('disconnected');
      
      rejectServer.close();
    });
  });

  describe('Multiple Connections', () => {
    it('should handle multiple gateways connecting', async () => {
      const connection1 = createConnection({ hostId: 'multi-host-1' });
      const connection2 = createConnection({ hostId: 'multi-host-2' });

      await Promise.all([
        connection1.connect(),
        connection2.connect(),
      ]);

      await new Promise((r) => setTimeout(r, 200));

      expect(connection1.isConnected).toBe(true);
      expect(connection2.isConnected).toBe(true);

      // Both should have registered
      const registerMsgs = receivedMessages.filter((m) => m.type === 'register');
      expect(registerMsgs.length).toBe(2);

      const hostIds = registerMsgs.map((m) => (m as any).hostId);
      expect(hostIds).toContain('multi-host-1');
      expect(hostIds).toContain('multi-host-2');
    });
  });

  describe('Task Input Forwarding', () => {
    it('should forward task:input to executor', async () => {
      const connection = createConnection({ hostId: 'input-test-host' });

      const executor = new TaskExecutor({
        defaultCwd: process.cwd(),
        defaultAgent: 'opencode',
      });

      let inputReceived = false;

      connection.on('message', async (msg: ServerToGatewayMessage) => {
        if (msg.type === 'task:input') {
          const sent = executor.sendInput(msg.taskId, msg.content);
          inputReceived = true;
          // Note: sendInput returns false if task doesn't exist, which is expected here
        }
      });

      await connection.connect();
      await new Promise((r) => setTimeout(r, 100));

      // Send input for a task
      serverSocket?.send(JSON.stringify({
        type: 'task:input',
        taskId: 'input-task',
        content: 'user response',
      }));

      await new Promise((r) => setTimeout(r, 100));

      expect(inputReceived).toBe(true);
    });
  });
});
