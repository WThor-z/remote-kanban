import { useCallback, useEffect, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

export interface Workspace {
  id: string;
  name: string;
  slug: string;
  rootPath: string;
  defaultProjectId: string | null;
  createdAt: string;
  updatedAt: string;
  archivedAt: string | null;
}

export interface UseWorkspacesResult {
  workspaces: Workspace[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  hasWorkspaces: boolean;
}

export const useWorkspaces = (): UseWorkspacesResult => {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [isLoading, setIsLoading] = useState(false);
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
    hasWorkspaces: workspaces.length > 0,
  };
};
