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
  
  // Track shells per socket to avoid double-kill
  const shellMap = new Map<string, { shell: ReturnType<PtyManager['spawn']>; subscription: { dispose: () => void }; killed: boolean }>();

  const safeKillShell = (socketId: string) => {
    const entry = shellMap.get(socketId);
    if (!entry || entry.killed) {
      return;
    }
    
    entry.killed = true;
    try {
      entry.subscription.dispose();
      entry.shell.kill();
    } catch (err) {
      // Ignore "already killed" errors
      if (!(err instanceof Error && err.message.includes('already'))) {
        console.error('Error cleanup shell:', err);
      }
    }
    shellMap.delete(socketId);
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
    // Kill any existing shell for previous connections (single-user mode)
    shellMap.forEach((_, id) => safeKillShell(id));
    
    // Spawn a shell for this client
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
        const entry = shellMap.get(socket.id);
        if (entry && !entry.killed) {
          try {
            entry.shell.write(data);
          } catch (err) {
            console.error('Error writing to shell:', err);
          }
        }
      });

      // Handle outgoing data from shell
      const subscription = shell.onData((data: string) => {
        socket.emit('output', data);
      });

      shellMap.set(socket.id, { shell, subscription, killed: false });

      socket.on('disconnect', () => {
        console.log('Client disconnected:', socket.id);
        safeKillShell(socket.id);
      });
    } catch (err) {
      console.error('Failed to spawn shell:', err);
      socket.emit('output', '\r\n\x1b[31mError: Failed to spawn shell process.\x1b[0m\r\n');
    }
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
