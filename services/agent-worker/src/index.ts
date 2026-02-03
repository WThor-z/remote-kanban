import Fastify from 'fastify';
import cors from '@fastify/cors';
import { z } from 'zod';
import { execa } from 'execa';
import { existsSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

const fastify = Fastify({ logger: true });

await fastify.register(cors);

const ExecuteSchema = z.object({
  taskId: z.string(),
  prompt: z.string(),
  cwd: z.string(),
  env: z.record(z.string(), z.string()).optional(),
  agentType: z.string().optional(),
  model: z.string().optional(), // e.g. "openai/gpt-4o-mini" or "google/gemini-3-flash"
});

// Store active processes
const activeProcesses = new Map<string, any>();

fastify.post('/execute', async (request, reply) => {
  let sendEvent: ((event: any) => void) | null = null;

  try {
    const { taskId, prompt, cwd, env, agentType, model } = ExecuteSchema.parse(request.body);
    const outputBuffer: string[] = [];

    // Set headers for SSE
    reply.raw.setHeader('Content-Type', 'text/event-stream');
    reply.raw.setHeader('Cache-Control', 'no-cache');
    reply.raw.setHeader('Connection', 'keep-alive');
    reply.raw.setHeader('Access-Control-Allow-Origin', '*');
    reply.hijack();
    reply.raw.flushHeaders?.();

    sendEvent = (event: any) => {
      reply.raw.write(`data: ${JSON.stringify(event)}\n\n`);
    };

    const pushOutput = (line: string) => {
      outputBuffer.push(line);
      if (outputBuffer.length > 5) {
        outputBuffer.shift();
      }
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

    const resolvedAgentType = (agentType || 'opencode').toLowerCase();
    const command = resolveCommand(resolvedAgentType);
    const args = resolveArgs(resolvedAgentType, prompt, model);

    // Use execa to spawn process
    // We use 'all' to capture both stdout and stderr
    let outputSeen = false;
    let outputTimer: ReturnType<typeof setTimeout> | null = setTimeout(() => {
      if (!outputSeen) {
        sendEvent?.({
          type: 'log',
          content: '[worker] No output from agent yet. Check opencode auth/model config if it persists.',
        });
      }
    }, 15000);

    let silenceTimeout: ReturnType<typeof setTimeout> | null = setTimeout(() => {
      if (!outputSeen) {
        sendEvent?.({
          type: 'status',
          status: 'failed',
          error: 'No output from agent within 60s. Verify opencode auth/model configuration.',
        });
        try {
          subprocess.kill();
        } catch {
          // ignore
        }
        reply.raw.end();
      }
    }, 60000);

    sendEvent({ type: 'log', content: `[worker] running: ${command} ${args.join(' ')}` });

    const heartbeatTimer = setInterval(() => {
      reply.raw.write(': heartbeat\n\n');
    }, 10000);

    const childEnv: Record<string, string> = {
      ...(process.env as Record<string, string>),
      ...(env || {}),
      CI: '1',
      TERM: 'dumb',
      FORCE_COLOR: '0',
      // Ensure no colors for easier parsing
      NO_COLOR: '1',
      // Clear proxy settings to avoid Privoxy errors
      HTTP_PROXY: '',
      HTTPS_PROXY: '',
      http_proxy: '',
      https_proxy: '',
      ALL_PROXY: '',
      all_proxy: '',
    };

    // Some environments (notably the OpenCode Desktop app) export OPENCODE_SERVER_USERNAME/
    // OPENCODE_SERVER_PASSWORD which enables Basic Auth on the local opencode server.
    // The CLI `run` flow can fail in that mode (often surfacing as "Session not found").
    // We explicitly drop these vars for headless execution.
    if (resolvedAgentType === 'opencode') {
      delete childEnv.OPENCODE_SERVER_USERNAME;
      delete childEnv.OPENCODE_SERVER_PASSWORD;
    }

    const subprocess = execa(command, args, {
      cwd,
      env: childEnv,
      all: true,
      reject: false, // Don't throw on non-zero exit
    });

    activeProcesses.set(taskId, subprocess);

    if (subprocess.all) {
      subprocess.all.on('data', (chunk) => {
        const line = chunk.toString().trim();
        if (line) {
          outputSeen = true;
          if (outputTimer) {
            clearTimeout(outputTimer);
            outputTimer = null;
          }
          if (silenceTimeout) {
            clearTimeout(silenceTimeout);
            silenceTimeout = null;
          }
          pushOutput(line);
          sendEvent?.({ type: 'log', content: line });
        }
      });
    }

    const { exitCode } = await subprocess;
    
    activeProcesses.delete(taskId);
    clearInterval(heartbeatTimer);
    if (outputTimer) {
      clearTimeout(outputTimer);
      outputTimer = null;
    }
    if (silenceTimeout) {
      clearTimeout(silenceTimeout);
      silenceTimeout = null;
    }

    if (exitCode === 0) {
      sendEvent({ type: 'status', status: 'completed' });
    } else {
      const lastOutput = outputBuffer[outputBuffer.length - 1];
      const error = lastOutput || `Process exited with code ${exitCode ?? 'unknown'}`;
      sendEvent({ type: 'status', status: 'failed', exitCode, error });
    }
    
    reply.raw.end();

  } catch (error) {
    request.log.error(error);
    const message = error instanceof Error ? error.message : String(error);
    if (sendEvent) {
      sendEvent({ type: 'status', status: 'failed', error: message });
      reply.raw.end();
      return;
    }
    if (!reply.raw.headersSent) {
      reply.code(400).send({ error: message });
    } else {
      // If headers sent, we can only end stream
      reply.raw.end();
    }
  }
});

function resolveCommand(agentType: string): string {
  const isWindows = process.platform === 'win32';
  switch (agentType) {
    case 'claude-code':
    case 'claudecode':
      return isWindows ? 'claude.cmd' : 'claude';
    case 'gemini-cli':
    case 'gemini':
      return isWindows ? 'gemini.cmd' : 'gemini';
    case 'codex':
      return isWindows ? 'codex.cmd' : 'codex';
    case 'opencode':
    default: {
      if (isWindows) {
        // Prefer the Desktop app's bundled CLI if available
        // It uses the same auth/config as the GUI and is more reliable
        const desktopCli = join(
          homedir(),
          'AppData',
          'Local',
          'OpenCode',
          'opencode-cli.exe'
        );
        if (existsSync(desktopCli)) {
          return desktopCli;
        }
      }
      return isWindows ? 'opencode.cmd' : 'opencode';
    }
  }
}

function resolveArgs(agentType: string, prompt: string, model?: string): string[] {
  if (agentType === 'opencode') {
    // --port 0 tells opencode to start its own temporary server on a random port
    // This avoids "Session not found" errors when no opencode serve is running
    // or when the existing server has auth enabled
    const args = [
      '--print-logs',
      '--log-level',
      'INFO',
      'run',
      '--format',
      'json',
      '--port',
      '0',
    ];
    // Optionally specify model (useful if default provider's token is expired)
    if (model) {
      args.push('-m', model);
    }
    args.push(prompt);
    return args;
  }

  // Default to passing prompt as positional to avoid unsupported flags
  return [prompt];
}

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
