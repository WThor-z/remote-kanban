import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import cors from 'cors';
import { KanbanStore } from './kanban';
import { AgentExecutor, AgentOutputParser } from './agent';
import { 
  createAgentSession, 
  AGENT_PRESETS,
  type KanbanTaskStatus, 
  type AgentType
} from '@opencode-vibe/protocol';

export function startServer(port: number = 3000) {
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
