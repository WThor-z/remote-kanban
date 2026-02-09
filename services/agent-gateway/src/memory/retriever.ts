import type { MemoryItem } from './types.js';
import type { MemoryQuery, RetrievalResult } from './types.js';
import { MemorySQLiteStore } from './store-sqlite.js';

const estimateTokens = (text: string): number => Math.ceil(text.length / 4);

const sortByInjectionPriority = (items: MemoryItem[]): MemoryItem[] => {
  return [...items].sort((a, b) => {
    const bucket = (item: MemoryItem): number => {
      if (item.scope === 'project' && item.pinned) {
        return 0;
      }
      if (item.scope === 'project') {
        return 1;
      }
      if (item.scope === 'host' && item.kind === 'preference') {
        return 2;
      }
      return 3;
    };
    const bucketDiff = bucket(a) - bucket(b);
    if (bucketDiff !== 0) {
      return bucketDiff;
    }
    if (a.pinned !== b.pinned) {
      return a.pinned ? -1 : 1;
    }
    return b.updatedAt.localeCompare(a.updatedAt);
  });
};

const buildContextBlock = (items: MemoryItem[]): string => {
  if (items.length === 0) {
    return '';
  }

  const lines = items.map((item) => {
    const prefix = `[${item.scope}/${item.kind}]`;
    return `- ${prefix} ${item.content}`;
  });

  return [
    'Relevant memory context:',
    ...lines,
    '',
    'Use this context when helpful. If conflicts appear, prioritize the latest task instruction.',
  ].join('\n');
};

export class MemoryRetriever {
  constructor(private readonly store: MemorySQLiteStore) {}

  async retrieve(query: MemoryQuery, topK: number, tokenBudget: number): Promise<RetrievalResult> {
    const initial = await this.store.list({
      ...query,
      enabledOnly: true,
      limit: Math.max(topK * 3, topK),
      offset: 0,
    });
    const prioritized = sortByInjectionPriority(initial).slice(0, Math.max(1, topK * 2));

    const selected: MemoryItem[] = [];
    let currentTokens = 0;

    for (const item of prioritized) {
      const tokenCost = estimateTokens(item.content) + 8;
      if (selected.length > 0 && currentTokens + tokenCost > tokenBudget) {
        continue;
      }
      selected.push(item);
      currentTokens += tokenCost;
      if (selected.length >= topK) {
        break;
      }
    }

    await this.store.touchHits(selected);

    const context = buildContextBlock(selected);
    return {
      items: selected,
      context,
      estimatedTokens: estimateTokens(context),
    };
  }
}
