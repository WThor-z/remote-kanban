// Re-export types from shared package if available, or define locally

/** Host capabilities - describes what agents a host supports */
export interface HostCapabilities {
  name: string;
  agents: ('opencode' | 'claude-code' | 'gemini')[];
  maxConcurrent: number;
  cwd: string;
  labels?: Record<string, string>;
}

export type MemoryScope = 'project' | 'host';
export type MemoryKind = 'preference' | 'constraint' | 'fact' | 'workflow';
export type MemorySource = 'auto_rule' | 'auto_llm' | 'manual';

export interface MemoryItem {
  id: string;
  hostId: string;
  projectId?: string;
  scope: MemoryScope;
  kind: MemoryKind;
  content: string;
  tags: string[];
  confidence: number;
  pinned: boolean;
  enabled: boolean;
  source: MemorySource;
  sourceTaskId?: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string;
  hitCount: number;
}

export interface MemorySettings {
  enabled: boolean;
  gatewayStoreEnabled: boolean;
  rustStoreEnabled: boolean;
  autoWrite: boolean;
  promptInjection: boolean;
  tokenBudget: number;
  retrievalTopK: number;
  llmExtractEnabled: boolean;
}

export interface MemorySettingsSnapshot extends MemorySettings {}

export interface TaskMemoryMetadata {
  projectId?: string;
  taskId?: string;
  taskTitle?: string;
  taskDescription?: string;
  memorySettingsSnapshot?: Partial<MemorySettingsSnapshot>;
}

export type GatewayMemoryAction =
  | 'settings.get'
  | 'settings.update'
  | 'items.list'
  | 'items.create'
  | 'items.update'
  | 'items.delete';

export interface GatewayMemoryRequest {
  requestId: string;
  action: GatewayMemoryAction;
  payload: Record<string, unknown>;
}

export interface GatewayMemoryResponse {
  requestId: string;
  ok: boolean;
  data?: unknown;
  error?: string;
}

export interface GatewayMemorySync {
  hostId: string;
  projectId?: string;
  op: 'upsert' | 'delete';
  items: MemoryItem[];
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
  metadata?: Record<string, unknown> & TaskMemoryMetadata;
}

/** Model information from OpenCode */
export interface ModelInfo {
  id: string;
  providerId: string;
  name: string;
  capabilities?: {
    temperature: boolean;
    reasoning: boolean;
    attachment: boolean;
    toolcall: boolean;
  };
}

/** Provider information from OpenCode */
export interface ProviderInfo {
  id: string;
  name: string;
  models: ModelInfo[];
}

/** Gateway agent event types */
export type GatewayAgentEventType =
  | 'log'
  | 'thinking'
  | 'tool_call'
  | 'tool_result'
  | 'file_change'
  | 'message'
  | 'error'
  | 'stdout'
  | 'stderr'
  | 'output';

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
  | { type: 'task:failed'; taskId: string; error: string; details?: unknown }
  | { type: 'models:response'; requestId: string; providers: ProviderInfo[] }
  | ({ type: 'memory:response' } & GatewayMemoryResponse)
  | { type: 'memory:sync'; sync: GatewayMemorySync };

/** Server -> Gateway messages */
export type ServerToGatewayMessage =
  | { type: 'registered'; ok: boolean; error?: string }
  | { type: 'ping' }
  | { type: 'task:execute'; task: TaskRequest }
  | { type: 'task:abort'; taskId: string }
  | { type: 'task:input'; taskId: string; content: string }
  | { type: 'models:request'; requestId: string }
  | ({ type: 'memory:request' } & GatewayMemoryRequest);

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
