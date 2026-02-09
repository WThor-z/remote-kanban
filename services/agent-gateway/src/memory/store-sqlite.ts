import { randomUUID } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

import type { MemoryItem, MemoryMutation } from './types.js';
import type { MemoryQuery, MemoryUpdatePatch } from './types.js';

interface PersistedMemoryFile {
  items: MemoryItem[];
}

interface ScoredMemory {
  item: MemoryItem;
  score: number;
}

const DEFAULT_FILE: PersistedMemoryFile = { items: [] };

const nowIso = (): string => new Date().toISOString();

const normalizeText = (value: string): string =>
  value
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .trim();

const tokenize = (value: string): string[] =>
  normalizeText(value)
    .split(/[^a-z0-9_\u4e00-\u9fff]+/i)
    .map((token) => token.trim())
    .filter((token) => token.length >= 2);

const safeJsonParse = <T>(raw: string, fallback: T): T => {
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
};

const buildFtsQuery = (search: string): string => {
  const tokens = tokenize(search);
  if (tokens.length === 0) {
    return '';
  }
  return tokens.map((token) => `"${token.replace(/"/g, '""')}"`).join(' OR ');
};

const hasValue = (value: unknown): value is string =>
  typeof value === 'string' && value.trim().length > 0;

export class MemorySQLiteStore {
  private readonly sqlitePath: string;
  private readonly fallbackPath: string;
  private db: any | null = null;
  private initialized = false;
  private writeQueue: Promise<void> = Promise.resolve();

  constructor(private readonly dataDir: string) {
    this.sqlitePath = path.join(this.dataDir, 'memory.sqlite');
    this.fallbackPath = path.join(this.dataDir, 'memory-items.json');
  }

  async init(): Promise<void> {
    if (this.initialized) {
      return;
    }

    await mkdir(this.dataDir, { recursive: true });
    await this.tryInitSqlite();

    if (!this.db) {
      await this.ensureFallbackFile();
    }

    this.initialized = true;
  }

  private async tryInitSqlite(): Promise<void> {
    try {
      const moduleName = 'node:sqlite';
      const sqliteModule = (await import(moduleName)) as {
        DatabaseSync?: new (file: string) => any;
      };
      if (!sqliteModule.DatabaseSync) {
        return;
      }

      const db = new sqliteModule.DatabaseSync(this.sqlitePath);
      db.exec(`
        CREATE TABLE IF NOT EXISTS memory_items (
          id TEXT PRIMARY KEY,
          host_id TEXT NOT NULL,
          project_id TEXT,
          scope TEXT NOT NULL,
          kind TEXT NOT NULL,
          content TEXT NOT NULL,
          tags TEXT NOT NULL,
          confidence REAL NOT NULL,
          pinned INTEGER NOT NULL DEFAULT 0,
          enabled INTEGER NOT NULL DEFAULT 1,
          source TEXT NOT NULL,
          source_task_id TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          last_used_at TEXT,
          hit_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_items_fts USING fts5(
          content,
          tags_text,
          content='memory_items',
          content_rowid='rowid'
        );

        CREATE TRIGGER IF NOT EXISTS memory_items_ai AFTER INSERT ON memory_items BEGIN
          INSERT INTO memory_items_fts(rowid, content, tags_text)
          VALUES (new.rowid, new.content, new.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS memory_items_ad AFTER DELETE ON memory_items BEGIN
          INSERT INTO memory_items_fts(memory_items_fts, rowid, content, tags_text)
          VALUES('delete', old.rowid, old.content, old.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS memory_items_au AFTER UPDATE ON memory_items BEGIN
          INSERT INTO memory_items_fts(memory_items_fts, rowid, content, tags_text)
          VALUES('delete', old.rowid, old.content, old.tags);
          INSERT INTO memory_items_fts(rowid, content, tags_text)
          VALUES (new.rowid, new.content, new.tags);
        END;

        CREATE INDEX IF NOT EXISTS idx_memory_items_host ON memory_items(host_id);
        CREATE INDEX IF NOT EXISTS idx_memory_items_project ON memory_items(project_id);
        CREATE INDEX IF NOT EXISTS idx_memory_items_scope ON memory_items(scope);
        CREATE INDEX IF NOT EXISTS idx_memory_items_kind ON memory_items(kind);
      `);
      this.db = db;
    } catch {
      this.db = null;
    }
  }

