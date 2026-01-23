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
    // console.log('Client connected:', socket.id);

    socket.on('disconnect', () => {
      // console.log('Client disconnected:', socket.id);
    });
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
