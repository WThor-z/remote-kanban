import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import App from '../App';
import { useTaskApi } from '../hooks/useTaskApi';

const { fetchTasksMock, createTaskModalRenderSpy } = vi.hoisted(() => ({
  fetchTasksMock: vi.fn(async () => {}),
  createTaskModalRenderSpy: vi.fn(),
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
    selectTask: vi.fn(),
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

vi.mock('../hooks/useWorkspaces', () => ({
  useWorkspaces: () => ({
    workspaces: [
      {
        id: 'ws-1',
        name: 'Workspace Alpha',
        slug: 'workspace-alpha',
        rootPath: '/tmp/workspace-alpha',
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
        defaultProjectId: null,
        createdAt: '2026-02-09T00:00:00Z',
        updatedAt: '2026-02-09T00:00:00Z',
        archivedAt: null,
      },
    ],
    isLoading: false,
    error: null,
    refresh: vi.fn(async () => {}),
    hasWorkspaces: true,
  }),
}));

vi.mock('../components/kanban/KanbanBoard', () => ({
  KanbanBoard: () => <div data-testid="kanban-board" />, 
}));

vi.mock('../components/task', () => ({
  TaskDetailPanel: () => null,
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

describe('App workspace scope handoff', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();

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

  it('passes selected workspace scope into create modal and scoped fetch', async () => {
    render(<App />);

    const scopeButton = screen.getByRole('button', { name: /workspace scope/i });
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
});