  private async ensureFallbackFile(): Promise<void> {
    try {
      await readFile(this.fallbackPath, 'utf8');
    } catch {
      await writeFile(this.fallbackPath, JSON.stringify(DEFAULT_FILE, null, 2), 'utf8');
    }
  }

  private async withWriteLock<T>(fn: () => Promise<T>): Promise<T> {
    let release: () => void = () => {};
    const previous = this.writeQueue;
    this.writeQueue = new Promise<void>((resolve) => {
      release = resolve;
    });
    await previous;
    try {
      return await fn();
    } finally {
      release();
    }
  }

  private async readFallbackItems(): Promise<MemoryItem[]> {
    const raw = await readFile(this.fallbackPath, 'utf8');
    const parsed = safeJsonParse<PersistedMemoryFile>(raw, DEFAULT_FILE);
    return parsed.items ?? [];
  }

  private async writeFallbackItems(items: MemoryItem[]): Promise<void> {
    await writeFile(this.fallbackPath, JSON.stringify({ items }, null, 2), 'utf8');
  }

  private rowToItem(row: any): MemoryItem {
    return {
      id: String(row.id),
      hostId: String(row.host_id),
      projectId: hasValue(row.project_id) ? String(row.project_id) : undefined,
      scope: row.scope,
      kind: row.kind,
      content: String(row.content),
      tags: safeJsonParse<string[]>(String(row.tags ?? '[]'), []),
      confidence: Number(row.confidence ?? 0.6),
      pinned: Number(row.pinned ?? 0) === 1,
      enabled: Number(row.enabled ?? 1) === 1,
      source: row.source,
      sourceTaskId: hasValue(row.source_task_id) ? String(row.source_task_id) : undefined,
      createdAt: String(row.created_at),
      updatedAt: String(row.updated_at),
      lastUsedAt: hasValue(row.last_used_at) ? String(row.last_used_at) : undefined,
      hitCount: Number(row.hit_count ?? 0),
    };
  }

  private toInsertParams(item: MemoryItem): Record<string, unknown> {
    return {
      id: item.id,
      host_id: item.hostId,
      project_id: item.projectId ?? null,
      scope: item.scope,
      kind: item.kind,
      content: item.content,
      tags: JSON.stringify(item.tags ?? []),
      confidence: item.confidence,
      pinned: item.pinned ? 1 : 0,
      enabled: item.enabled ? 1 : 0,
      source: item.source,
      source_task_id: item.sourceTaskId ?? null,
      created_at: item.createdAt,
      updated_at: item.updatedAt,
      last_used_at: item.lastUsedAt ?? null,
      hit_count: item.hitCount,
    };
  }

  private buildItemFromMutation(mutation: MemoryMutation): MemoryItem {
    const now = nowIso();
    return {
      id: randomUUID(),
      hostId: mutation.hostId,
      projectId: mutation.projectId,
      scope: mutation.scope,
      kind: mutation.kind,
      content: mutation.content.trim(),
      tags: mutation.tags ?? [],
      confidence: mutation.confidence ?? 0.8,
      pinned: mutation.pinned ?? false,
      enabled: mutation.enabled ?? true,
      source: mutation.source ?? 'manual',
      sourceTaskId: mutation.sourceTaskId,
      createdAt: now,
      updatedAt: now,
      lastUsedAt: undefined,
      hitCount: 0,
    };
  }

  private async loadAllItems(): Promise<MemoryItem[]> {
    await this.init();

    if (!this.db) {
      return this.readFallbackItems();
    }

    const rows = this.db.prepare('SELECT * FROM memory_items').all() as any[];
    return rows.map((row) => this.rowToItem(row));
  }

