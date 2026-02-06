/**
 * Agent Gateway - Entry Point
 * 
 * This service runs on remote hosts and connects to the central server
 * via WebSocket to receive and execute agent tasks.
 */

import { GatewayConnection } from './connection.js';
import { TaskExecutor } from './executor.js';
import type { ServerToGatewayMessage, TaskRequest } from './types.js';

const parseAllowedProjectRoots = (): string[] => {
  const raw = process.env.GATEWAY_ALLOWED_PROJECT_ROOTS;
  if (!raw) {
    return [];
  }
  return raw
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
};

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
  allowedProjectRoots: parseAllowedProjectRoots(),
};

// Create connection
const connection = new GatewayConnection({
  serverUrl: config.serverUrl,
  hostId: config.hostId,
  authToken: config.authToken,
  capabilities: config.capabilities,
  reconnect: true,
});

// Create executor (using SDK instead of CLI)
const executor = new TaskExecutor({
  defaultCwd: config.capabilities.cwd,
  defaultAgent: 'opencode',
  serverPort: parseInt(process.env.OPENCODE_PORT || '0'), // 0 = 随机端口
  allowedRoots: config.allowedProjectRoots,
});

// Forward executor events to server
executor.on('event', ({ taskId, event }) => {
  // Log locally for debugging
  console.log(`[executor] ${event.type}: ${event.content?.substring(0, 200) || ''}`);
  
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
    case 'models:request':
      await handleModelsRequest(msg.requestId);
      break;
  }
});

async function handleTaskExecute(task: TaskRequest): Promise<void> {
  console.log(`[Gateway] Executing task ${task.taskId}`);
  console.log(`[Gateway] Prompt: ${task.prompt}`);
  console.log(`[Gateway] CWD: ${task.cwd}`);
  console.log(`[Gateway] Agent: ${task.agentType}`);
  console.log(`[Gateway] Model: ${task.model || '(not specified)'}`);
  
  connection.send({
    type: 'task:started',
    taskId: task.taskId,
    sessionId: '', // OpenCode session ID, filled later if available
  });

  try {
    const result = await executor.execute(task);

    if (!result.success) {
      const errorMessage = result.output || 'Task execution rejected';
      console.error(`[Gateway] Task ${task.taskId} rejected: ${errorMessage}`);
      connection.send({
        type: 'task:failed',
        taskId: task.taskId,
        error: errorMessage,
        details: {
          code: 'CWD_NOT_ALLOWED',
          cwd: task.cwd,
        },
      });
      return;
    }

    console.log(`[Gateway] Task ${task.taskId} completed with exit code: ${result.exitCode}`);
    
    connection.send({
      type: 'task:completed',
      taskId: task.taskId,
      result,
    });
  } catch (err) {
    console.error(`[Gateway] Task ${task.taskId} failed:`, err);
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

async function handleModelsRequest(requestId: string): Promise<void> {
  console.log(`[Gateway] Fetching models for request ${requestId}`);
  try {
    const providers = await executor.getAvailableModels();
    console.log(`[Gateway] Got ${providers.length} providers with models:`, 
      providers.map(p => `${p.name}(${p.models.length})`).join(', '));
    connection.send({
      type: 'models:response',
      requestId,
      providers,
    });
    console.log(`[Gateway] Sent ${providers.length} providers for request ${requestId}`);
  } catch (err) {
    console.error(`[Gateway] Failed to fetch models:`, err);
    // Send error message to help with debugging
    console.error(`[Gateway] Error details:`, err instanceof Error ? err.stack : String(err));
    connection.send({
      type: 'models:response',
      requestId,
      providers: [],
    });
  }
}

// Startup
console.log('[Gateway] Starting Agent Gateway (SDK mode)...');
console.log(`[Gateway] Server: ${config.serverUrl}`);
console.log(`[Gateway] Host ID: ${config.hostId}`);
console.log(`[Gateway] Capabilities:`, config.capabilities);
if (config.allowedProjectRoots.length > 0) {
  console.log(`[Gateway] Allowed project roots: ${config.allowedProjectRoots.join(', ')}`);
}

connection.connect().catch((err) => {
  console.error('[Gateway] Failed to connect:', err.message);
});

// Graceful shutdown
process.on('SIGINT', async () => {
  console.log('[Gateway] Shutting down...');
  await executor.shutdown();
  connection.disconnect();
  process.exit(0);
});

process.on('SIGTERM', async () => {
  console.log('[Gateway] Shutting down...');
  await executor.shutdown();
  connection.disconnect();
  process.exit(0);
});
