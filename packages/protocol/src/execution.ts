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
