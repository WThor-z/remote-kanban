import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import App from '../App';
import { useTaskApi } from '../hooks/useTaskApi';
import { CONSOLE_LANGUAGE_STORAGE_KEY } from '../i18n/consoleLanguage';
import { WORKSPACE_SCOPE_STORAGE_KEY } from '../utils/workspaceScopeStorage';

const { fetchTasksMock, createTaskModalRenderSpy, selectTaskMock, confirmMock } = vi.hoisted(() => ({
  fetchTasksMock: vi.fn(async () => {}),
  createTaskModalRenderSpy: vi.fn(),
  selectTaskMock: vi.fn(),
  confirmMock: vi.fn(() => true),
}));

vi.mock('../hooks/useOpencode', () => ({
  useOpencode: () => ({
    isConnected: true,
    socket: null,
  }),
}));

vi.mock('../hooks/useKanban', () => ({
  useKanban: () => ({
    board: {
      tasks: {
        'task-1': {
          id: 'task-1',
          title: 'Task One',
          status: 'todo',
          createdAt: Date.now(),
        },
      },
      columns: {
        todo: { id: 'todo', title: 'To Do', taskIds: ['task-1'] },
        doing: { id: 'doing', title: 'Doing', taskIds: [] },
        done: { id: 'done', title: 'Done', taskIds: [] },
      },
      columnOrder: ['todo', 'doing', 'done'],
    },
    moveTask: vi.fn(),
    deleteTask: vi.fn(),
    requestSync: vi.fn(),
  }),
}));

vi.mock('../hooks/useTaskSession', () => ({
  useTaskSession: () => ({
    history: [],
    status: null,
    isLoading: false,
    error: null,
    selectTask: selectTaskMock,
    stopTask: vi.fn(),
    sendMessage: vi.fn(),
  }),
}));

vi.mock('../hooks/useTaskApi', () => ({
  useTaskApi: vi.fn(),
}));

vi.mock('../hooks/useTaskExecutor', () => ({
  useTaskExecutor: () => ({
    currentSession: null,
    isExecuting: false,
    error: null,
    startExecution: vi.fn(async () => null),
    stopExecution: vi.fn(async () => undefined),
    getExecutionStatus: vi.fn(async () => null),
    cleanupWorktree: vi.fn(async () => false),
    sendInput: vi.fn(async () => false),
  }),
}));

vi.mock('../hooks/useGatewayInfo', () => ({
  useGatewayInfo: () => ({
    info: {
      version: 'test',
      workerUrl: 'ws://localhost:3000',
      dataDir: '/tmp/data',
    },
    isLoading: false,
    error: null,
    refresh: vi.fn(),
  }),
}));

vi.mock('../hooks/useHosts', () => ({
  useHosts: () => ({
    hosts: [
      {
        hostId: 'host-1',
        name: 'Host Alpha',
        status: 'online',
        capabilities: {
          name: 'Alpha',
          agents: ['opencode'],
          maxConcurrent: 1,
          cwd: '/tmp',
        },
        activeTasks: [],
        lastHeartbeat: Date.now(),
        connectedAt: Date.now(),
      },
    ],
    isLoading: false,
    error: null,
    refresh: vi.fn(async () => {}),
    getHostsForAgent: vi.fn(() => []),
    hasAvailableHosts: true,
  }),
}));

vi.mock('../hooks/useWorkspaces', () => ({
  useWorkspaces: () => ({
    workspaces: [
      {
        id: 'ws-1',
        name: 'Workspace Alpha',
        slug: 'workspace-alpha',
        rootPath: '/tmp/workspace-alpha',
        hostId: 'host-1',
        defaultProjectId: null,
        createdAt: '2026-02-09T00:00:00Z',
        updatedAt: '2026-02-09T00:00:00Z',
        archivedAt: null,
      },
      {
        id: 'ws-2',
        name: 'Workspace Beta',
        slug: 'workspace-beta',
        rootPath: '/tmp/workspace-beta',
        hostId: 'host-1',
        defaultProjectId: null,
        createdAt: '2026-02-09T00:00:00Z',
        updatedAt: '2026-02-09T00:00:00Z',
        archivedAt: null,
      },
    ],
    isLoading: false,
    error: null,
    refresh: vi.fn(async () => {}),
    createWorkspace: vi.fn(async () => null),
    deleteWorkspace: vi.fn(async () => false),
    hasWorkspaces: true,
  }),
}));