  private async saveItem(item: MemoryItem): Promise<void> {
    await this.init();

    if (!this.db) {
      const items = await this.readFallbackItems();
      const idx = items.findIndex((existing) => existing.id === item.id);
      if (idx >= 0) {
        items[idx] = item;
      } else {
        items.push(item);
      }
      await this.writeFallbackItems(items);
      return;
    }

    this.db
      .prepare(
        `INSERT INTO memory_items (
          id, host_id, project_id, scope, kind, content, tags, confidence, pinned, enabled,
          source, source_task_id, created_at, updated_at, last_used_at, hit_count
        ) VALUES (
          @id, @host_id, @project_id, @scope, @kind, @content, @tags, @confidence, @pinned,
          @enabled, @source, @source_task_id, @created_at, @updated_at, @last_used_at, @hit_count
        )
        ON CONFLICT(id) DO UPDATE SET
          host_id = excluded.host_id,
          project_id = excluded.project_id,
          scope = excluded.scope,
          kind = excluded.kind,
          content = excluded.content,
          tags = excluded.tags,
          confidence = excluded.confidence,
          pinned = excluded.pinned,
          enabled = excluded.enabled,
          source = excluded.source,
          source_task_id = excluded.source_task_id,
          created_at = excluded.created_at,
          updated_at = excluded.updated_at,
          last_used_at = excluded.last_used_at,
          hit_count = excluded.hit_count`
      )
      .run(this.toInsertParams(item));
  }

  private async deleteById(id: string): Promise<boolean> {
    await this.init();
    if (!this.db) {
      const items = await this.readFallbackItems();
      const next = items.filter((item) => item.id !== id);
      if (next.length === items.length) {
        return false;
      }
      await this.writeFallbackItems(next);
      return true;
    }

    const result = this.db.prepare('DELETE FROM memory_items WHERE id = ?').run(id);
    return Number(result.changes ?? 0) > 0;
  }

  private scoreBySearch(item: MemoryItem, query: string, corpusSize: number): number {
    const queryTokens = tokenize(query);
    if (queryTokens.length === 0) {
      return 0;
    }
    const haystack = `${item.content} ${item.tags.join(' ')}`.toLowerCase();
    let score = 0;
    for (const token of queryTokens) {
      const count = haystack.split(token).length - 1;
      if (count > 0) {
        score += (1 + Math.log10(count + 1)) * Math.log10(corpusSize + 1);
      }
    }
    if (item.pinned) {
      score += 1.5;
    }
    if (item.kind === 'preference') {
      score += 0.3;
    }
    return score;
  }

  private filterAndRank(items: MemoryItem[], query: MemoryQuery): MemoryItem[] {
    let filtered = items.filter((item) => item.hostId === query.hostId);
    if (query.projectId) {
      filtered = filtered.filter(
        (item) => item.scope === 'host' || item.projectId === query.projectId
      );
    }
    if (query.scope) {
      filtered = filtered.filter((item) => item.scope === query.scope);
    }
    if (query.kind) {
      filtered = filtered.filter((item) => item.kind === query.kind);
    }
    if (query.enabledOnly) {
      filtered = filtered.filter((item) => item.enabled);
    }

    const scored: ScoredMemory[] = filtered.map((item) => {
      const score = query.search ? this.scoreBySearch(item, query.search, filtered.length) : 0;
      return { item, score };
    });

    scored.sort((a, b) => {
      if (query.search) {
        if (b.score !== a.score) {
          return b.score - a.score;
        }
      }
      if (a.item.pinned !== b.item.pinned) {
        return a.item.pinned ? -1 : 1;
      }
      return b.item.updatedAt.localeCompare(a.item.updatedAt);
    });

    const offset = Math.max(0, query.offset ?? 0);
    const limit = query.limit ? Math.max(1, query.limit) : scored.length;
    return scored.slice(offset, offset + limit).map((entry) => entry.item);
  }

