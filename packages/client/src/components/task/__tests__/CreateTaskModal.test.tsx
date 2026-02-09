import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import { CreateTaskModal } from '../CreateTaskModal';
import { useModels } from '../../../hooks/useModels';
import { useProjects } from '../../../hooks/useProjects';

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
        {
          id: 'project-2',
          gatewayId: 'gateway-2',
          workspaceId: 'ws-2',
          name: 'Project Beta',
          localPath: '/tmp/project-beta',
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
    });
  });

  it('shows workspace scope selector', () => {
    render(
      <CreateTaskModal
        isOpen
        onClose={onClose}
        onCreate={onCreate}
      />,
    );

    expect(screen.getByText('Workspace Scope')).toBeInTheDocument();
  });

  it('passes selected workspace id into useProjects filter', async () => {
    render(
      <CreateTaskModal
        isOpen
        onClose={onClose}
        onCreate={onCreate}
      />,
    );

    const openWorkspaceSelector = screen.getByRole('button', {
      name: /select workspace scope/i,
    });

    fireEvent.click(openWorkspaceSelector);
    fireEvent.click(screen.getByRole('button', { name: /workspace beta/i }));

    await waitFor(() => {
      expect(useProjects).toHaveBeenLastCalledWith({ workspaceId: 'ws-2' });
    });
  });
});