vi.mock('../components/kanban/KanbanBoard', () => ({
  KanbanBoard: ({ onTaskClick }: { onTaskClick: (task: { id: string; title: string; status: string; createdAt: number }) => void }) => (
    <div data-testid="kanban-board">
      <button
        type="button"
        onClick={() => onTaskClick({ id: 'task-1', title: 'Task One', status: 'todo', createdAt: Date.now() })}
      >
        Open Task
      </button>
    </div>
  ),
}));

vi.mock('../components/task', () => ({
  TaskDetailPanel: () => <div data-testid="task-detail-panel">detail</div>,
  CreateTaskModal: (props: {
    isOpen: boolean;
    defaultWorkspaceId?: string;
  }) => {
    createTaskModalRenderSpy(props);
    if (!props.isOpen) {
      return null;
    }

    return (
      <div data-testid="create-task-modal-scope">
        {props.defaultWorkspaceId || 'all'}
      </div>
    );
  },
}));

vi.mock('../components/memory/MemoryPage', () => ({
  MemoryPage: () => <div data-testid="memory-page" />,
}));

describe('App workspace scope handoff', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    vi.stubGlobal('confirm', confirmMock);

    (useTaskApi as Mock).mockImplementation(() => ({
      tasks: [
        {
          id: 'task-1',
          projectId: 'project-1',
          workspaceId: 'ws-2',
          title: 'Task One',
          description: null,
          status: 'todo',
          priority: 'medium',
          agentType: 'opencode',
          baseBranch: 'main',
          model: null,
          createdAt: '2026-02-09T00:00:00Z',
          updatedAt: '2026-02-09T00:00:00Z',
        },
      ],
      isLoading: false,
      error: null,
      fetchTasks: fetchTasksMock,
      createTask: vi.fn(async () => null),
      getTask: vi.fn(async () => null),
      updateTask: vi.fn(async () => null),
      deleteTask: vi.fn(async () => false),
      clearError: vi.fn(),
    }));
  });

  it('gates app behind workspace entry until workspace confirmation', () => {
    render(<App />);

    expect(screen.getByText('Workspace Entry')).toBeInTheDocument();
    expect(screen.queryByTestId('kanban-board')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /memory/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /create project/i })).not.toBeInTheDocument();
  });

  it('supports toggling workspace entry language to Chinese', () => {
    render(<App />);

    expect(screen.getByText('Workspace Entry')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /switch language/i }));

    expect(screen.getByText('工作区入口')).toBeInTheDocument();
  });

  it('loads stored Chinese language preference on startup', () => {
    window.localStorage.setItem(CONSOLE_LANGUAGE_STORAGE_KEY, 'zh');

    render(<App />);

    expect(screen.getByText('工作区入口')).toBeInTheDocument();
  });

  it('applies Chinese copy across app shell after language toggle', async () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText(/select workspace/i), { target: { value: 'ws-1' } });
    fireEvent.click(screen.getByRole('button', { name: /continue to workspace/i }));
    await screen.findByTestId('kanban-board');

    fireEvent.click(screen.getByRole('button', { name: /switch language/i }));

    expect(screen.getByRole('button', { name: /看板/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /记忆/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /项目管理/i })).toBeInTheDocument();
    expect(screen.getByText('OpenCode Vibe 指挥中枢')).toBeInTheDocument();
  });

  it('allows project management only after entering a workspace', async () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText(/select workspace/i), { target: { value: 'ws-1' } });
    fireEvent.click(screen.getByRole('button', { name: /continue to workspace/i }));
    await screen.findByTestId('kanban-board');

    fireEvent.click(screen.getByRole('button', { name: /manage projects/i }));

    expect(screen.queryByText('Workspace Entry')).not.toBeInTheDocument();
    expect(screen.getByTestId('workspace-project-management-page')).toBeInTheDocument();
    expect(screen.getByText('Workspace Projects')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create project/i })).toBeInTheDocument();
  });

  it('preselects stored workspace but requires explicit continue', async () => {
    window.localStorage.setItem(WORKSPACE_SCOPE_STORAGE_KEY, 'ws-2');

    render(<App />);

    expect(screen.getByText('Workspace Entry')).toBeInTheDocument();
    expect(screen.getByText('Workspace Beta')).toBeInTheDocument();
    expect(screen.queryByTestId('kanban-board')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /continue to workspace/i }));

    await waitFor(() => {
      expect(fetchTasksMock).toHaveBeenCalledWith({ workspaceId: 'ws-2' });
    });
    expect(screen.getByTestId('kanban-board')).toBeInTheDocument();
  });

  it('keeps entry page when stored workspace is stale', () => {
    window.localStorage.setItem(WORKSPACE_SCOPE_STORAGE_KEY, 'stale-workspace');

    render(<App />);

    expect(screen.getByText(/Previously selected workspace is no longer available/i)).toBeInTheDocument();
    expect(screen.getByText('Workspace Entry')).toBeInTheDocument();
    expect(screen.queryByTestId('kanban-board')).not.toBeInTheDocument();
  });

  it('passes selected workspace scope into create modal and scoped fetch', async () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText(/select workspace/i), { target: { value: 'ws-1' } });
    fireEvent.click(screen.getByRole('button', { name: /continue to workspace/i }));

    const scopeButton = await screen.findByRole('button', { name: /workspace scope/i });
    fireEvent.click(scopeButton);
    fireEvent.click(screen.getByRole('button', { name: /workspace beta/i }));

    await waitFor(() => {
      expect(fetchTasksMock).toHaveBeenCalledWith({ workspaceId: 'ws-2' });
    });

    fireEvent.click(screen.getByRole('button', { name: /new task capsule/i }));

    expect(screen.getByTestId('create-task-modal-scope')).toHaveTextContent('ws-2');
    expect(createTaskModalRenderSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        isOpen: true,
        defaultWorkspaceId: 'ws-2',
      }),
    );
  });

  it('asks for workspace switch confirmation and clears selected task context on confirm', async () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText(/select workspace/i), { target: { value: 'ws-1' } });
    fireEvent.click(screen.getByRole('button', { name: /continue to workspace/i }));
    fireEvent.click(await screen.findByRole('button', { name: /open task/i }));
    expect(screen.getByTestId('task-detail-panel')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /workspace scope/i }));
    fireEvent.click(screen.getByRole('button', { name: /workspace beta/i }));

    expect(confirmMock).toHaveBeenCalledWith(
      expect.stringContaining('Running tasks will continue in the background'),
    );

    await waitFor(() => {
      expect(fetchTasksMock).toHaveBeenCalledWith({ workspaceId: 'ws-2' });
    });
    expect(selectTaskMock).toHaveBeenCalledWith(null);
    expect(screen.queryByTestId('task-detail-panel')).not.toBeInTheDocument();
  });

  it('does not switch workspace when confirmation is canceled', async () => {
    confirmMock.mockReturnValue(false);
    render(<App />);

    fireEvent.change(screen.getByLabelText(/select workspace/i), { target: { value: 'ws-1' } });
    fireEvent.click(screen.getByRole('button', { name: /continue to workspace/i }));
    await screen.findByTestId('kanban-board');

    fireEvent.click(screen.getByRole('button', { name: /workspace scope/i }));
    fireEvent.click(screen.getByRole('button', { name: /workspace beta/i }));

    await waitFor(() => {
      expect(fetchTasksMock).not.toHaveBeenCalledWith({ workspaceId: 'ws-2' });
    });
  });
});
