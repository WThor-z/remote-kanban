import os from 'node:os';
import path from 'node:path';
import { mkdtemp, rm } from 'node:fs/promises';
import { describe, expect, it } from 'vitest';

import { MemoryRetriever } from './retriever.js';
import { MemorySQLiteStore } from './store-sqlite.js';

describe('MemoryRetriever', () => {
  it('prioritizes pinned project memory and respects token budget', async () => {
    const tempDir = await mkdtemp(path.join(os.tmpdir(), 'memory-retriever-'));
    try {
      const store = new MemorySQLiteStore(tempDir);
      await store.init();

      await store.create({
        hostId: 'host-a',
        projectId: 'proj-a',
        scope: 'project',
        kind: 'constraint',
        content: 'Always run tests before commit.',
        pinned: true,
        source: 'manual',
      });
      await store.create({
        hostId: 'host-a',
        scope: 'host',
        kind: 'preference',
        content: 'Prefer concise commit messages.',
        source: 'manual',
      });

      const retriever = new MemoryRetriever(store);
      const result = await retriever.retrieve(
        {
          hostId: 'host-a',
          projectId: 'proj-a',
          search: 'tests commit message',
          enabledOnly: true,
        },
        5,
        200
      );

      expect(result.items.length).toBeGreaterThan(0);
      expect(result.items[0].pinned).toBe(true);
      expect(result.context).toContain('Relevant memory context:');
      expect(result.estimatedTokens).toBeLessThanOrEqual(200);
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
