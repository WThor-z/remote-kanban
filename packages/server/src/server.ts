import express from 'express';
import { createServer } from 'http';
import { Server } from 'socket.io';
import cors from 'cors';
import { PtyManager } from '@opencode-vibe/pty-manager';

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

  // Initialize PtyManager - intended for use in future socket events
  const ptyManager = new PtyManager();

  io.on('connection', (socket) => {
    console.log('Client connected:', socket.id);
    
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

      socket.on('disconnect', () => {
        console.log('Client disconnected:', socket.id);
        try {
          shell.kill();
          subscription.dispose();
        } catch (err) {
          console.error('Error cleanup shell:', err);
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
