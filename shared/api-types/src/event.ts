/**
 * Event API Types
 * 
 * Shared type definitions for real-time events (WebSocket).
 */

/** Agent event types */
export type AgentEventType = 
  | 'thinking'
  | 'command'
  | 'file_change'
  | 'tool_call'
  | 'message'
  | 'error'
  | 'approval_required'
  | 'completed';

/** Base event interface */
export interface BaseEvent {
  type: AgentEventType;
  timestamp: string;
  taskId: string;
  sessionId: string;
}

/** Agent thinking/reasoning event */
export interface ThinkingEvent extends BaseEvent {
  type: 'thinking';
  content: string;
}

/** Command execution event */
export interface CommandEvent extends BaseEvent {
  type: 'command';
  command: string;
  output?: string;
  exitCode?: number;
}

/** File change event */
export interface FileChangeEvent extends BaseEvent {
  type: 'file_change';
  path: string;
  changeType: 'added' | 'modified' | 'deleted';
  diff?: string;
  additions?: number;
  deletions?: number;
}

/** Tool call event */
export interface ToolCallEvent extends BaseEvent {
  type: 'tool_call';
  tool: string;
  args: Record<string, unknown>;
  result?: unknown;
}

/** Agent message event */
export interface MessageEvent extends BaseEvent {
  type: 'message';
  content: string;
}

/** Error event */
export interface ErrorEvent extends BaseEvent {
  type: 'error';
  message: string;
  stack?: string;
}

/** Approval required event */
export interface ApprovalRequiredEvent extends BaseEvent {
  type: 'approval_required';
  action: string;
  description: string;
  riskLevel: 'low' | 'medium' | 'high';
}

/** Completion event */
export interface CompletedEvent extends BaseEvent {
  type: 'completed';
  success: boolean;
  summary?: string;
}

/** Union type for all agent events */
export type AgentEvent = 
  | ThinkingEvent
  | CommandEvent
  | FileChangeEvent
  | ToolCallEvent
  | MessageEvent
  | ErrorEvent
  | ApprovalRequiredEvent
  | CompletedEvent;

/** WebSocket message from client to server */
export type ClientMessage = 
  | { type: 'subscribe'; taskId: string }
  | { type: 'unsubscribe'; taskId: string }
  | { type: 'send_message'; taskId: string; content: string }
  | { type: 'approve'; taskId: string; eventId: string }
  | { type: 'deny'; taskId: string; eventId: string };

/** WebSocket message from server to client */
export type ServerMessage = 
  | { type: 'event'; event: AgentEvent }
  | { type: 'status_change'; taskId: string; status: string }
  | { type: 'error'; message: string };
