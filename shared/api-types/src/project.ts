/**
 * Project API Types
 * 
 * Shared type definitions for project-related API endpoints.
 */

/** A project containing tasks */
export interface Project {
  id: string;
  name: string;
  path: string;           // Local file system path
  description?: string;
  defaultBranch: string;
  createdAt: string;
  updatedAt: string;
}

/** Request body for creating a new project */
export interface CreateProjectRequest {
  name: string;
  path: string;
  description?: string;
  defaultBranch?: string;
}

/** Response for project list endpoint */
export interface ProjectListResponse {
  projects: Project[];
  total: number;
}
