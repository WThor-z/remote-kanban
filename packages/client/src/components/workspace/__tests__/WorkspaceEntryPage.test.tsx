import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import { WorkspaceEntryPage } from '../WorkspaceEntryPage';
import { useHosts } from '../../../hooks/useHosts';
import { useProjects } from '../../../hooks/useProjects';
import { useWorkspaces } from '../../../hooks/useWorkspaces';

vi.mock('../../../hooks/useHosts', () => ({
  useHosts: vi.fn(),
}));

vi.mock('../../../hooks/useWorkspaces', () => ({
  useWorkspaces: vi.fn(),
}));

vi.mock('../../../hooks/useProjects', () => ({
  useProjects: vi.fn(),
}));

describe('WorkspaceEntryPage', () => {
  const onContinue = vi.fn();
  const deleteWorkspaceMock = vi.fn(async () => true);
  const createWorkspaceMock = vi.fn();
  const discoverWorkspaceProjectsMock = vi.fn();
  const createWorkspaceProjectMock = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    (useHosts as Mock).mockReturnValue({
      hosts: [
        {
          hostId: 'host-1',
          name: 'Host Alpha',
          status: 'online',
          capabilities: {
            name: 'Host Alpha',
            agents: ['opencode'],
            maxConcurrent: 2,
            cwd: '/tmp/alpha',
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
    });

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
      createWorkspace: createWorkspaceMock,
      deleteWorkspace: deleteWorkspaceMock,
      hasWorkspaces: true,
    });

    createWorkspaceMock.mockResolvedValue(null);
    discoverWorkspaceProjectsMock.mockResolvedValue([]);
    createWorkspaceProjectMock.mockResolvedValue(null);

    (useProjects as Mock).mockReturnValue({
      projects: [],
      isLoading: false,
      error: null,
      refresh: vi.fn(async () => {}),
      getProject: vi.fn(),
      getProjectsForGateway: vi.fn(() => []),
      hasProjects: false,
      discoverWorkspaceProjects: discoverWorkspaceProjectsMock,
      createWorkspaceProject: createWorkspaceProjectMock,
    });
  });

  it('requires exact typed workspace name before deleting', async () => {
    deleteWorkspaceMock.mockResolvedValue(true);

    render(
      <WorkspaceEntryPage
        selectedWorkspaceId="ws-1"
        hasStaleStoredWorkspace={false}
        onSelectionChange={vi.fn()}
        onContinue={onContinue}
      />,
    );

    const deleteButton = screen.getByRole('button', { name: /delete workspace/i });
    expect(deleteButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText(/type workspace name to confirm deletion/i), {
      target: { value: 'Workspace Al' },
    });
    expect(deleteButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText(/type workspace name to confirm deletion/i), {
      target: { value: 'Workspace Alpha' },
    });
    expect(deleteButton).not.toBeDisabled();

    fireEvent.click(deleteButton);

    await waitFor(() => {
      expect(deleteWorkspaceMock).toHaveBeenCalledWith('ws-1', { confirmName: 'Workspace Alpha' });
    });
  });

  it('hides project management until workspace is entered', () => {
    render(
      <WorkspaceEntryPage
        selectedWorkspaceId="ws-1"
        hasStaleStoredWorkspace={false}
        onSelectionChange={vi.fn()}
        onContinue={onContinue}
      />,
    );

    expect(screen.queryByText('Workspace Projects')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /discover projects/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /create project/i })).not.toBeInTheDocument();
  });

  it('allows manual host fallback when host list is unavailable', async () => {
    createWorkspaceMock.mockResolvedValue({
      id: 'ws-2',
      name: 'Workspace Beta',
      slug: 'workspace-beta',
      hostId: 'custom-host-42',
      rootPath: '/tmp/workspace-beta',
      defaultProjectId: null,
      createdAt: '2026-01-02T00:00:00Z',
      updatedAt: '2026-01-02T00:00:00Z',
      archivedAt: null,
    });

    (useHosts as Mock).mockReturnValue({
      hosts: [],
      isLoading: false,
      error: 'host discovery unavailable',
      refresh: vi.fn(async () => {}),
      getHostsForAgent: vi.fn(() => []),
      hasAvailableHosts: false,
    });

    render(
      <WorkspaceEntryPage
        selectedWorkspaceId="ws-1"
        hasStaleStoredWorkspace={false}
        onSelectionChange={vi.fn()}
        onContinue={onContinue}
      />,
    );

    fireEvent.change(screen.getByLabelText(/manual host id/i), { target: { value: 'custom-host-42' } });
    fireEvent.change(screen.getByLabelText(/^workspace name$/i), { target: { value: 'Workspace Beta' } });
    fireEvent.change(screen.getByLabelText(/root path/i), { target: { value: '/tmp/workspace-beta' } });
    fireEvent.click(screen.getByRole('button', { name: /create workspace/i }));

    await waitFor(() => {
      expect(createWorkspaceMock).toHaveBeenCalledWith({
        name: 'Workspace Beta',
        rootPath: '/tmp/workspace-beta',
        hostId: 'custom-host-42',
      });
    });
  });

  it('discovers workspace projects and marks registered ones', async () => {
    discoverWorkspaceProjectsMock.mockResolvedValue([
      {
        name: 'repo-a',
        localPath: '/tmp/workspace-alpha/repo-a',
        source: 'discovered',
        registeredProjectId: 'project-7',
      },
      {
        name: 'repo-b',
        localPath: '/tmp/workspace-alpha/repo-b',
        source: 'discovered',
        registeredProjectId: null,
      },
    ]);

    render(
      <WorkspaceEntryPage
        selectedWorkspaceId="ws-1"
        hasStaleStoredWorkspace={false}
        onSelectionChange={vi.fn()}
        onContinue={onContinue}
        allowProjectManagement
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /discover projects/i }));

    await waitFor(() => {
      expect(discoverWorkspaceProjectsMock).toHaveBeenCalledWith('ws-1');
    });

    expect(screen.getByText('repo-a')).toBeInTheDocument();
    expect(screen.getByText('repo-b')).toBeInTheDocument();
    expect(screen.getByText(/registered/i)).toBeInTheDocument();
  });

  it('creates workspace project from management form', async () => {
    createWorkspaceProjectMock.mockResolvedValue({
      id: 'project-1',
      gatewayId: 'host-1',
      workspaceId: 'ws-1',
      name: 'repo-manual',
      localPath: '/tmp/workspace-alpha/repo-manual',
      remoteUrl: null,
      defaultBranch: 'main',
      createdAt: '2026-01-02T00:00:00Z',
      updatedAt: '2026-01-02T00:00:00Z',
    });

    render(
      <WorkspaceEntryPage
        selectedWorkspaceId="ws-1"
        hasStaleStoredWorkspace={false}
        onSelectionChange={vi.fn()}
        onContinue={onContinue}
        allowProjectManagement
      />,
    );

    expect(screen.getByText('/tmp/workspace-alpha')).toBeInTheDocument();
    const autoPathInput = screen.getByLabelText(/project local path \(auto\)/i);
    expect(autoPathInput).toHaveAttribute('readonly');

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
