import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useOpencode } from '../useOpencode';
import { io } from 'socket.io-client';

vi.mock('socket.io-client', () => {
  const on = vi.fn();
  const off = vi.fn();
  const connect = vi.fn();
  const disconnect = vi.fn();
  const socket = { on, off, connect, disconnect, active: true };
  return {
    io: vi.fn(() => socket),
  };
});

describe('useOpencode', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should set isConnected to true on connect event', () => {
    const { result } = renderHook(() => useOpencode());

    const socketMock = io();
    const onMock = socketMock.on as unknown as ReturnType<typeof vi.fn>;
    
    // Expect 'connect' listener to be registered
    expect(onMock).toHaveBeenCalledWith('connect', expect.any(Function));

    const calls = onMock.mock.calls;
    const connectCall = calls.find((call) => call[0] === 'connect');
    
    if (!connectCall) {
      throw new Error('connect listener not registered');
    }

    const connectCallback = connectCall[1];

    act(() => {
      connectCallback();
    });

    expect(result.current.isConnected).toBe(true);
  });

  it('uses environment URL when provided', () => {
    process.env.OPENCODE_SOCKET_URL = 'http://localhost:4321';

    renderHook(() => useOpencode());

    expect(io).toHaveBeenCalledWith('http://localhost:4321');

    delete process.env.OPENCODE_SOCKET_URL;
  });
});
