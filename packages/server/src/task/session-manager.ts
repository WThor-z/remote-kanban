import { EventEmitter } from 'events';
import { promises as fs } from 'fs';
import * as path from 'path';
import type {
  TaskSessionHistory,
  ChatMessage,
  AgentSessionStatus,
  AgentOutputEvent,
  AgentStatusEvent,
  KanbanTask,
} from '@opencode-vibe/protocol';
import {
  createTaskSessionHistory,
  generateMessageId,
  createAgentSession,
  AGENT_PRESETS,
} from '@opencode-vibe/protocol';
import { AgentExecutor } from '../agent/executor.js';
import { KanbanStore } from '../kanban/store.js';

export interface TaskSessionManagerOptions {
  /** 任务存储目录 */
  tasksDir: string;
  /** Kanban Store 实例 */
  kanbanStore: KanbanStore;
  /** Agent Executor 实例 */
  agentExecutor: AgentExecutor;
}

type TaskSessionEventType = 
  | 'status'
  | 'message'
  | 'history'
  | 'error';

/**
 * TaskSessionManager - 管理任务与 Agent 会话的关联
 *
 * 职责：
 * 1. 当任务开始执行时，启动 Agent 会话
 * 2. 将 Agent 输出转换为 ChatMessage 并持久化
 * 3. 当 Agent 完成时，自动更新 Kanban 任务状态
 */
export class TaskSessionManager {
  private tasksDir: string;
  private kanbanStore: KanbanStore;
  private agentExecutor: AgentExecutor;
  private eventEmitter: EventEmitter = new EventEmitter();

  /** taskId -> sessionId 映射 */
  private taskToSession: Map<string, string> = new Map();
  /** sessionId -> taskId 映射 */
  private sessionToTask: Map<string, string> = new Map();
  /** 活跃的任务会话历史 (内存缓存) */
  private activeHistories: Map<string, TaskSessionHistory> = new Map();

  constructor(options: TaskSessionManagerOptions) {
    this.tasksDir = options.tasksDir;
    this.kanbanStore = options.kanbanStore;
    this.agentExecutor = options.agentExecutor;

    // 监听 Agent 事件
    this.setupAgentListeners();
  }

  /**
   * 初始化 - 确保任务目录存在
   */
  async initialize(): Promise<void> {
    await fs.mkdir(this.tasksDir, { recursive: true });
  }

  /**
   * 执行任务 - 启动 Agent 会话
   */
  async executeTask(taskId: string): Promise<void> {
    // 获取任务信息
    const state = this.kanbanStore.getState();
    const task = state.tasks[taskId];

    if (!task) {
      throw new Error(`Task ${taskId} not found`);
    }

    // 检查是否已有活跃会话
    if (this.taskToSession.has(taskId)) {
      const existingSessionId = this.taskToSession.get(taskId)!;
      const existingSession = this.agentExecutor.getSession(existingSessionId);
      if (existingSession && ['starting', 'running'].includes(existingSession.status)) {
        throw new Error(`Task ${taskId} is already running`);
      }
    }

    // 创建或加载任务会话历史
    let history = await this.loadHistory(taskId);
    if (!history) {
      history = createTaskSessionHistory(
        taskId,
        task.title,
        task.description || task.title
      );
    }

    // 更新会话状态
    history.status = 'starting';
    history.startedAt = Date.now();

    // 添加用户消息 (任务描述作为初始 prompt)
    const userMessage: ChatMessage = {
      id: generateMessageId(),
      role: 'user',
      content: task.description || task.title,
      timestamp: Date.now(),
    };
    history.messages.push(userMessage);

    // 保存并缓存
    await this.saveHistory(history);
    this.activeHistories.set(taskId, history);

    // 建立映射关系
    this.taskToSession.set(taskId, history.sessionId);
    this.sessionToTask.set(history.sessionId, taskId);

    // 更新 Kanban 任务状态为 doing
    this.updateTaskStatus(taskId, 'doing', history.sessionId);

    // 发送状态事件
    this.emit('status', { taskId, status: 'starting' });
    this.emit('message', { taskId, message: userMessage });

    // 创建 Agent 会话并启动
    const agentSession = createAgentSession(
      'opencode',
      task.description || task.title,
      taskId
    );
    agentSession.id = history.sessionId;

    try {
      await this.agentExecutor.start(agentSession, {
        ...AGENT_PRESETS.opencode,
        cwd: process.cwd(),
      });
    } catch (error) {
      // 启动失败
      history.status = 'failed';
      history.error = error instanceof Error ? error.message : String(error);
      await this.saveHistory(history);
      this.emit('error', { taskId, error: history.error });
    }
  }

  /**
   * 停止任务执行
   */
  async stopTask(taskId: string): Promise<void> {
    const sessionId = this.taskToSession.get(taskId);
    if (!sessionId) {
      throw new Error(`No active session for task ${taskId}`);
    }

    await this.agentExecutor.stop(sessionId);

    // 更新历史
    const history = this.activeHistories.get(taskId);
    if (history) {
      history.status = 'aborted';
      history.completedAt = Date.now();
      await this.saveHistory(history);
    }

    // 更新任务状态回 todo
    this.updateTaskStatus(taskId, 'todo');
    this.emit('status', { taskId, status: 'aborted' });
  }

