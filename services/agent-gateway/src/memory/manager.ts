import type { MemoryKind, MemoryScope, MemorySettings, TaskMemoryMetadata, TaskRequest } from '../types.js';
import { clampMemorySettings, mergeMemorySettings } from './settings.js';
import { extractLlmCandidates, shouldRunLlmFallback } from './extractor-llm.js';
import { extractRuleCandidates } from './extractor-rule.js';
import { MemoryRetriever } from './retriever.js';
import { MemoryMarkdownStore } from './store-markdown.js';
import { MemorySQLiteStore } from './store-sqlite.js';
import type {
  MemoryExtractCandidate,
  MemoryManagerOptions,
  MemoryMutation,
  MemoryQuery,
  MemoryUpdatePatch,
  PostRunPersistContext,
  PreparePromptResult,
} from './types.js';

const asString = (value: unknown): string | undefined =>
  typeof value === 'string' && value.trim().length > 0 ? value.trim() : undefined;

const asBoolean = (value: unknown): boolean | undefined =>
  typeof value === 'boolean' ? value : undefined;

const asNumber = (value: unknown): number | undefined =>
  typeof value === 'number' && Number.isFinite(value) ? value : undefined;

const uniqueCandidates = (items: MemoryExtractCandidate[]): MemoryExtractCandidate[] => {
  const seen = new Set<string>();
  const deduped: MemoryExtractCandidate[] = [];
  for (const item of items) {
    const key = `${item.scope}|${item.kind}|${item.content.toLowerCase().trim()}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    deduped.push(item);
  }
  return deduped;
};

const metadataFromTask = (task: TaskRequest): TaskMemoryMetadata => {
  const metadata = (task.metadata ?? {}) as Record<string, unknown>;
  return {
    projectId: asString(metadata.projectId),
    taskId: asString(metadata.taskId) ?? task.taskId,
    taskTitle: asString(metadata.taskTitle),
    taskDescription: asString(metadata.taskDescription),
    memorySettingsSnapshot:
      typeof metadata.memorySettingsSnapshot === 'object' && metadata.memorySettingsSnapshot
        ? (metadata.memorySettingsSnapshot as Partial<MemorySettings>)
        : undefined,
  };
};

export class AgentMemoryManager {
  private readonly store: MemorySQLiteStore;
  private readonly markdownStore: MemoryMarkdownStore;
  private readonly retriever: MemoryRetriever;
  private settings: MemorySettings;
  private initialized = false;

  constructor(private readonly options: MemoryManagerOptions) {
    this.store = new MemorySQLiteStore(options.dataDir);
    this.markdownStore = new MemoryMarkdownStore(options.dataDir);
    this.retriever = new MemoryRetriever(this.store);
    this.settings = clampMemorySettings(options.settings);
  }

  async init(): Promise<void> {
    if (this.initialized) {
      return;
    }
    await this.store.init();
    this.initialized = true;
  }

  getSettings(): MemorySettings {
    return { ...this.settings };
  }

  updateSettings(patch: Partial<MemorySettings>): MemorySettings {
    this.settings = clampMemorySettings(mergeMemorySettings(this.settings, patch));
    return this.getSettings();
  }

  async queryItems(query: MemoryQuery): Promise<unknown> {
    await this.init();
    return this.store.list(query);
  }

  async createItem(mutation: MemoryMutation): Promise<unknown> {
    await this.init();
    return this.store.create(mutation);
  }

  async updateItem(id: string, patch: MemoryUpdatePatch): Promise<unknown> {
    await this.init();
    return this.store.update(id, patch);
  }

  async deleteItem(id: string): Promise<unknown> {
    await this.init();
    const deleted = await this.store.delete(id);
    return { deleted };
  }

  private async extractCandidates(ctx: PostRunPersistContext): Promise<MemoryExtractCandidate[]> {
    const metadata = metadataFromTask(ctx.task);
    const extractContext = {
      hostId: this.options.hostId,
      projectId: metadata.projectId,
      taskId: metadata.taskId,
      taskTitle: metadata.taskTitle,
      taskDescription: metadata.taskDescription,
      taskPrompt: ctx.originalPrompt,
      taskOutput: ctx.output,
    };

    const ruleCandidates = extractRuleCandidates(extractContext);
    let llmCandidates: MemoryExtractCandidate[] = [];

    if (
      this.settings.llmExtractEnabled &&
      shouldRunLlmFallback(ruleCandidates, 3, 0.65)
    ) {
      llmCandidates = await extractLlmCandidates({
        opencodeClient: ctx.opencodeClient,
        model: ctx.model,
        context: extractContext,
      });
    }

    return uniqueCandidates([...ruleCandidates, ...llmCandidates]).slice(0, 10);
  }

  async preparePrompt(task: TaskRequest, basePrompt: string): Promise<PreparePromptResult> {
    await this.init();

    const metadata = metadataFromTask(task);
    const snapshot = metadata.memorySettingsSnapshot
      ? clampMemorySettings(mergeMemorySettings(this.settings, metadata.memorySettingsSnapshot))
      : this.settings;

    if (!snapshot.enabled || !snapshot.promptInjection || !snapshot.gatewayStoreEnabled) {
      return {
        prompt: basePrompt,
        injectedCount: 0,
        estimatedTokens: 0,
      };
    }

    const retrieved = await this.retriever.retrieve(
      {
        hostId: this.options.hostId,
        projectId: metadata.projectId,
        search: `${metadata.taskTitle ?? ''}\n${metadata.taskDescription ?? ''}\n${basePrompt}`.trim(),
      },
      snapshot.retrievalTopK,
      snapshot.tokenBudget
    );

    if (!retrieved.context) {
      return {
        prompt: basePrompt,
        injectedCount: 0,
        estimatedTokens: 0,
      };
    }

    const prompt = `${retrieved.context}\n\nTask instruction:\n${basePrompt}`;
    return {
      prompt,
      injectedCount: retrieved.items.length,
      estimatedTokens: retrieved.estimatedTokens,
    };
  }

  async postRunPersist(ctx: PostRunPersistContext): Promise<{ count: number }> {
    await this.init();
    if (!this.settings.enabled || !this.settings.autoWrite || !this.settings.gatewayStoreEnabled) {
      return { count: 0 };
    }

    const metadata = metadataFromTask(ctx.task);
    const candidates = await this.extractCandidates(ctx);
    if (candidates.length === 0) {
      return { count: 0 };
    }

    const mutations: MemoryMutation[] = candidates.map((candidate) => ({
      hostId: this.options.hostId,
      projectId: metadata.projectId,
      scope: candidate.scope,
      kind: candidate.kind,
      content: candidate.content,
      tags: candidate.tags,
      confidence: candidate.confidence,
      source: candidate.source,
      sourceTaskId: metadata.taskId ?? ctx.task.taskId,
      enabled: true,
    }));

    const upserted = await this.store.upsertAuto(this.options.hostId, metadata.projectId, mutations);
    for (const item of upserted) {
      await this.markdownStore.write(item, item.scope === 'project' ? ctx.task.cwd : undefined);
    }

    if (this.settings.rustStoreEnabled && this.options.onSync) {
      this.options.onSync({
        hostId: this.options.hostId,
        projectId: metadata.projectId,
        op: 'upsert',
        items: upserted,
      });
    }

    return { count: upserted.length };
  }

  async handleMemoryRequest(action: string, payload: Record<string, unknown>): Promise<unknown> {
    await this.init();
    switch (action) {
      case 'settings.get':
        return this.getSettings();
      case 'settings.update': {
        const patch = payload.patch as Partial<MemorySettings> | undefined;
        return this.updateSettings(patch ?? {});
      }
      case 'items.list': {
        const query: MemoryQuery = {
          hostId: asString(payload.hostId) ?? this.options.hostId,
          projectId: asString(payload.projectId),
          scope: asString(payload.scope) as MemoryScope | undefined,
          kind: asString(payload.kind) as MemoryKind | undefined,
          search: asString(payload.search),
          enabledOnly: asBoolean(payload.enabledOnly),
          limit: asNumber(payload.limit),
          offset: asNumber(payload.offset),
        };
        return this.queryItems(query);
      }
      case 'items.create': {
        const mutation: MemoryMutation = {
          hostId: asString(payload.hostId) ?? this.options.hostId,
          projectId: asString(payload.projectId),
          scope: (asString(payload.scope) as MemoryScope) ?? 'project',
          kind: (asString(payload.kind) as MemoryKind) ?? 'fact',
          content: asString(payload.content) ?? '',
          tags: Array.isArray(payload.tags)
            ? payload.tags.filter((tag): tag is string => typeof tag === 'string')
            : [],
          confidence: asNumber(payload.confidence) ?? 0.8,
          pinned: asBoolean(payload.pinned) ?? false,
          enabled: asBoolean(payload.enabled) ?? true,
          source: 'manual',
          sourceTaskId: asString(payload.sourceTaskId),
        };
        if (!mutation.content.trim()) {
          throw new Error('Memory content is required');
        }
        return this.createItem(mutation);
      }
      case 'items.update': {
        const id = asString(payload.id);
        if (!id) {
          return null;
        }
        const patch: MemoryUpdatePatch = {
          content: asString(payload.content),
          kind: asString(payload.kind) as MemoryKind | undefined,
          scope: asString(payload.scope) as MemoryScope | undefined,
          tags: Array.isArray(payload.tags)
            ? payload.tags.filter((tag): tag is string => typeof tag === 'string')
            : undefined,
          confidence: asNumber(payload.confidence),
          pinned: asBoolean(payload.pinned),
          enabled: asBoolean(payload.enabled),
        };
        return this.updateItem(id, patch);
      }
      case 'items.delete': {
        const id = asString(payload.id);
        if (!id) {
          return { deleted: false };
        }
        return this.deleteItem(id);
      }
      default:
        throw new Error(`Unsupported memory action: ${action}`);
    }
  }
}
