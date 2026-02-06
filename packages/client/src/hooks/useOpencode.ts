import { useEffect, useState } from 'react';
import { io, Socket } from 'socket.io-client';
import { resolveGatewaySocketUrl } from '../config/endpoints';

let socket: Socket | undefined;
let socketUrl: string | undefined;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

export const __resetOpencodeSocketForTests = () => {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = undefined;
  }

  socket?.disconnect();
  socket = undefined;
  socketUrl = undefined;
};

const resolveSocketUrl = () => resolveGatewaySocketUrl();

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
      // Use WebSocket transport only to avoid CORS issues with polling
      socket = io(url, {
        transports: ['websocket'],
      });
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

  return { isConnected, socket };
};
