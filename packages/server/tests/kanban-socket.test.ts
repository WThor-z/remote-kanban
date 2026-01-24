import { describe, it, expect, beforeEach, afterEach, vi, beforeAll, afterAll } from 'vitest';
import { io as ioc, Socket } from 'socket.io-client';
import { startServer } from '../src/server';
import type { KanbanBoardState } from '@opencode-vibe/protocol';

// 注意：这是集成测试，需要实际启动服务器
// 由于 KanbanManager 需要真实文件系统，这里使用 mock

describe('Kanban Socket Events', () => {
  let server: ReturnType<typeof startServer>;
  let client: Socket;
  const PORT = 3099;

  beforeAll(() => {
    server = startServer(PORT);
  });

  afterAll(() => {
    server.stop();
  });

  beforeEach(() => {
    return new Promise<void>((resolve) => {
      client = ioc(`http://localhost:${PORT}`, {
        transports: ['websocket'],
      });
      client.on('connect', resolve);
    });
  });

  afterEach(() => {
    client.disconnect();
  });

  it('客户端连接后收到 kanban:sync 事件', async () => {
    const syncPromise = new Promise<KanbanBoardState>((resolve) => {
      client.on('kanban:sync', (state: KanbanBoardState) => {
        resolve(state);
      });
    });

    // 请求同步
    client.emit('kanban:request-sync');

    const state = await syncPromise;
    expect(state.columnOrder).toEqual(['todo', 'doing', 'done']);
  });

  it('创建任务后收到更新', async () => {
    const updatePromise = new Promise<KanbanBoardState>((resolve) => {
      client.on('kanban:sync', (state: KanbanBoardState) => {
        if (Object.keys(state.tasks).length > 0) {
          resolve(state);
        }
      });
    });

    client.emit('kanban:create', { title: 'Socket 测试任务' });

    const state = await updatePromise;
    const tasks = Object.values(state.tasks);
    expect(tasks.some((t) => t.title === 'Socket 测试任务')).toBe(true);
  });
});
