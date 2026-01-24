import * as fs from 'fs';
import * as path from 'path';
import {
  type KanbanBoardState,
  type KanbanTask,
  type KanbanTaskStatus,
  createEmptyBoardState,
} from '@opencode-vibe/protocol';
import { KanbanStore, type StateChangeCallback } from './store';

export class KanbanManager {
  private store: KanbanStore;
  private filePath: string;
  private watcher: fs.FSWatcher | null = null;
  private isSaving = false;

  constructor(projectDir: string) {
    const opencodePath = path.join(projectDir, '.opencode');
    this.filePath = path.join(opencodePath, 'kanban.json');

    // 加载或创建状态
    const initialState = this.loadFromFile(opencodePath);
    this.store = new KanbanStore(initialState);

    // 订阅变更，自动持久化
    this.store.subscribe((state) => {
      this.saveToFile(state);
    });

    // 启动文件监听
    this.startWatching();
  }

  getState(): KanbanBoardState {
    return this.store.getState();
  }

  createTask(title: string, description?: string): KanbanTask {
    return this.store.createTask(title, description);
  }

  moveTask(taskId: string, targetStatus: KanbanTaskStatus, targetIndex?: number): void {
    this.store.moveTask(taskId, targetStatus, targetIndex);
  }

  deleteTask(taskId: string): void {
    this.store.deleteTask(taskId);
  }

  subscribe(callback: StateChangeCallback): () => void {
    return this.store.subscribe(callback);
  }

  dispose(): void {
    if (this.watcher) {
      this.watcher.close();
      this.watcher = null;
    }
  }

  private loadFromFile(opencodePath: string): KanbanBoardState {
    if (!fs.existsSync(this.filePath)) {
      // 创建目录和默认文件
      fs.mkdirSync(opencodePath, { recursive: true });
      const defaultState = createEmptyBoardState();
      fs.writeFileSync(this.filePath, JSON.stringify(defaultState, null, 2));
      return defaultState;
    }

    try {
      const content = fs.readFileSync(this.filePath, 'utf-8');
      return JSON.parse(content) as KanbanBoardState;
    } catch {
      // 文件损坏，返回默认状态
      const defaultState = createEmptyBoardState();
      fs.writeFileSync(this.filePath, JSON.stringify(defaultState, null, 2));
      return defaultState;
    }
  }

  private saveToFile(state: KanbanBoardState): void {
    if (this.isSaving) return;
    this.isSaving = true;

    try {
      fs.writeFileSync(this.filePath, JSON.stringify(state, null, 2));
    } finally {
      // 延迟重置标志，避免文件监听触发循环
      setTimeout(() => {
        this.isSaving = false;
      }, 100);
    }
  }

  private startWatching(): void {
    this.watcher = fs.watch(this.filePath, (eventType) => {
      if (eventType === 'change' && !this.isSaving) {
        // 外部修改，重新加载
        try {
          const content = fs.readFileSync(this.filePath, 'utf-8');
          const newState = JSON.parse(content) as KanbanBoardState;
          this.store.loadState(newState);
        } catch {
          // 忽略解析错误
        }
      }
    });
  }
}
