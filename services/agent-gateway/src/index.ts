/**
 * Agent Gateway - Entry Point
 * 
 * This service runs on remote hosts and connects to the central server
 * via WebSocket to receive and execute agent tasks.
 */

import { GatewayConnection } from './connection.js';
import { TaskExecutor } from './executor.js';
import type { ServerToGatewayMessage, TaskRequest } from './types.js';

// Configuration from environment variables
const config = {
  serverUrl: process.env.GATEWAY_SERVER_URL || 'ws://localhost:3001',
  hostId: process.env.GATEWAY_HOST_ID || `host-${Date.now()}`,
  authToken: process.env.GATEWAY_AUTH_TOKEN || 'dev-token',
  capabilities: {
    name: process.env.GATEWAY_HOST_NAME || 'Development Host',
    agents: ['opencode'] as ('opencode' | 'claude-code' | 'gemini')[],
    maxConcurrent: parseInt(process.env.GATEWAY_MAX_CONCURRENT || '2'),
    cwd: process.env.GATEWAY_CWD || process.cwd(),
  },
};

// Create connection
const connection = new GatewayConnection({
  serverUrl: config.serverUrl,
  hostId: config.hostId,
  authToken: config.authToken,
  capabilities: config.capabilities,
  reconnect: true,
});

// Create executor
const executor = new TaskExecutor({
  defaultCwd: config.capabilities.cwd,
  defaultAgent: 'opencode',
});

// Forward executor events to server
executor.on('event', ({ taskId, event }) => {
  connection.send({
    type: 'task:event',
    taskId,
    event,
  });
});

// Handle server messages
connection.on('message', async (msg: ServerToGatewayMessage) => {
  switch (msg.type) {
    case 'task:execute':
      await handleTaskExecute(msg.task);
      break;
    case 'task:abort':
      handleTaskAbort(msg.taskId);
      break;
    case 'task:input':
      handleTaskInput(msg.taskId, msg.content);
      break;
  }
});

async function handleTaskExecute(task: TaskRequest): Promise<void> {
  console.log(`[Gateway] Executing task ${task.taskId}`);
  
  connection.send({
    type: 'task:started',
    taskId: task.taskId,
    sessionId: '', // OpenCode session ID, filled later if available
  });

  try {
    const result = await executor.execute(task);
    
    connection.send({
      type: 'task:completed',
      taskId: task.taskId,
      result,
    });
  } catch (err) {
    connection.send({
      type: 'task:failed',
      taskId: task.taskId,
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

function handleTaskAbort(taskId: string): void {
  console.log(`[Gateway] Aborting task ${taskId}`);
  const aborted = executor.abort(taskId);
  if (!aborted) {
    console.warn(`[Gateway] Task ${taskId} not found`);
  }
}

function handleTaskInput(taskId: string, content: string): void {
  console.log(`[Gateway] Sending input to task ${taskId}`);
  executor.sendInput(taskId, content);
}

// Startup
console.log('[Gateway] Starting Agent Gateway...');
console.log(`[Gateway] Server: ${config.serverUrl}`);
console.log(`[Gateway] Host ID: ${config.hostId}`);
console.log(`[Gateway] Capabilities:`, config.capabilities);

connection.connect().catch((err) => {
  console.error('[Gateway] Failed to connect:', err.message);
});

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('[Gateway] Shutting down...');
  connection.disconnect();
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log('[Gateway] Shutting down...');
  connection.disconnect();
  process.exit(0);
});
