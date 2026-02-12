/**
 * useProjects Hook
 * 
 * Fetches and manages the list of registered projects.
 * Projects are automatically registered when a Gateway connects.
 */

import { useCallback, useEffect, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

/** Git status for a task/project */
export type GitStatus = 
  | 'none'
  | 'worktree_created'
  | 'committed'
  | 'merged'
  | 'pushed'
  | 'conflict'
  | 'failed';

/** Merge strategy options */
export type MergeStrategy = 'auto' | 'manual';

/** Conflict resolution strategy */
export type ConflictStrategy = 'ai_resolve' | 'keep_branch';

/** Project information from the API */
export interface Project {
  id: string;
  gatewayId: string;
  workspaceId: string;
  name: string;
  localPath: string;
  remoteUrl: string | null;
  defaultBranch: string;
  createdAt: string;
  updatedAt: string;
}

export interface UseProjectsOptions {
  workspaceId?: string;
}

export interface UseProjectsResult {
  /** List of registered projects */
  projects: Project[];
  /** Whether projects are being loaded */
  isLoading: boolean;
  /** Error message if fetch failed */
  error: string | null;
  /** Refresh the projects list */
  refresh: () => Promise<void>;
  /** Get a project by ID */
  getProject: (id: string) => Project | undefined;
  /** Get projects for a specific gateway */
  getProjectsForGateway: (gatewayId: string) => Project[];
  /** Check if any project is available */
  hasProjects: boolean;
  /** Discover git repositories under a workspace root */
  discoverWorkspaceProjects: (workspaceId: string) => Promise<DiscoveredWorkspaceProject[]>;
  /** Register a project under the selected workspace */
  createWorkspaceProject: (workspaceId: string, input: CreateWorkspaceProjectInput) => Promise<Project | null>;
}

export interface CreateWorkspaceProjectInput {
  name: string;
  localPath: string;
  remoteUrl?: string;
  defaultBranch?: string;
  worktreeDir?: string;
}

export interface DiscoveredWorkspaceProject {
  name: string;
  localPath: string;
  source: string;
  registeredProjectId: string | null;
}

interface WorkspaceProjectPayload {
  id: string;
  gatewayId: string;
  workspaceId: string;
  name: string;
  localPath: string;
  remoteUrl: string | null;
  defaultBranch: string;
  createdAt?: string;
  updatedAt?: string;
}

const parseErrorMessage = async (response: Response, fallback: string): Promise<string> => {
  try {
    const payload = await response.json();
    if (typeof payload?.error === 'string') {
      return payload.error;
    }
  } catch {
    // Ignore non-JSON errors.
  }
  return fallback;
};

export const useProjects = ({ workspaceId }: UseProjectsOptions = {}): UseProjectsResult => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const fetchProjects = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
      if (workspaceId) {
        params.set('workspaceId', workspaceId);
      }
      const query = params.toString();
      const response = await fetch(`${baseUrl}/api/projects${query ? `?${query}` : ''}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch projects (${response.status})`);
      }
      const data = await response.json();
      // API returns { projects: [...] } or just [...]
      const projectList = Array.isArray(data) ? data : (data.projects || []);
      setProjects(projectList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load projects');
      setProjects([]);
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl, workspaceId]);

  useEffect(() => {
    fetchProjects();
    // Refresh every 60 seconds
    const interval = setInterval(fetchProjects, 60000);
    return () => clearInterval(interval);
  }, [fetchProjects]);

  const getProject = useCallback((id: string): Project | undefined => {
    return projects.find(p => p.id === id);
  }, [projects]);

  const getProjectsForGateway = useCallback((gatewayId: string): Project[] => {
    return projects.filter(p => p.gatewayId === gatewayId);
  }, [projects]);

  const hasProjects = projects.length > 0;

  const discoverWorkspaceProjects = useCallback(async (workspaceId: string): Promise<DiscoveredWorkspaceProject[]> => {
    if (!workspaceId) {
      setError('Workspace is required to discover projects');
      return [];
    }

    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/workspaces/${workspaceId}/projects/discover`);
      if (!response.ok) {
        const message = await parseErrorMessage(response, `Failed to discover projects (${response.status})`);
        throw new Error(message);
      }

      const payload = await response.json();
      return Array.isArray(payload) ? payload : [];
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to discover projects');
      return [];
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const createWorkspaceProject = useCallback(async (
    workspaceId: string,
    input: CreateWorkspaceProjectInput,
  ): Promise<Project | null> => {
    if (!workspaceId) {
      setError('Workspace is required to create a project');
      return null;
    }

    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/workspaces/${workspaceId}/projects`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: input.name,
          localPath: input.localPath,
          remoteUrl: input.remoteUrl,
          defaultBranch: input.defaultBranch,
          worktreeDir: input.worktreeDir,
        }),
      });

      if (!response.ok) {
        const message = await parseErrorMessage(response, `Failed to create project (${response.status})`);
        throw new Error(message);
      }

      const payload = (await response.json()) as WorkspaceProjectPayload;
      const timestamp = new Date().toISOString();
      const created: Project = {
        id: payload.id,
        gatewayId: payload.gatewayId,
        workspaceId: payload.workspaceId,
        name: payload.name,
        localPath: payload.localPath,
        remoteUrl: payload.remoteUrl,
        defaultBranch: payload.defaultBranch,
        createdAt: payload.createdAt ?? timestamp,
        updatedAt: payload.updatedAt ?? timestamp,
      };
      setProjects((prev) => [created, ...prev.filter((project) => project.id !== created.id)]);
      return created;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create project');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  return {
    projects,
    isLoading,
    error,
    refresh: fetchProjects,
    getProject,
    getProjectsForGateway,
    hasProjects,
    discoverWorkspaceProjects,
    createWorkspaceProject,
  };
};
