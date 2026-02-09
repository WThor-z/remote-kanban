import { appendFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

import type { MemoryItem } from './types.js';

const dateKey = (): string => new Date().toISOString().slice(0, 10);
const timeKey = (): string => new Date().toISOString();

const ensureDir = async (dir: string): Promise<void> => {
  await mkdir(dir, { recursive: true });
};

const fileExistsContent = async (filePath: string): Promise<string> => {
  try {
    return await readFile(filePath, 'utf8');
  } catch {
    return '';
  }
};

const toBullet = (item: MemoryItem): string => {
  const tags = item.tags.length > 0 ? ` tags=${item.tags.join(',')}` : '';
  return `- [${item.kind}] ${item.content} (confidence=${item.confidence.toFixed(2)}, source=${item.source}${tags})`;
};

export class MemoryMarkdownStore {
  constructor(private readonly gatewayDataDir: string) {}

  private projectRoot(projectCwd: string): string {
    return path.join(projectCwd, '.opencode', 'memory');
  }

  private hostRoot(): string {
    return path.join(this.gatewayDataDir, '.opencode', 'memory', 'global');
  }

  private resolveRoot(scope: MemoryItem['scope'], projectCwd?: string): string | null {
    if (scope === 'project') {
      if (!projectCwd) {
        return null;
      }
      return this.projectRoot(projectCwd);
    }
    return this.hostRoot();
  }

  async write(item: MemoryItem, projectCwd?: string): Promise<void> {
    const root = this.resolveRoot(item.scope, projectCwd);
    if (!root) {
      return;
    }

    await ensureDir(path.join(root, 'daily'));
    const dailyPath = path.join(root, 'daily', `${dateKey()}.md`);
    const summaryPath = path.join(root, 'MEMORY.md');

    const line = `${timeKey()} ${toBullet(item)}\n`;
    await appendFile(dailyPath, line, 'utf8');

    const currentSummary = await fileExistsContent(summaryPath);
    const bullet = toBullet(item);
    if (!currentSummary.includes(bullet)) {
      const next = currentSummary
        ? `${currentSummary.trimEnd()}\n${bullet}\n`
        : `# Memory\n\n${bullet}\n`;
      await writeFile(summaryPath, next, 'utf8');
    }
  }
}
