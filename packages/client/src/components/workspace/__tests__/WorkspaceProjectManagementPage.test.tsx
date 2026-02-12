import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import { WorkspaceProjectManagementPage } from '../WorkspaceProjectManagementPage';
import { useProjects } from '../../../hooks/useProjects';
import { useWorkspaces } from '../../../hooks/useWorkspaces';

vi.mock('../../../hooks/useWorkspaces', () => ({
  useWorkspaces: vi.fn(),
}));

vi.mock('../../../hooks/useProjects', () => ({
  useProjects: vi.fn(),
}));

describe('WorkspaceProjectManagementPage', () => {
  const discoverWorkspaceProjectsMock = vi.fn();
  const createWorkspaceProjectMock = vi.fn();
  const refreshMock = vi.fn(async () => {});

  beforeEach(() => {
    vi.clearAllMocks();

    discoverWorkspaceProjectsMock.mockResolvedValue([]);
    createWorkspaceProjectMock.mockResolvedValue(null);

    (useWorkspaces as Mock).mockReturnValue({
      workspaces: [
        {
          id: 'ws-1',
          name: 'Workspace Alpha',
          slug: 'workspace-alpha',
          hostId: 'host-1',
          rootPath: '/tmp/workspace-alpha',
          defaultProjectId: null,
          createdAt: '2026-01-01T00:00:00Z',
          updatedAt: '2026-01-01T00:00:00Z',
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

    (useProjects as Mock).mockReturnValue({
      projects: [],
      isLoading: false,
      error: null,
      refresh: refreshMock,
      getProject: vi.fn(),
      getProjectsForGateway: vi.fn(() => []),
      hasProjects: false,
      discoverWorkspaceProjects: discoverWorkspaceProjectsMock,
      createWorkspaceProject: createWorkspaceProjectMock,
    });
  });

  it('renders discovered repositories in dedicated list container', () => {
    render(<WorkspaceProjectManagementPage workspaceId="ws-1" />);

    const list = screen.getByTestId('workspace-project-list');
    expect(list).toBeInTheDocument();
    expect(list).not.toHaveClass('dropdown-panel');
    expect(screen.getByText('/tmp/workspace-alpha')).toBeInTheDocument();
    const autoPathInput = screen.getByLabelText(/project local path \(auto\)/i);
    expect(autoPathInput).toHaveAttribute('readonly');
  });

  it('creates workspace project from management form', async () => {
    createWorkspaceProjectMock.mockResolvedValue({
      id: 'project-1',
      gatewayId: 'host-1',
      workspaceId: 'ws-1',
      name: 'repo-manual',
      localPath: '/tmp/workspace-alpha/repo-manual',
      remoteUrl: null,
      defaultBranch: 'develop',
      createdAt: '2026-01-02T00:00:00Z',
      updatedAt: '2026-01-02T00:00:00Z',
    });

    render(<WorkspaceProjectManagementPage workspaceId="ws-1" />);

    fireEvent.change(screen.getByLabelText(/project name/i), { target: { value: 'repo-manual' } });
    fireEvent.change(screen.getByLabelText(/default branch/i), { target: { value: 'develop' } });
    fireEvent.change(screen.getByLabelText(/remote url/i), { target: { value: 'https://example.com/repo.git' } });

    fireEvent.click(screen.getByRole('button', { name: /create project/i }));

    await waitFor(() => {
      expect(createWorkspaceProjectMock).toHaveBeenCalledWith('ws-1', {
        name: 'repo-manual',
        localPath: '/tmp/workspace-alpha/repo-manual',
        defaultBranch: 'develop',
        remoteUrl: 'https://example.com/repo.git',
      });
    });
  });
});
