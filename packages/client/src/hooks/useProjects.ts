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
}

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

  return {
    projects,
    isLoading,
    error,
    refresh: fetchProjects,
    getProject,
    getProjectsForGateway,
    hasProjects,
  };
};
