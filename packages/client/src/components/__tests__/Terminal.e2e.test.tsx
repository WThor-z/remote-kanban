import { render, waitFor, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeAll, afterAll, beforeEach, vi } from 'vitest';
import type { AddressInfo } from 'net';
import { startServer } from '../../../../server/src/server';
import { InputBar } from '../InputBar';
import { Terminal } from '../Terminal';

type OnDataHandler = (data: string) => void;

const mockTermWrite = vi.fn();
const mockTermOnData = vi.fn();
const mockTermOpen = vi.fn();
const mockTermLoadAddon = vi.fn();

let serverSocket: { emit: (event: string, data: string) => void } | null = null;
let lastInput: string | null = null;

vi.mock('xterm', () => {
  return {
    Terminal: class {
      open = mockTermOpen;
      loadAddon = mockTermLoadAddon;
      write = mockTermWrite;
      onData = mockTermOnData;
      dispose = vi.fn();
      options = {};
    },
  };
});

vi.mock('xterm-addon-fit', () => {
  return {
    FitAddon: class {
      fit = vi.fn();
      dispose = vi.fn();
    },
  };
});

vi.mock('@opencode-vibe/pty-manager', () => {
  const shell = {
    onData: vi.fn((_handler: OnDataHandler) => ({ dispose: vi.fn() })),
    write: vi.fn(),
    kill: vi.fn(),
  };

  return {
    PtyManager: class {
      spawn = vi.fn(() => shell);
    },
  };
});

// Mock useKanban to prevent interference with InputBar
vi.mock('../../hooks/useKanban', () => ({
  useKanban: vi.fn(() => ({
    board: { tasks: {}, columns: {}, columnOrder: [] },
    isLoading: false,
    createTask: vi.fn(),
    moveTask: vi.fn(),
    deleteTask: vi.fn(),
  })),
}));

describe('Terminal E2E', () => {
  let stopServer: () => void;
  let serverPort: number;

  beforeAll(async () => {
    const app = startServer(0);
    stopServer = app.stop;

    app.io.on('connection', (socket) => {
      serverSocket = socket;
      socket.on('input', (data: string) => {
        lastInput = data;
      });
    });

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
    lastInput = null;
    serverSocket = null;
    vi.clearAllMocks();
  });

  it('propagates input to server output and terminal rendering', async () => {
    render(
      <div>
        <InputBar />
        <Terminal />
      </div>
    );

    await waitFor(() => expect(mockTermOpen).toHaveBeenCalled());
    await waitFor(() => expect(serverSocket).not.toBeNull());

    const input = document.querySelector('[data-testid="input-bar-input"]') as HTMLInputElement;
    const form = document.querySelector('[data-testid="input-bar-form"]') as HTMLFormElement;

    fireEvent.change(input, { target: { value: 'echo hello' } });
    fireEvent.submit(form);

    await waitFor(() => expect(lastInput).toBe('echo hello\r'));

    serverSocket?.emit('output', 'echo hello\r');

    await waitFor(() => expect(mockTermWrite).toHaveBeenCalledWith('echo hello\r'));
  });
});
