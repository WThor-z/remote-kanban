import Fastify from 'fastify';
import cors from '@fastify/cors';
import { z } from 'zod';
import { execa } from 'execa';
import { join } from 'path';

const fastify = Fastify({ logger: true });

await fastify.register(cors);

const ExecuteSchema = z.object({
  taskId: z.string(),
  prompt: z.string(),
  cwd: z.string(),
  env: z.record(z.string(), z.string()).optional(),
  agentType: z.string().optional(),
});

// Store active processes
const activeProcesses = new Map<string, any>();

fastify.post('/execute', async (request, reply) => {
  try {
    const { taskId, prompt, cwd, env, agentType } = ExecuteSchema.parse(request.body);
    
    // Set headers for SSE
    reply.raw.setHeader('Content-Type', 'text/event-stream');
    reply.raw.setHeader('Cache-Control', 'no-cache');
    reply.raw.setHeader('Connection', 'keep-alive');
    reply.raw.setHeader('Access-Control-Allow-Origin', '*');

    const sendEvent = (event: any) => {
      reply.raw.write(`data: ${JSON.stringify(event)}\n\n`);
    };

    sendEvent({ type: 'status', status: 'starting' });

    // Mock execution for testing
    if (prompt.includes('mock-test')) {
      sendEvent({ type: 'log', content: 'Mocking execution...' });
      await new Promise(resolve => setTimeout(resolve, 500));
      sendEvent({ type: 'log', content: 'Step 1: Analyzing request' });
      await new Promise(resolve => setTimeout(resolve, 500));
      sendEvent({ type: 'log', content: 'Thinking: I should generate a response.' });
      await new Promise(resolve => setTimeout(resolve, 500));
      sendEvent({ type: 'log', content: 'Step 2: Performing action' });
      await new Promise(resolve => setTimeout(resolve, 500));
      sendEvent({ type: 'log', content: 'Step 3: Verifying result' });
      sendEvent({ type: 'status', status: 'completed' });
      reply.raw.end();
      return;
    }

    // Determine command based on agentType
    // For now defaulting to 'opencode'
    // On Windows, execa handles .cmd extension automatically usually, but we can be explicit
    const command = process.platform === 'win32' ? 'opencode.cmd' : 'opencode';
    
    // Use execa to spawn process
    // We use 'all' to capture both stdout and stderr
    const subprocess = execa(command, ['--non-interactive', prompt], {
      cwd,
      env: {
        ...process.env,
        ...env,
        // Ensure no colors for easier parsing
        NO_COLOR: '1',
        // Clear proxy settings to avoid Privoxy errors
        HTTP_PROXY: '',
        HTTPS_PROXY: '',
        http_proxy: '',
        https_proxy: '',
        ALL_PROXY: '',
        all_proxy: '',
      },
      all: true,
      reject: false, // Don't throw on non-zero exit
    });

    activeProcesses.set(taskId, subprocess);

    if (subprocess.all) {
      subprocess.all.on('data', (chunk) => {
        const line = chunk.toString().trim();
        if (line) {
          sendEvent({ type: 'log', content: line });
        }
      });
    }

    const { exitCode } = await subprocess;
    
    activeProcesses.delete(taskId);

    if (exitCode === 0) {
      sendEvent({ type: 'status', status: 'completed' });
    } else {
      sendEvent({ type: 'status', status: 'failed', exitCode });
    }
    
    reply.raw.end();

  } catch (error) {
    request.log.error(error);
    if (!reply.raw.headersSent) {
      reply.code(400).send({ error: (error as Error).message });
    } else {
      // If headers sent, we can only end stream
      reply.raw.end();
    }
  }
});

fastify.post('/stop', async (request, reply) => {
  const StopSchema = z.object({ taskId: z.string() });
  const { taskId } = StopSchema.parse(request.body);

  const subprocess = activeProcesses.get(taskId);
  if (subprocess) {
    subprocess.kill();
    activeProcesses.delete(taskId);
    return { success: true };
  }
  
  return { success: false, message: 'Task not found' };
});

fastify.post('/input', async (request, reply) => {
  const InputSchema = z.object({ 
    taskId: z.string(),
    content: z.string() 
  });
  const { taskId, content } = InputSchema.parse(request.body);

  const subprocess = activeProcesses.get(taskId);
  if (subprocess && subprocess.stdin) {
    try {
      subprocess.stdin.write(content + '\n');
      return { success: true };
    } catch (e) {
      request.log.error(e);
      return { success: false, message: 'Failed to write to stdin' };
    }
  }
  
  return { success: false, message: 'Task not found or stdin unavailable' };
});

const start = async () => {
  try {
    await fastify.listen({ port: 4000, host: '0.0.0.0' });
    console.log('Agent Worker running on http://localhost:4000');
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
};

start();
