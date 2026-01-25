/**
 * Agent API Types
 * 
 * Shared type definitions for AI coding agent configuration.
 */

/** Supported AI coding agents */
export type AgentType = 
  | 'opencode'
  | 'claude_code'
  | 'gemini_cli'
  | 'codex'
  | 'amp'
  | 'cursor_cli'
  | 'github_copilot';

/** Agent profile configuration */
export interface AgentProfile {
  id: string;
  name: string;
  agentType: AgentType;
  command: string;        // CLI command to run the agent
  args?: string[];        // Additional CLI arguments
  env?: Record<string, string>;  // Environment variables
  isDefault?: boolean;
}

/** Agent execution status */
export type AgentStatus = 
  | 'idle'
  | 'starting'
  | 'running'
  | 'waiting_approval'
  | 'completed'
  | 'failed'
  | 'stopped';

/** Agent execution session */
export interface AgentSession {
  id: string;
  taskId: string;
  agentProfile: AgentProfile;
  status: AgentStatus;
  worktreePath?: string;
  startedAt: string;
  completedAt?: string;
}
