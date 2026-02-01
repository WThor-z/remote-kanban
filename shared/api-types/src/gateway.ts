// shared/api-types/src/gateway.ts

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

/** Gateway agent event - emitted during task execution */
export interface GatewayAgentEvent {
  type: GatewayAgentEventType;
  content?: string;
  data?: unknown;
  timestamp: number;
}

/** Task result - returned when task completes */
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
  | { type: 'task:input'; taskId: string; content: string }
  | { type: 'config:update'; config: Partial<GatewayConfig> };

/** Gateway configuration (sent from server) */
export interface GatewayConfig {
  heartbeatInterval: number;
  taskTimeout: number;
  maxRetries: number;
}

/** Host status - used for API responses */
export interface HostStatus {
  hostId: string;
  name: string;
  status: 'online' | 'offline' | 'busy';
  capabilities: HostCapabilities;
  activeTasks: string[];
  lastHeartbeat: number;
  connectedAt: number;
}
