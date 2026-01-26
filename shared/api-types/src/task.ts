/**
 * Task API Types
 * 
 * Shared type definitions for task-related API endpoints.
 * These types are used by both the frontend and backend.
 */

/** Task status in the kanban board */
export type TaskStatus = 'todo' | 'in_progress' | 'in_review' | 'done';

/** Task priority level */
export type TaskPriority = 'low' | 'medium' | 'high';

/** A task in the kanban board */
export interface Task {
  id: string;
  title: string;
  description?: string;
  status: TaskStatus;
  priority: TaskPriority;
  createdAt: string;  // ISO 8601 date string
  updatedAt: string;  // ISO 8601 date string
}

/** Request body for creating a new task */
export interface CreateTaskRequest {
  title: string;
  description?: string;
  priority?: TaskPriority;
}

/** Request body for updating an existing task */
export interface UpdateTaskRequest {
  title?: string;
  description?: string;
  status?: TaskStatus;
  priority?: TaskPriority;
}

/** Response for task list endpoint */
export interface TaskListResponse {
  tasks: Task[];
  total: number;
}

/** Response for single task endpoint */
export interface TaskResponse {
  task: Task;
}
