export type ExecutionStatus = 
  | 'pending' | 'initializing' | 'creating_worktree' 
  | 'starting' | 'running' | 'paused' 
  | 'completed' | 'failed' | 'cancelled' | 'cleaning_up';

export interface ExecutionEventBase {
  id: string;
  session_id: string;
  task_id: string;
  timestamp: string; // ISO string from Rust chrono
}

export type ExecutionEvent = ExecutionEventBase & (
  | { event_type: 'status_changed'; old_status: ExecutionStatus; new_status: ExecutionStatus }
  | { event_type: 'agent_event'; event: AgentEvent }
  | { event_type: 'session_started'; worktree_path: string; branch: string }
  | { event_type: 'session_ended'; status: ExecutionStatus; duration_ms: number }
  | { event_type: 'progress'; message: string; percentage?: number }
);

export type AgentEvent = 
  | { type: 'thinking'; content: string }
  | { type: 'command'; command: string; output: string; exit_code?: number }
  | { type: 'file_change'; path: string; action: FileAction; diff?: string }
  | { type: 'tool_call'; tool: string; args: any; result?: any }
  | { type: 'message'; content: string }
  | { type: 'error'; message: string; recoverable: boolean }
  | { type: 'completed'; success: boolean; summary?: string }
  | { type: 'raw_output'; stream: 'stdout' | 'stderr'; content: string };

export type FileAction = 'created' | 'modified' | 'deleted' | 'renamed';

export interface OrchestratorExecutionEvent {
  executionId: string;
  orgId: string;
  traceId: string;
  seq: number;
  ts: number;
  taskId: string;
  hostId?: string;
  payload: ExecutionEvent;
}

export interface OrchestratorExecutionEventsResponse {
  events: OrchestratorExecutionEvent[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface OrchestratorExecutionListItem {
  executionId: string;
  taskId: string;
  projectId?: string;
  workspaceId?: string;
  hostId?: string;
  traceId?: string;
  orgId?: string;
  parentExecutionId?: string;
  agentRole?: string;
  handoffId?: string;
  agentType: string;
  baseBranch: string;
  status: string;
  createdAt: string;
  startedAt?: string;
  endedAt?: string;
  durationMs?: number;
  summary?: string;
  error?: string;
  eventCount: number;
}

export interface OrchestratorExecutionListResponse {
  items: OrchestratorExecutionListItem[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface OrchestratorAuditEvent {
  id: string;
  ts: string;
  orgId: string;
  actor: string;
  action: string;
  executionId?: string;
  taskId?: string;
  hostId?: string;
  traceId?: string;
  status?: string;
  details?: Record<string, unknown>;
}

export interface OrchestratorAuditListResponse {
  items: OrchestratorAuditEvent[];
  hasMore: boolean;
  nextOffset?: number;
}
