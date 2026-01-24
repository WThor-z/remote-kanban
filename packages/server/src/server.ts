import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import cors from 'cors';
import * as path from 'path';
import { KanbanStore } from './kanban';
import { AgentExecutor, AgentOutputParser } from './agent';
import { TaskSessionManager } from './task';
import { 
  createAgentSession, 
  AGENT_PRESETS,
  type KanbanTaskStatus, 
  type AgentType,
  type ChatMessage,
  type AgentSessionStatus,
} from '@opencode-vibe/protocol';

export async function startServer(port: number = 3000) {
  const app = express();
  app.use(cors());

  const httpServer = createServer(app);
  const io = new Server(httpServer, {
    cors: {
      origin: '*',
      methods: ['GET', 'POST']
    }
  });

  // Initialize KanbanStore (in-memory for now, KanbanManager for file persistence)
  const kanbanStore = new KanbanStore();

  // Broadcast kanban state to all clients when it changes
  kanbanStore.subscribe((state) => {
    io.emit('kanban:sync', state);
  });

  // Initialize AgentExecutor and OutputParser
  const agentExecutor = new AgentExecutor();
  const agentParser = new AgentOutputParser();

  // Initialize TaskSessionManager
  const tasksDir = path.join(process.cwd(), '.opencode', 'tasks');
  const taskSessionManager = new TaskSessionManager({
    tasksDir,
    kanbanStore,
    agentExecutor,
  });
  await taskSessionManager.initialize();

  // TaskSessionManager event forwarding
  taskSessionManager.on('status', (payload) => {
    const { taskId, status } = payload as { taskId: string; status: AgentSessionStatus };
    io.emit('task:status', { taskId, status });
  });

  taskSessionManager.on('message', (payload) => {
    const { taskId, message } = payload as { taskId: string; message: ChatMessage };
    io.emit('task:message', { taskId, message });
  });

  taskSessionManager.on('error', (payload) => {
    const { taskId, error } = payload as { taskId: string; error: string };
    io.emit('task:error', { taskId, error });
  });

  // Agent event forwarding - output events
  agentExecutor.onOutput((event) => {
    io.emit('agent:output', event);
    
    // Parse output for task detection
    const parseResults = agentParser.parseChunk(event.data);
    for (const result of parseResults) {
      if (result.taskDetected) {
        io.emit('agent:task-detected', {
          sessionId: event.sessionId,
          action: result.taskDetected.action,
          taskTitle: result.taskDetected.taskTitle,
          timestamp: Date.now(),
        });
      }
    }
  });

  // Agent event forwarding - status events
  agentExecutor.onStatus((event) => {
    io.emit('agent:status', event);
  });

  io.on('connection', (socket) => {
    console.log('Client connected:', socket.id);

    // === Kanban Events ===
    socket.on('kanban:request-sync', () => {
      socket.emit('kanban:sync', kanbanStore.getState());
    });

    socket.on('kanban:create', (payload: { title: string; description?: string }) => {
      try {
        kanbanStore.createTask(payload.title, payload.description);
      } catch (err) {
        socket.emit('kanban:error', { message: (err as Error).message });
      }
    });

    socket.on('kanban:move', (payload: { taskId: string; targetStatus: KanbanTaskStatus; targetIndex?: number }) => {
      try {
        kanbanStore.moveTask(payload.taskId, payload.targetStatus, payload.targetIndex);
      } catch (err) {
        socket.emit('kanban:error', { message: (err as Error).message });
      }
    });

    socket.on('kanban:delete', (payload: { taskId: string }) => {
      try {
        kanbanStore.deleteTask(payload.taskId);
      } catch (err) {
        socket.emit('kanban:error', { message: (err as Error).message });
      }
    });

    // === Agent Events ===
    socket.on('agent:start', async (payload: { agentType: AgentType; prompt: string; taskId?: string }) => {
      try {
        const { agentType, prompt, taskId } = payload;
        const preset = AGENT_PRESETS[agentType];
        
        if (!preset) {
          socket.emit('agent:error', { message: `Unknown agent type: ${agentType}` });
          return;
        }

        const session = createAgentSession(agentType, prompt, taskId);
        const config = { ...preset, cwd: process.cwd() };
        
        // Start is now async - emit session immediately, then start
        socket.emit('agent:session', session);
        
        // Start the agent (async - will emit events via agentExecutor event handlers)
        await agentExecutor.start(session, config);
      } catch (err) {
        socket.emit('agent:error', { message: (err as Error).message });
      }
    });

    socket.on('agent:stop', async (payload: { sessionId: string }) => {
      try {
        await agentExecutor.stop(payload.sessionId);
        socket.emit('agent:session', agentExecutor.getSession(payload.sessionId));
      } catch (err) {
        socket.emit('agent:error', { sessionId: payload.sessionId, message: (err as Error).message });
      }
    });

    socket.on('agent:input', (payload: { sessionId: string; data: string }) => {
      try {
        agentExecutor.write(payload.sessionId, payload.data);
      } catch (err) {
        socket.emit('agent:error', { sessionId: payload.sessionId, message: (err as Error).message });
      }
    });

    socket.on('agent:list', () => {
      socket.emit('agent:sessions', agentExecutor.getAllSessions());
    });

    // === Task Session Events (Task-Agent Integration) ===
    socket.on('task:execute', async (payload: { taskId: string }) => {
      try {
        await taskSessionManager.executeTask(payload.taskId);
      } catch (err) {
        socket.emit('task:error', { taskId: payload.taskId, error: (err as Error).message });
      }
    });

    socket.on('task:stop', async (payload: { taskId: string }) => {
      try {
        await taskSessionManager.stopTask(payload.taskId);
      } catch (err) {
        socket.emit('task:error', { taskId: payload.taskId, error: (err as Error).message });
      }
    });

    socket.on('task:message', async (payload: { taskId: string; content: string }) => {
      try {
        await taskSessionManager.sendMessage(payload.taskId, payload.content);
      } catch (err) {
        socket.emit('task:error', { taskId: payload.taskId, error: (err as Error).message });
      }
    });

    socket.on('task:history', async (payload: { taskId: string }) => {
      try {
        let history = await taskSessionManager.getHistory(payload.taskId);
        
        // 如果没有历史，为新任务创建一个初始的空历史
        if (!history) {
          const state = kanbanStore.getState();
          const task = state.tasks[payload.taskId];
          if (task) {
            history = {
              taskId: payload.taskId,
              sessionId: '',
              title: task.title,
              description: task.description || task.title,
              messages: [],
              status: 'idle',
              createdAt: task.createdAt,
            };
          }
        }
        
        if (history) {
          socket.emit('task:history', history);
        } else {
          socket.emit('task:error', { taskId: payload.taskId, error: 'Task not found' });
        }
      } catch (err) {
        socket.emit('task:error', { taskId: payload.taskId, error: (err as Error).message });
      }
    });

    socket.on('disconnect', () => {
      console.log('Client disconnected:', socket.id);
    });
  });

  httpServer.listen(port, () => {
    console.log(`Server listening on http://localhost:${port}`);
  });

  return {
    httpServer,
    io,
    stop: () => {
        io.close();
        httpServer.close();
    }
  };
}