  /**
   * 发送用户消息到正在执行的任务
   */
  async sendMessage(taskId: string, content: string): Promise<void> {
    const history = this.activeHistories.get(taskId);
    if (!history) {
      throw new Error(`No active session for task ${taskId}`);
    }

    // 添加用户消息
    const userMessage: ChatMessage = {
      id: generateMessageId(),
      role: 'user',
      content,
      timestamp: Date.now(),
    };
    history.messages.push(userMessage);
    await this.saveHistory(history);
    this.emit('message', { taskId, message: userMessage });

    // TODO: 发送到 Agent (需要 OpenCode API 支持连续对话)
    // 当前 OpenCode API 每次 sendMessage 是一个完整对话
    // 可能需要创建新会话或等待 OpenCode 支持
  }

  /**
   * 获取任务历史
   */
  async getHistory(taskId: string): Promise<TaskSessionHistory | null> {
    // 先检查内存缓存
    if (this.activeHistories.has(taskId)) {
      return this.activeHistories.get(taskId)!;
    }
    // 从文件加载
    return this.loadHistory(taskId);
  }

  /**
   * 注册事件监听
   */
  on(event: TaskSessionEventType, callback: (payload: unknown) => void): void {
    this.eventEmitter.on(event, callback);
  }

  // ============ Private Methods ============

  /**
   * 设置 Agent 事件监听
   */
  private setupAgentListeners(): void {
    // 监听 Agent 输出
    this.agentExecutor.onOutput((event: AgentOutputEvent) => {
      const taskId = this.sessionToTask.get(event.sessionId);
      if (!taskId) return;

      const history = this.activeHistories.get(taskId);
      if (!history) return;

      // 将输出转换为 assistant 消息
      // 累积输出到最后一条 assistant 消息
      const lastMessage = history.messages[history.messages.length - 1];
      if (lastMessage?.role === 'assistant') {
        // 追加到现有消息
        lastMessage.content += event.data;
      } else {
        // 创建新的 assistant 消息
        const assistantMessage: ChatMessage = {
          id: generateMessageId(),
          role: 'assistant',
          content: event.data,
          timestamp: Date.now(),
        };
        history.messages.push(assistantMessage);
        this.emit('message', { taskId, message: assistantMessage });
      }

      // 定期保存 (节流)
      this.debouncedSave(taskId, history);
    });

    // 监听 Agent 状态变更
    this.agentExecutor.onStatus((event: AgentStatusEvent) => {
      const taskId = this.sessionToTask.get(event.sessionId);
      if (!taskId) return;

      const history = this.activeHistories.get(taskId);
      if (!history) return;

      history.status = event.currentStatus;

      // 处理完成/失败
      if (event.currentStatus === 'completed') {
        history.completedAt = Date.now();
        history.stats = {
          duration: history.startedAt 
            ? history.completedAt - history.startedAt 
            : 0,
        };
        
        // 更新任务状态为 done
        this.updateTaskStatus(taskId, 'done');
        this.saveHistory(history);
        
        // 清理映射
        this.cleanupTask(taskId);
      } else if (event.currentStatus === 'failed') {
        history.completedAt = Date.now();
        history.error = event.error;
        
        // 任务失败，状态回到 todo
        this.updateTaskStatus(taskId, 'todo');
        this.saveHistory(history);
        
        // 清理映射
        this.cleanupTask(taskId);
      } else if (event.currentStatus === 'aborted') {
        history.completedAt = Date.now();
        this.saveHistory(history);
        this.cleanupTask(taskId);
      }

      this.emit('status', { taskId, status: event.currentStatus });
    });
  }

  /**
   * 更新 Kanban 任务状态
   */
  private updateTaskStatus(
    taskId: string, 
    status: KanbanTask['status'],
    sessionId?: string
  ): void {
    try {
      this.kanbanStore.moveTask(taskId, status);
      
      // 更新任务的 sessionId
      if (sessionId) {
        const state = this.kanbanStore.getState();
        const task = state.tasks[taskId];
        if (task) {
          // 直接修改任务对象 (Store 会在下次操作时持久化)
          task.sessionId = sessionId;
          task.updatedAt = Date.now();
        }
      }
    } catch (error) {
      console.error(`Failed to update task ${taskId} status:`, error);
    }
  }

  /**
   * 清理任务映射
   */
  private cleanupTask(taskId: string): void {
    const sessionId = this.taskToSession.get(taskId);
    if (sessionId) {
      this.sessionToTask.delete(sessionId);
    }
    this.taskToSession.delete(taskId);
    this.activeHistories.delete(taskId);
  }

  /**
   * 加载任务历史
   */
  private async loadHistory(taskId: string): Promise<TaskSessionHistory | null> {
    const filePath = path.join(this.tasksDir, `${taskId}.json`);
    try {
      const content = await fs.readFile(filePath, 'utf-8');
      return JSON.parse(content) as TaskSessionHistory;
    } catch {
      return null;
    }
  }

  /**
   * 保存任务历史
   */
  private async saveHistory(history: TaskSessionHistory): Promise<void> {
    const filePath = path.join(this.tasksDir, `${history.taskId}.json`);
    await fs.writeFile(filePath, JSON.stringify(history, null, 2), 'utf-8');
  }

  /**
   * 节流保存
   */
  private saveTimeouts: Map<string, NodeJS.Timeout> = new Map();
  private debouncedSave(taskId: string, history: TaskSessionHistory): void {
    const existing = this.saveTimeouts.get(taskId);
    if (existing) {
      clearTimeout(existing);
    }
    const timeout = setTimeout(() => {
      this.saveHistory(history);
      this.saveTimeouts.delete(taskId);
    }, 1000);
    this.saveTimeouts.set(taskId, timeout);
  }

  /**
   * 发送事件
   */
  private emit(event: TaskSessionEventType, payload: unknown): void {
    this.eventEmitter.emit(event, payload);
  }
}
