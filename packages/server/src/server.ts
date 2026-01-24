import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import cors from 'cors';
import { PtyManager } from '@opencode-vibe/pty-manager';
import { KanbanStore } from './kanban';
import type { KanbanTaskStatus } from '@opencode-vibe/protocol';

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

  // Initialize PtyManager - intended for use in future socket events
  const ptyManager = new PtyManager();
  let activeSession: {
    socket: Parameters<Parameters<typeof io.on>[1]>[0];
    shell: ReturnType<PtyManager['spawn']>;
    subscription: { dispose: () => void };
  } | null = null;

  const cleanupSession = () => {
    if (!activeSession) {
      return;
    }

    try {
      activeSession.shell.kill();
      activeSession.subscription.dispose();
      activeSession.socket.disconnect(true);
    } catch (err) {
      console.error('Error cleanup shell:', err);
    } finally {
      activeSession = null;
    }
  };

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

    // === PTY Session ===
    if (activeSession) {
      socket.emit('output', '\r\n\x1b[33mNotice: Previous session closed.\x1b[0m\r\n');
      cleanupSession();
    }
    
    // Spawn a shell for this client
    // For MVP, we spawn a new shell for each connection, 
    // or we could use a session ID to reconnect to existing PTYs.
    // Let's keep it simple: One shell per socket.
    try {
      const shellCmd = process.platform === 'win32' ? 'powershell.exe' : 'bash';
      const shell = ptyManager.spawn(shellCmd, [], {
        cols: 80,
        rows: 24,
        cwd: process.cwd(),
        env: process.env as Record<string, string>
      });

      // Handle incoming data from client
      socket.on('input', (data: string) => {
        try {
          shell.write(data);
        } catch (err) {
          console.error('Error writing to shell:', err);
        }
      });

      // Handle outgoing data from shell
      const subscription = shell.onData((data: string) => {
        socket.emit('output', data);
      });

      activeSession = { socket, shell, subscription };

      socket.on('disconnect', () => {
        console.log('Client disconnected:', socket.id);
        try {
          shell.kill();
          subscription.dispose();
        } catch (err) {
          console.error('Error cleanup shell:', err);
        }

        if (activeSession?.socket.id === socket.id) {
          activeSession = null;
        }
      });
    } catch (err) {
      console.error('Failed to spawn shell:', err);
      socket.emit('output', '\r\n\x1b[31mError: Failed to spawn shell process.\x1b[0m\r\n');
    }
  });

  httpServer.listen(port);

  return {
    httpServer,
    io,
    stop: () => {
        io.close();
        httpServer.close();
    }
  };
}
