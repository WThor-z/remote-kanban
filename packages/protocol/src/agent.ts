/**
 * Agent Protocol Types
 * 
 * 定义 AI 编码代理的类型和事件协议
 */

// ============ Agent Configuration ============

/** 支持的 Agent 类型 */
export type AgentType = 
  | 'opencode'      // SST OpenCode
  | 'claude-code'   // Anthropic Claude Code
  | 'codex'         // OpenAI Codex
  | 'gemini-cli'    // Google Gemini CLI
  | 'custom';       // 自定义 Agent

/** Agent 配置 */
export interface AgentConfig {
  type: AgentType;
  name: string;
  /** CLI 命令，如 'npx opencode-ai' */
  command: string;
  /** 命令参数模板，{prompt} 会被替换 */
  args: string[];
  /** 环境变量 */
  env?: Record<string, string>;
  /** 工作目录 */
  cwd?: string;
}

/** 预设的 Agent 配置 */
export const AGENT_PRESETS: Record<AgentType, Omit<AgentConfig, 'cwd'>> = {
  opencode: {
    type: 'opencode',
    name: 'OpenCode',
    command: 'opencode',
    args: ['run', '{prompt}'],
  },
  'claude-code': {
    type: 'claude-code',
    name: 'Claude Code',
    command: 'claude',
    args: ['--print', '{prompt}'],
  },
  codex: {
    type: 'codex',
    name: 'OpenAI Codex',
    command: 'codex',
    args: ['{prompt}'],
  },
  'gemini-cli': {
    type: 'gemini-cli',
    name: 'Gemini CLI',
    command: 'gemini',
    args: ['{prompt}'],
  },
  custom: {
    type: 'custom',
    name: 'Custom Agent',
    command: '',
    args: [],
  },
};

// ============ Agent Session ============

/** Agent 会话状态 */
export type AgentSessionStatus = 
  | 'idle'        // 空闲
  | 'starting'    // 启动中
  | 'running'     // 运行中
  | 'paused'      // 暂停
  | 'completed'   // 完成
  | 'failed'      // 失败
  | 'aborted';    // 中止

/** Agent 会话 */
export interface AgentSession {
  id: string;
  agentType: AgentType;
  status: AgentSessionStatus;
  /** 关联的 Kanban 任务 ID */
  taskId?: string;
  /** 提示词 */
  prompt: string;
  /** 开始时间 */
  startedAt?: number;
  /** 结束时间 */
  endedAt?: number;
  /** 进程 ID */
  pid?: number;
  /** 错误信息 */
  error?: string;
}

