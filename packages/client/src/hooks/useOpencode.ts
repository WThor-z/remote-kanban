import { useEffect, useState, useCallback } from 'react';
import { io, Socket } from 'socket.io-client';

let socket: Socket | undefined;

export const useOpencode = () => {
  const [isConnected, setIsConnected] = useState(socket?.connected || false);

  useEffect(() => {
    if (!socket) {
      socket = io('http://localhost:3000');
    }

    function onConnect() {
      setIsConnected(true);
    }

    function onDisconnect() {
      setIsConnected(false);
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
  }, []);

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
