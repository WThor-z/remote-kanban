// Re-export types from shared package if available, or define locally

/** Host capabilities - describes what agents a host supports */
export interface HostCapabilities {
  name: string;
  agents: ('opencode' | 'claude-code' | 'gemini')[];
  maxConcurrent: number;
  cwd: string;
  labels?: Record<string, string>;
}

/** Task request sent from server to gateway */
export interface TaskRequest {
  taskId: string;
  prompt: string;
  cwd: string;
  agentType: string;
  model?: string;
  env?: Record<string, string>;
  timeout?: number;
  metadata?: Record<string, unknown>;
}

/** Gateway agent event types */
export type GatewayAgentEventType = 
  | 'log' 
  | 'thinking' 
  | 'tool_call' 
  | 'tool_result' 
  | 'file_change' 
  | 'message' 
  | 'error';

/** Gateway agent event */
export interface GatewayAgentEvent {
  type: GatewayAgentEventType;
  content?: string;
  data?: unknown;
  timestamp: number;
}

/** Task result */
export interface TaskResult {
  success: boolean;
  exitCode?: number;
  output?: string;
  duration?: number;
  filesChanged?: string[];
}

/** Gateway -> Server messages */
export type GatewayToServerMessage =
  | { type: 'register'; hostId: string; capabilities: HostCapabilities }
  | { type: 'heartbeat'; timestamp: number }
  | { type: 'task:started'; taskId: string; sessionId: string }
  | { type: 'task:event'; taskId: string; event: GatewayAgentEvent }
  | { type: 'task:completed'; taskId: string; result: TaskResult }
  | { type: 'task:failed'; taskId: string; error: string; details?: unknown };

/** Server -> Gateway messages */
export type ServerToGatewayMessage =
  | { type: 'registered'; ok: boolean; error?: string }
  | { type: 'ping' }
  | { type: 'task:execute'; task: TaskRequest }
  | { type: 'task:abort'; taskId: string }
  | { type: 'task:input'; taskId: string; content: string };

/** Gateway connection options */
export interface GatewayOptions {
  serverUrl: string;
  hostId: string;
  authToken: string;
  capabilities: HostCapabilities;
  reconnect?: boolean;
}

/** Connection state */
export interface ConnectionState {
  status: 'disconnected' | 'connecting' | 'connected' | 'registered';
  lastError?: string;
  reconnectAttempt: number;
}