/** 创建新会话 */
export const createAgentSession = (
  agentType: AgentType,
  prompt: string,
  taskId?: string
): AgentSession => ({
  id: `agent-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
  agentType,
  status: 'idle',
  taskId,
  prompt,
});

// ============ Agent Events ============

/** Agent 输出类型 */
export type AgentOutputType = 
  | 'stdout'      // 标准输出
  | 'stderr'      // 错误输出
  | 'system';     // 系统消息

/** Agent 输出事件 */
export interface AgentOutputEvent {
  sessionId: string;
  type: AgentOutputType;
  data: string;
  timestamp: number;
}

/** Agent 状态变更事件 */
export interface AgentStatusEvent {
  sessionId: string;
  previousStatus: AgentSessionStatus;
  currentStatus: AgentSessionStatus;
  timestamp: number;
  error?: string;
}

/** Agent 任务检测事件 (从输出解析) */
export interface AgentTaskDetectedEvent {
  sessionId: string;
  action: 'create' | 'start' | 'complete' | 'fail';
  taskTitle?: string;
  taskId?: string;
  timestamp: number;
}

// ============ Socket Events ============

/** Client → Server 事件 */
export interface AgentClientEvents {
  'agent:start': (payload: { agentType: AgentType; prompt: string; taskId?: string }) => void;
  'agent:stop': (payload: { sessionId: string }) => void;
  'agent:input': (payload: { sessionId: string; data: string }) => void;
  'agent:list': () => void;
}

/** Server → Client 事件 */
export interface AgentServerEvents {
  'agent:output': (event: AgentOutputEvent) => void;
  'agent:status': (event: AgentStatusEvent) => void;
  'agent:session': (session: AgentSession) => void;
  'agent:sessions': (sessions: AgentSession[]) => void;
  'agent:task-detected': (event: AgentTaskDetectedEvent) => void;
  'agent:error': (error: { sessionId?: string; message: string }) => void;
}

// ============ Utility Functions ============

/** 检查会话是否活跃 */
export const isSessionActive = (session: AgentSession): boolean => {
  return ['starting', 'running', 'paused'].includes(session.status);
};

/** 检查会话是否可启动 */
export const canStartSession = (session: AgentSession): boolean => {
  return session.status === 'idle';
};

/** 检查会话是否可停止 */
export const canStopSession = (session: AgentSession): boolean => {
  return isSessionActive(session);
};

/** 格式化会话持续时间 */
export const formatSessionDuration = (session: AgentSession): string => {
  if (!session.startedAt) return '0s';
  
  const endTime = session.endedAt || Date.now();
  const duration = Math.floor((endTime - session.startedAt) / 1000);
  
  if (duration < 60) return `${duration}s`;
  if (duration < 3600) return `${Math.floor(duration / 60)}m ${duration % 60}s`;
  
  const hours = Math.floor(duration / 3600);
  const mins = Math.floor((duration % 3600) / 60);
  return `${hours}h ${mins}m`;
};

// ============ Task Session History (持久化) ============

/** 消息角色 */
export type MessageRole = 'user' | 'assistant' | 'system';

/** 对话消息 */
export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
  /** 消息类型：text, tool_use, tool_result, thinking 等 */
  type?: string;
  /** 工具调用信息 */
  toolCall?: {
    name: string;
    input: Record<string, unknown>;
  };
  /** 工具结果 */
  toolResult?: {
    success: boolean;
    output: string;
  };
}

/** 任务会话历史 - 存储在 .opencode/tasks/{taskId}.json */
export interface TaskSessionHistory {
  /** 任务 ID */
  taskId: string;
  /** 任务标题 */
  title: string;
  /** 任务描述 (初始 Prompt) */
  description: string;
  /** 会话 ID */
  sessionId: string;
  /** 会话状态 */
  status: AgentSessionStatus;
  /** 对话历史 */
  messages: ChatMessage[];
  /** 创建时间 */
  createdAt: number;
  /** 开始执行时间 */
  startedAt?: number;
  /** 完成时间 */
  completedAt?: number;
  /** 错误信息 */
  error?: string;
  /** 统计信息 */
  stats?: {
    totalTokens?: number;
    duration?: number;
    toolCalls?: number;
  };
}

/** 创建空的任务会话历史 */
export const createTaskSessionHistory = (
  taskId: string,
  title: string,
  description: string
): TaskSessionHistory => ({
  taskId,
  title,
  description,
  sessionId: `session-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
  status: 'idle',
  messages: [],
  createdAt: Date.now(),
});

/** 生成消息 ID */
export const generateMessageId = (): string => {
  return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
};

// ============ Task Session Events ============

/** 任务会话事件类型 */
export type TaskSessionEventType =
  | 'task:execute'      // 开始执行任务
  | 'task:message'      // 新消息
  | 'task:complete'     // 任务完成
  | 'task:fail'         // 任务失败
  | 'task:history';     // 获取历史

/** Client → Server: 任务会话事件 */
export interface TaskSessionClientEvents {
  /** 开始执行任务 */
  'task:execute': (payload: { taskId: string }) => void;
  /** 发送用户消息 */
  'task:message': (payload: { taskId: string; content: string }) => void;
  /** 停止任务执行 */
  'task:stop': (payload: { taskId: string }) => void;
  /** 获取任务历史 */
  'task:history': (payload: { taskId: string }) => void;
}

/** Server → Client: 任务会话事件 */
export interface TaskSessionServerEvents {
  /** 任务状态变更 */
  'task:status': (payload: { taskId: string; status: AgentSessionStatus }) => void;
  /** 新消息 */
  'task:message': (payload: { taskId: string; message: ChatMessage }) => void;
  /** 任务历史 */
  'task:history': (payload: TaskSessionHistory) => void;
  /** 任务错误 */
  'task:error': (payload: { taskId: string; error: string }) => void;
}