  private async listWithSqlite(query: MemoryQuery): Promise<MemoryItem[]> {
    if (!this.db) {
      return [];
    }

    const limit = Math.max(1, Math.min(query.limit ?? 50, 500));
    const offset = Math.max(0, query.offset ?? 0);
    const filters: string[] = ['m.host_id = @hostId'];
    const params: Record<string, unknown> = {
      hostId: query.hostId,
      limit,
      offset,
    };

    if (query.projectId) {
      filters.push('(m.scope = "host" OR m.project_id = @projectId)');
      params.projectId = query.projectId;
    }
    if (query.scope) {
      filters.push('m.scope = @scope');
      params.scope = query.scope;
    }
    if (query.kind) {
      filters.push('m.kind = @kind');
      params.kind = query.kind;
    }
    if (query.enabledOnly) {
      filters.push('m.enabled = 1');
    }

    const whereClause = filters.length > 0 ? `WHERE ${filters.join(' AND ')}` : '';
    const ftsQuery = query.search ? buildFtsQuery(query.search) : '';
    if (ftsQuery) {
      params.ftsQuery = ftsQuery;
      const rows = this.db
        .prepare(
          `SELECT m.*
           FROM memory_items AS m
           JOIN memory_items_fts AS fts ON fts.rowid = m.rowid
           ${whereClause} AND fts MATCH @ftsQuery
           ORDER BY bm25(memory_items_fts), m.pinned DESC, m.updated_at DESC
           LIMIT @limit OFFSET @offset`
        )
        .all(params) as any[];
      return rows.map((row) => this.rowToItem(row));
    }

    const rows = this.db
      .prepare(
        `SELECT m.*
         FROM memory_items AS m
         ${whereClause}
         ORDER BY m.pinned DESC, m.updated_at DESC
         LIMIT @limit OFFSET @offset`
      )
      .all(params) as any[];
    return rows.map((row) => this.rowToItem(row));
  }

  async list(query: MemoryQuery): Promise<MemoryItem[]> {
    await this.init();

    if (this.db) {
      try {
        return await this.listWithSqlite(query);
      } catch {
        const items = await this.loadAllItems();
        return this.filterAndRank(items, query);
      }
    }

    const items = await this.loadAllItems();
    return this.filterAndRank(items, query);
  }

  async create(mutation: MemoryMutation): Promise<MemoryItem> {
    return this.withWriteLock(async () => {
      const item = this.buildItemFromMutation(mutation);
      await this.saveItem(item);
      return item;
    });
  }

  async update(id: string, patch: MemoryUpdatePatch): Promise<MemoryItem | null> {
    return this.withWriteLock(async () => {
      const items = await this.loadAllItems();
      const current = items.find((item) => item.id === id);
      if (!current) {
        return null;
      }

      const next: MemoryItem = {
        ...current,
        ...patch,
        content: patch.content?.trim() ?? current.content,
        tags: patch.tags ?? current.tags,
        updatedAt: nowIso(),
      };
      await this.saveItem(next);
      return next;
    });
  }

  async delete(id: string): Promise<boolean> {
    return this.withWriteLock(async () => this.deleteById(id));
  }

  async upsertAuto(
    hostId: string,
    projectId: string | undefined,
    candidates: MemoryMutation[]
  ): Promise<MemoryItem[]> {
    return this.withWriteLock(async () => {
      const existing = await this.loadAllItems();
      const upserted: MemoryItem[] = [];

      for (const candidate of candidates) {
        if (!candidate.content.trim()) {
          continue;
        }

        const normalized = normalizeText(candidate.content);
        const duplicate = existing.find(
          (item) =>
            item.hostId === hostId &&
            item.projectId === projectId &&
            item.scope === candidate.scope &&
            item.kind === candidate.kind &&
            normalizeText(item.content) === normalized
        );

        if (duplicate) {
          const next: MemoryItem = {
            ...duplicate,
            content: candidate.content.trim(),
            tags: Array.from(new Set([...(duplicate.tags ?? []), ...(candidate.tags ?? [])])),
            confidence: Math.max(duplicate.confidence, candidate.confidence ?? 0.7),
            enabled: true,
            updatedAt: nowIso(),
            source: candidate.source ?? duplicate.source,
            sourceTaskId: candidate.sourceTaskId ?? duplicate.sourceTaskId,
          };
          await this.saveItem(next);
          const existingIdx = existing.findIndex((item) => item.id === duplicate.id);
          if (existingIdx >= 0) {
            existing[existingIdx] = next;
          }
          upserted.push(next);
          continue;
        }

        const created = this.buildItemFromMutation({
          ...candidate,
          hostId,
          projectId,
          enabled: true,
        });
        await this.saveItem(created);
        existing.push(created);
        upserted.push(created);
      }

      return upserted;
    });
  }

  async touchHits(items: MemoryItem[]): Promise<void> {
    await this.withWriteLock(async () => {
      const touchedAt = nowIso();
      for (const item of items) {
        const next: MemoryItem = {
          ...item,
          hitCount: item.hitCount + 1,
          lastUsedAt: touchedAt,
          updatedAt: touchedAt,
        };
        await this.saveItem(next);
      }
    });
  }
}
