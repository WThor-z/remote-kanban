import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import { CreateTaskModal } from '../CreateTaskModal';
import { useModels } from '../../../hooks/useModels';
import { useProjects } from '../../../hooks/useProjects';
import { WorkspaceScopeProvider } from '../../../context/workspaceScopeContext';

const { useWorkspacesMock } = vi.hoisted(() => ({
  useWorkspacesMock: vi.fn(),
}));

vi.mock('../../../hooks/useModels', () => ({
  useModels: vi.fn(),
}));

vi.mock('../../../hooks/useProjects', () => ({
  useProjects: vi.fn(),
}));

vi.mock('../../../hooks/useWorkspaces', () => ({
  useWorkspaces: useWorkspacesMock,
}));

describe('CreateTaskModal workspace filtering', () => {
  const onClose = vi.fn();
  const onCreate = vi.fn(async () => true);

  beforeEach(() => {
    vi.clearAllMocks();

    (useModels as Mock).mockReturnValue({
      modelOptions: [],
      isLoading: false,
      fetchModels: vi.fn(),
      clearModels: vi.fn(),
    });

    (useProjects as Mock).mockReturnValue({
      projects: [
        {
          id: 'project-1',
          gatewayId: 'gateway-1',
          workspaceId: 'ws-1',
          name: 'Project Alpha',
          localPath: '/tmp/project-alpha',
          remoteUrl: null,
          defaultBranch: 'main',
          createdAt: '2026-02-09T00:00:00Z',
          updatedAt: '2026-02-09T00:00:00Z',
        },
      ],
      isLoading: false,
      error: null,
      refresh: vi.fn(async () => {}),
      getProject: vi.fn(),
      getProjectsForGateway: vi.fn(),
      hasProjects: true,
    });

    useWorkspacesMock.mockReturnValue({
      workspaces: [
        {
          id: 'ws-1',
          name: 'Workspace Alpha',
          slug: 'workspace-alpha',
          hostId: 'host-1',
          rootPath: '/tmp/workspace-alpha',
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
    });
  });

  it('shows selected workspace scope selector', () => {
    render(<CreateTaskModal isOpen onClose={onClose} onCreate={onCreate} defaultWorkspaceId="ws-1" />);

    expect(screen.getByLabelText('Selected workspace scope')).toBeInTheDocument();
    expect(screen.getByText('Workspace Alpha')).toBeInTheDocument();
  });

  it('always passes selected workspace id into useProjects filter', async () => {
    render(<CreateTaskModal isOpen onClose={onClose} onCreate={onCreate} defaultWorkspaceId="ws-1" />);

    await waitFor(() => {
      expect(useProjects).toHaveBeenLastCalledWith({ workspaceId: 'ws-1' });
    });
  });

  it('does not render all workspace fallback option', () => {
    render(<CreateTaskModal isOpen onClose={onClose} onCreate={onCreate} defaultWorkspaceId="ws-1" />);

    expect(screen.queryByRole('button', { name: /all workspaces/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /show projects from all workspaces/i })).not.toBeInTheDocument();
  });

  it('keeps workspace filter scoped even when project list is empty', async () => {
    (useProjects as Mock).mockReturnValue({
      projects: [],
      isLoading: false,
      error: null,
      refresh: vi.fn(async () => {}),
      getProject: vi.fn(),
      getProjectsForGateway: vi.fn(),
      hasProjects: false,
    });

    render(<CreateTaskModal isOpen onClose={onClose} onCreate={onCreate} defaultWorkspaceId="ws-1" />);

    await waitFor(() => {
      expect(useProjects).toHaveBeenLastCalledWith({ workspaceId: 'ws-1' });
    });
  });

  it('falls back to workspace scope context when defaultWorkspaceId is absent', async () => {
    render(
      <WorkspaceScopeProvider value={{ activeWorkspaceId: 'ws-1', setActiveWorkspaceId: vi.fn() }}>
        <CreateTaskModal isOpen onClose={onClose} onCreate={onCreate} />
      </WorkspaceScopeProvider>,
    );

    await waitFor(() => {
      expect(useProjects).toHaveBeenLastCalledWith({ workspaceId: 'ws-1' });
    });
  });

  it('keeps create action disabled until project is selected', () => {
    render(<CreateTaskModal isOpen onClose={onClose} onCreate={onCreate} defaultWorkspaceId="ws-1" />);

    fireEvent.change(screen.getByLabelText(/directive title/i), { target: { value: 'Task A' } });

    expect(screen.getByRole('button', { name: /create capsule/i })).toBeDisabled();
  });
});
