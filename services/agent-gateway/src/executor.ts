import { spawn, ChildProcess } from 'child_process';
import { EventEmitter } from 'events';
import type { TaskRequest, GatewayAgentEvent, TaskResult } from './types.js';

export interface ExecutorOptions {
  defaultCwd: string;
  defaultAgent: string;
}

export class TaskExecutor extends EventEmitter {
  private activeTasks = new Map<string, ChildProcess>();

  constructor(private options: ExecutorOptions) {
    super();
  }

  async execute(task: TaskRequest): Promise<TaskResult> {
    const startTime = Date.now();
    const cwd = task.cwd || this.options.defaultCwd;
    const agentType = task.agentType || this.options.defaultAgent;

    return new Promise((resolve, reject) => {
      const command = this.resolveCommand(agentType);
      const args = this.resolveArgs(agentType, task.prompt, task.model);

      this.emitEvent(task.taskId, {
        type: 'log',
        content: `[executor] Starting: ${command} ${args.join(' ')}`,
        timestamp: Date.now(),
      });

      const child = spawn(command, args, {
        cwd,
        env: {
          ...process.env,
          ...(task.env || {}),
          CI: '1',
          NO_COLOR: '1',
          TERM: 'dumb',
        },
        shell: true,
      });

      this.activeTasks.set(task.taskId, child);

      let output = '';
      let errorOutput = '';

      child.stdout?.on('data', (chunk) => {
        const text = chunk.toString();
        output += text;
        this.parseAndEmitEvents(task.taskId, text);
      });

      child.stderr?.on('data', (chunk) => {
        const text = chunk.toString();
        errorOutput += text;
        this.emitEvent(task.taskId, {
          type: 'error',
          content: text,
          timestamp: Date.now(),
        });
      });

      child.on('error', (err) => {
        this.activeTasks.delete(task.taskId);
        reject(err);
      });

      child.on('exit', (code) => {
        this.activeTasks.delete(task.taskId);
        const duration = Date.now() - startTime;

        resolve({
          success: code === 0,
          exitCode: code ?? undefined,
          output: output.slice(-2000), // Keep last 2000 chars
          duration,
        });
      });

      // Timeout handling
      if (task.timeout) {
        setTimeout(() => {
          if (this.activeTasks.has(task.taskId)) {
            child.kill();
            this.activeTasks.delete(task.taskId);
            reject(new Error(`Task timeout after ${task.timeout}ms`));
          }
        }, task.timeout);
      }
    });
  }

  abort(taskId: string): boolean {
    const child = this.activeTasks.get(taskId);
    if (child) {
      child.kill();
      this.activeTasks.delete(taskId);
      return true;
    }
    return false;
  }

  sendInput(taskId: string, content: string): boolean {
    const child = this.activeTasks.get(taskId);
    if (child?.stdin) {
      child.stdin.write(content + '\n');
      return true;
    }
    return false;
  }

  private emitEvent(taskId: string, event: GatewayAgentEvent): void {
    this.emit('event', { taskId, event });
  }

  private parseAndEmitEvents(taskId: string, text: string): void {
    const lines = text.split('\n').filter(Boolean);
    
    for (const line of lines) {
      try {
        const parsed = JSON.parse(line);
        // OpenCode JSON format output
        if (parsed.type) {
          this.emitEvent(taskId, {
            type: this.mapEventType(parsed.type),
            content: parsed.content || parsed.message,
            data: parsed,
            timestamp: Date.now(),
          });
          continue;
        }
      } catch {
        // Not JSON, treat as plain log
      }

      // Plain log
      this.emitEvent(taskId, {
        type: 'log',
        content: line,
        timestamp: Date.now(),
      });
    }
  }

  private mapEventType(type: string): GatewayAgentEvent['type'] {
    const mapping: Record<string, GatewayAgentEvent['type']> = {
      'thinking': 'thinking',
      'tool_call': 'tool_call',
      'tool_result': 'tool_result',
      'file_change': 'file_change',
      'message': 'message',
      'error': 'error',
    };
    return mapping[type] || 'log';
  }

  private resolveCommand(agentType: string): string {
    const isWindows = process.platform === 'win32';
    switch (agentType) {
      case 'claude-code':
      case 'claudecode':
        return isWindows ? 'claude.cmd' : 'claude';
      case 'gemini-cli':
      case 'gemini':
        return isWindows ? 'gemini.cmd' : 'gemini';
      case 'opencode':
      default:
        return isWindows ? 'opencode.cmd' : 'opencode';
    }
  }

  private resolveArgs(agentType: string, prompt: string, model?: string): string[] {
    if (agentType === 'opencode') {
      const args = [
        '--print-logs',
        '--log-level', 'INFO',
        'run',
        '--format', 'json',
        '--port', '0',
      ];
      if (model) {
        args.push('-m', model);
      }
      args.push(prompt);
      return args;
    }
    return [prompt];
  }

  get activeTaskCount(): number {
    return this.activeTasks.size;
  }

  get activeTaskIds(): string[] {
    return Array.from(this.activeTasks.keys());
  }
}
