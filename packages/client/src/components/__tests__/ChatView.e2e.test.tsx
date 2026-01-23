import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from 'vitest';
import type { AddressInfo } from 'net';
import { startServer } from '../../../../server/src/server';
import { ChatView } from '../ChatView';
import { useOpencode } from '../../hooks/useOpencode';

type OnDataHandler = (data: string) => void;

let onDataHandler: OnDataHandler | null = null;

vi.mock('@opencode-vibe/pty-manager', () => {
  const shell = {
    onData: vi.fn((handler: OnDataHandler) => {
      onDataHandler = handler;
      return { dispose: vi.fn() };
    }),
    write: vi.fn((data: string) => {
      if (onDataHandler) {
        onDataHandler(data);
      }
    }),
    kill: vi.fn(),
  };

  return {
    PtyManager: class {
      spawn = vi.fn(() => shell);
    },
  };
});

describe('ChatView E2E', () => {
  let stopServer: () => void;
  let serverPort: number;

  beforeAll(async () => {
    const app = startServer(0);
    stopServer = app.stop;

    await new Promise<void>((resolve) => {
      app.httpServer.on('listening', () => {
        serverPort = (app.httpServer.address() as AddressInfo).port;
        resolve();
      });
    });

    process.env.OPENCODE_SOCKET_URL = `http://localhost:${serverPort}`;
  });

  afterAll(() => {
    delete process.env.OPENCODE_SOCKET_URL;
    if (stopServer) {
      stopServer();
    }
  });

  beforeEach(() => {
    onDataHandler = null;
    vi.clearAllMocks();
  });

  it('routes server output into filtered chat view', async () => {
    const { result, unmount } = renderHook(() => useOpencode());
    render(<ChatView />);

    await waitFor(() => expect(result.current.isConnected).toBe(true));

    act(() => {
      result.current.write('STATUS: Ready');
      result.current.write('[ERROR] Boom');
    });

    await waitFor(() => expect(screen.getByText('Ready')).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText('Boom')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: 'Log' }));
    fireEvent.click(screen.getByRole('button', { name: 'Error' }));

    expect(screen.getByText('Boom')).toBeInTheDocument();
    expect(screen.queryByText('Ready')).not.toBeInTheDocument();

    act(() => {
      result.current.socket?.disconnect();
    });

    unmount();
  });
});
