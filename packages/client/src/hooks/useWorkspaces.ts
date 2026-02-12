import { useCallback, useEffect, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

export interface Workspace {
  id: string;
  name: string;
  slug: string;
  hostId: string;
  rootPath: string;
  defaultProjectId: string | null;
  createdAt: string;
  updatedAt: string;
  archivedAt: string | null;
}

export interface CreateWorkspaceInput {
  name: string;
  hostId: string;
  rootPath: string;
}

export interface DeleteWorkspaceInput {
  confirmName: string;
}

export interface UseWorkspacesResult {
  workspaces: Workspace[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  createWorkspace: (input: CreateWorkspaceInput) => Promise<Workspace | null>;
  deleteWorkspace: (workspaceId: string, input: DeleteWorkspaceInput) => Promise<boolean>;
  hasWorkspaces: boolean;
}

export const useWorkspaces = (): UseWorkspacesResult => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const fetchWorkspaces = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/workspaces`);
      if (!response.ok) {
        throw new Error(`Failed to fetch workspaces (${response.status})`);
      }

      const data = await response.json();
      const workspaceList = Array.isArray(data) ? data : (data.workspaces || []);
      setWorkspaces(workspaceList.filter((workspace: Workspace) => !workspace.archivedAt));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load workspaces');
      setWorkspaces([]);
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const createWorkspace = useCallback(async (input: CreateWorkspaceInput): Promise<Workspace | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/workspaces`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: input.name,
          hostId: input.hostId,
          rootPath: input.rootPath,
        }),
      });

      if (!response.ok) {
        let message = `Failed to create workspace (${response.status})`;
        try {
          const payload = await response.json();
          if (typeof payload?.error === 'string') {
            message = payload.error;
          }
        } catch {
          // Ignore non-JSON responses.
        }
        throw new Error(message);
      }

      const workspace = (await response.json()) as Workspace;
      setWorkspaces((prev) => [workspace, ...prev.filter((existing) => existing.id !== workspace.id)]);
      return workspace;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create workspace');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const deleteWorkspace = useCallback(async (workspaceId: string, input: DeleteWorkspaceInput): Promise<boolean> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/workspaces/${workspaceId}`, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ confirmName: input.confirmName }),
      });

      if (!response.ok) {
        let message = `Failed to delete workspace (${response.status})`;
        try {
          const payload = await response.json();
          if (typeof payload?.error === 'string') {
            message = payload.error;
          }
        } catch {
          // Ignore non-JSON responses.
        }
        throw new Error(message);
      }

      setWorkspaces((prev) => prev.filter((workspace) => workspace.id !== workspaceId));
      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete workspace');
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  useEffect(() => {
    fetchWorkspaces();
    const interval = setInterval(fetchWorkspaces, 60000);
    return () => clearInterval(interval);
  }, [fetchWorkspaces]);

  return {
    workspaces,
    isLoading,
    error,
    refresh: fetchWorkspaces,
    createWorkspace,
    deleteWorkspace,
    hasWorkspaces: workspaces.length > 0,
  };
};
