/**
 * Agent Output Parser
 * 
 * 从 AI 编码代理的 CLI 输出中提取任务状态信息
 */

// ============ Types ============

/** 解析结果 */
export interface ParseResult {
  taskDetected?: {
    action: 'create' | 'start' | 'complete' | 'fail';
    taskTitle?: string;
  };
  isThinking?: boolean;
  isExecuting?: boolean;
  progress?: number; // 0-100
}

// ============ Patterns ============

/** OpenCode 输出模式 */
const OPENCODE_PATTERNS = {
  thinking: /⏳\s*Thinking/i,
  executing: /🔧\s*Running\s+tool:/i,
  complete: /✅\s*Task\s+completed/i,
  fail: /❌\s*(Error|Failed)/i,
  create: /📋\s*Creating\s+task:\s*(.+)/i,
  start: /🚀\s*Starting:\s*(.+)/i,
};

/** 通用模式 (其他 Agent) */
const GENERIC_PATTERNS = {
  create: /\[TASK\]\s*Creating:\s*(.+)/i,
  start: /\[TASK\]\s*Starting:\s*(.+)/i,
  complete: /\[TASK\]\s*Complete/i,
  fail: /\[ERROR\]/i,
  progress: /progress:\s*(\d+)%/i,
};

// ============ Utility Functions ============

/**
 * 移除 ANSI 转义序列
 */
const stripAnsi = (str: string): string => {
  // 匹配 ANSI 转义序列: ESC[ ... m 格式
  return str.replace(/\x1b\[[0-9;]*m/g, '');
};

/**
 * 检查行是否为空或仅包含空白
 */
const isEmptyLine = (line: string): boolean => {
  return line.trim().length === 0;
};

// ============ Parser Class ============

/**
 * Agent 输出解析器
 * 
 * 用于从 AI 编码代理的 CLI 输出中提取任务状态信息
 */
export class AgentOutputParser {
  /**
   * 解析单行输出
   */
  parseLine(line: string): ParseResult {
    const result: ParseResult = {};
    
    // 移除 ANSI 转义序列
    const cleanLine = stripAnsi(line);
    
    // 空行返回空结果
    if (isEmptyLine(cleanLine)) {
      return result;
    }

    // 1. 检测任务操作 (优先级最高)
    const taskAction = this.parseTaskAction(cleanLine);
    if (taskAction) {
      result.taskDetected = taskAction;
      // 任务操作可能同时有进度
      const progress = this.parseProgress(cleanLine);
      if (progress !== undefined) {
        result.progress = progress;
      }
      return result;
    }

    // 2. 检测状态
    if (this.isThinking(cleanLine)) {
      result.isThinking = true;
    }

    if (this.isExecuting(cleanLine)) {
      result.isExecuting = true;
    }

    // 3. 检测进度
    const progress = this.parseProgress(cleanLine);
    if (progress !== undefined) {
      result.progress = progress;
    }

    return result;
  }

  /**
   * 解析多行输出（累积状态）
   */
  parseChunk(chunk: string): ParseResult[] {
    if (!chunk) {
      return [];
    }

    // 支持 Unix (LF) 和 Windows (CRLF) 换行符
    const lines = chunk.split(/\r?\n/);
    
    const results: ParseResult[] = [];
    
    for (const line of lines) {
      const result = this.parseLine(line);
      // 只添加非空结果
      if (Object.keys(result).length > 0) {
        results.push(result);
      }
    }
    
    return results;
  }

  /**
   * 重置解析器状态
   */
  reset(): void {
    // 当前实现是无状态的，所以 reset 是空操作
    // 保留此方法以便将来扩展有状态解析
  }

  // ============ Private Methods ============

  /**
   * 解析任务操作
   */
  private parseTaskAction(line: string): ParseResult['taskDetected'] | null {
    // OpenCode 模式 - 创建任务
    let match = line.match(OPENCODE_PATTERNS.create);
    if (match) {
      return { action: 'create', taskTitle: match[1].trim() };
    }

    // OpenCode 模式 - 开始任务
    match = line.match(OPENCODE_PATTERNS.start);
    if (match) {
      return { action: 'start', taskTitle: match[1].trim() };
    }

    // OpenCode 模式 - 完成任务
    if (OPENCODE_PATTERNS.complete.test(line)) {
      return { action: 'complete' };
    }

    // OpenCode 模式 - 失败
    if (OPENCODE_PATTERNS.fail.test(line)) {
      return { action: 'fail' };
    }

    // 通用模式 - 创建任务
    match = line.match(GENERIC_PATTERNS.create);
    if (match) {
      return { action: 'create', taskTitle: match[1].trim() };
    }

    // 通用模式 - 开始任务
    match = line.match(GENERIC_PATTERNS.start);
    if (match) {
      return { action: 'start', taskTitle: match[1].trim() };
    }

    // 通用模式 - 完成任务
    if (GENERIC_PATTERNS.complete.test(line)) {
      return { action: 'complete' };
    }

    // 通用模式 - 失败
    if (GENERIC_PATTERNS.fail.test(line)) {
      return { action: 'fail' };
    }

    return null;
  }

  /**
   * 检测是否正在思考
   */
  private isThinking(line: string): boolean {
    return OPENCODE_PATTERNS.thinking.test(line);
  }

  /**
   * 检测是否正在执行工具
   */
  private isExecuting(line: string): boolean {
    return OPENCODE_PATTERNS.executing.test(line);
  }

  /**
   * 解析进度百分比
   */
  private parseProgress(line: string): number | undefined {
    const match = line.match(GENERIC_PATTERNS.progress);
    if (match) {
      const value = parseInt(match[1], 10);
      // 确保在有效范围内
      if (value >= 0 && value <= 100) {
        return value;
      }
    }
    return undefined;
  }
}

// ============ Exports ============

export default AgentOutputParser;
