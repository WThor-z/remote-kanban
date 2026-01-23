import { useEffect, useState, useCallback } from 'react';
import { io, Socket } from 'socket.io-client';

let socket: Socket | undefined;
let socketUrl: string | undefined;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

const resolveSocketUrl = () => {
  const envUrl = typeof import.meta !== 'undefined'
    ? import.meta.env?.VITE_OPENCODE_SOCKET_URL
    : undefined;

  if (envUrl) {
    return envUrl;
  }

  if (typeof process !== 'undefined' && process.env?.OPENCODE_SOCKET_URL) {
    return process.env.OPENCODE_SOCKET_URL;
  }

  return 'http://localhost:3000';
};

const scheduleReconnect = (delayMs: number) => {
  if (!socket || reconnectTimer) {
    return;
  }

  reconnectTimer = setTimeout(() => {
    reconnectTimer = undefined;
    socket?.connect();
  }, delayMs);
};

export const useOpencode = () => {
  const url = resolveSocketUrl();
  const [isConnected, setIsConnected] = useState(socket?.connected || false);

  useEffect(() => {
    if (!socket || socketUrl !== url) {
      socket?.disconnect();
      socket = io(url);
      socketUrl = url;
    }

    function onConnect() {
      setIsConnected(true);
    }

    function onDisconnect() {
      setIsConnected(false);
      scheduleReconnect(500);
    }

    socket.on('connect', onConnect);
    socket.on('disconnect', onDisconnect);
    
    // Check initial connection status
    setTimeout(() => {
      if (socket?.connected) {
        setIsConnected(true);
      }
    }, 0);

    return () => {
      socket?.off('connect', onConnect);
      socket?.off('disconnect', onDisconnect);
    };
  }, [url]);

  const write = useCallback((data: string) => {
    socket?.emit('input', data);
  }, []);

  const onData = useCallback((callback: (data: string) => void) => {
    if (!socket) return () => {};

    const handler = (data: string) => callback(data);
    socket.on('output', handler);

    return () => {
      socket?.off('output', handler);
    };
  }, []);

  return { isConnected, socket, write, onData };
};
