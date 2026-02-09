import type { MemoryItem, MemoryKind, MemoryScope, MemorySettings, MemorySource, TaskRequest } from '../types.js';

export type {
  MemoryItem,
  MemoryKind,
  MemoryScope,
  MemorySettings,
  MemorySource,
};

export interface MemoryQuery {
  hostId: string;
  projectId?: string;
  scope?: MemoryScope;
  kind?: MemoryKind;
  search?: string;
  enabledOnly?: boolean;
  limit?: number;
  offset?: number;
}

export interface MemoryMutation {
  hostId: string;
  projectId?: string;
  scope: MemoryScope;
  kind: MemoryKind;
  content: string;
  tags?: string[];
  confidence?: number;
  pinned?: boolean;
  enabled?: boolean;
  source?: MemorySource;
  sourceTaskId?: string;
}

export interface MemoryUpdatePatch {
  content?: string;
  kind?: MemoryKind;
  scope?: MemoryScope;
  tags?: string[];
  confidence?: number;
  pinned?: boolean;
  enabled?: boolean;
}

export interface RetrievalResult {
  items: MemoryItem[];
  context: string;
  estimatedTokens: number;
}

export interface MemoryExtractContext {
  hostId: string;
  projectId?: string;
  taskId?: string;
  taskTitle?: string;
  taskDescription?: string;
  taskPrompt: string;
  taskOutput: string;
}

export interface MemoryExtractCandidate {
  scope: MemoryScope;
  kind: MemoryKind;
  content: string;
  tags: string[];
  confidence: number;
  source: MemorySource;
}

export interface PreparePromptResult {
  prompt: string;
  injectedCount: number;
  estimatedTokens: number;
}

export interface PostRunPersistContext {
  task: TaskRequest;
  originalPrompt: string;
  finalPrompt: string;
  output: string;
  model?: string;
  opencodeClient?: unknown;
}

export interface MemorySyncPayload {
  hostId: string;
  projectId?: string;
  op: 'upsert' | 'delete';
  items: MemoryItem[];
}

export interface MemoryManagerOptions {
  hostId: string;
  dataDir: string;
  settings: MemorySettings;
  onSync?: (sync: MemorySyncPayload) => void;
}
