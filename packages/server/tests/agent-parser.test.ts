import { describe, it, expect, beforeEach } from 'vitest';
import { AgentOutputParser, ParseResult } from '../src/agent/parser';

describe('AgentOutputParser', () => {
  let parser: AgentOutputParser;

  beforeEach(() => {
    parser = new AgentOutputParser();
  });

  describe('parseLine - OpenCode patterns', () => {
    it('检测 thinking 状态', () => {
      const result = parser.parseLine('⏳ Thinking...');
      expect(result.isThinking).toBe(true);
    });

    it('检测 executing 状态 (工具调用)', () => {
      const result = parser.parseLine('🔧 Running tool: read_file');
      expect(result.isExecuting).toBe(true);
    });

    it('检测任务完成', () => {
      const result = parser.parseLine('✅ Task completed');
      expect(result.taskDetected?.action).toBe('complete');
    });

    it('检测任务失败/错误', () => {
      const result = parser.parseLine('❌ Error: something went wrong');
      expect(result.taskDetected?.action).toBe('fail');
    });

    it('检测创建任务', () => {
      const result = parser.parseLine('📋 Creating task: Implement user authentication');
      expect(result.taskDetected?.action).toBe('create');
      expect(result.taskDetected?.taskTitle).toBe('Implement user authentication');
    });

    it('检测开始任务', () => {
      const result = parser.parseLine('🚀 Starting: Fix login bug');
      expect(result.taskDetected?.action).toBe('start');
      expect(result.taskDetected?.taskTitle).toBe('Fix login bug');
    });

    it('带 ANSI 转义序列的行', () => {
      const result = parser.parseLine('\x1b[32m✅ Task completed\x1b[0m');
      expect(result.taskDetected?.action).toBe('complete');
    });
  });

  describe('parseLine - 通用模式', () => {
    it('检测 [TASK] Creating 模式', () => {
      const result = parser.parseLine('[TASK] Creating: Build API endpoint');
      expect(result.taskDetected?.action).toBe('create');
      expect(result.taskDetected?.taskTitle).toBe('Build API endpoint');
    });

    it('检测 [TASK] Complete 模式', () => {
      const result = parser.parseLine('[TASK] Complete');
      expect(result.taskDetected?.action).toBe('complete');
    });

    it('检测 [TASK] Starting 模式', () => {
      const result = parser.parseLine('[TASK] Starting: Refactor code');
      expect(result.taskDetected?.action).toBe('start');
      expect(result.taskDetected?.taskTitle).toBe('Refactor code');
    });

    it('检测 [ERROR] 模式', () => {
      const result = parser.parseLine('[ERROR] Failed to compile');
      expect(result.taskDetected?.action).toBe('fail');
    });

    it('检测 Progress 百分比', () => {
      const result = parser.parseLine('Progress: 50%');
      expect(result.progress).toBe(50);
    });

    it('检测各种进度格式', () => {
      expect(parser.parseLine('Progress: 0%').progress).toBe(0);
      expect(parser.parseLine('Progress: 100%').progress).toBe(100);
      expect(parser.parseLine('progress: 75%').progress).toBe(75);
    });
  });

  describe('parseLine - 边缘情况', () => {
    it('空行返回空结果', () => {
      const result = parser.parseLine('');
      expect(result).toEqual({});
    });

    it('纯空白行返回空结果', () => {
      const result = parser.parseLine('   \t  ');
      expect(result).toEqual({});
    });

    it('普通输出行返回空结果', () => {
      const result = parser.parseLine('Installing dependencies...');
      expect(result).toEqual({});
    });

    it('处理乱码/二进制数据不崩溃', () => {
      const result = parser.parseLine('\x00\x01\x02\xFF\xFE');
      expect(result).toBeDefined();
    });

    it('处理超长行', () => {
      const longLine = 'a'.repeat(10000);
      const result = parser.parseLine(longLine);
      expect(result).toBeDefined();
    });

    it('处理包含特殊字符的任务标题', () => {
      const result = parser.parseLine('📋 Creating task: Fix bug #123 (urgent!)');
      expect(result.taskDetected?.taskTitle).toBe('Fix bug #123 (urgent!)');
    });
  });

  describe('parseChunk - 多行解析', () => {
    it('解析多行输出', () => {
      const chunk = `⏳ Thinking...
🔧 Running tool: read_file
✅ Task completed`;
      
      const results = parser.parseChunk(chunk);
      
      expect(results).toHaveLength(3);
      expect(results[0].isThinking).toBe(true);
      expect(results[1].isExecuting).toBe(true);
      expect(results[2].taskDetected?.action).toBe('complete');
    });

    it('处理 Windows 换行符 (CRLF)', () => {
      const chunk = '⏳ Thinking...\r\n✅ Task completed\r\n';
      
      const results = parser.parseChunk(chunk);
      
      expect(results).toHaveLength(2);
      expect(results[0].isThinking).toBe(true);
      expect(results[1].taskDetected?.action).toBe('complete');
    });

    it('过滤空行结果', () => {
      const chunk = `⏳ Thinking...

✅ Task completed

`;
      
      const results = parser.parseChunk(chunk);
      
      expect(results).toHaveLength(2);
    });

    it('处理空 chunk', () => {
      const results = parser.parseChunk('');
      expect(results).toEqual([]);
    });

    it('累积解析多个 chunk', () => {
      const chunk1 = '📋 Creating task: Task 1\n';
      const chunk2 = '🚀 Starting: Task 1\n';
      const chunk3 = '✅ Task completed\n';
      
      const results1 = parser.parseChunk(chunk1);
      const results2 = parser.parseChunk(chunk2);
      const results3 = parser.parseChunk(chunk3);
      
      expect(results1[0].taskDetected?.action).toBe('create');
      expect(results2[0].taskDetected?.action).toBe('start');
      expect(results3[0].taskDetected?.action).toBe('complete');
    });
  });

  describe('reset', () => {
    it('重置解析器状态', () => {
      // 解析一些内容
      parser.parseChunk('⏳ Thinking...');
      
      // 重置
      parser.reset();
      
      // 应该能正常继续使用
      const result = parser.parseLine('✅ Task completed');
      expect(result.taskDetected?.action).toBe('complete');
    });
  });

  describe('复杂场景', () => {
    it('混合 OpenCode 和通用模式', () => {
      const chunk = `[TASK] Creating: Setup project
⏳ Thinking...
Progress: 25%
🔧 Running tool: write_file
Progress: 75%
✅ Task completed`;
      
      const results = parser.parseChunk(chunk);
      
      expect(results[0].taskDetected?.action).toBe('create');
      expect(results[1].isThinking).toBe(true);
      expect(results[2].progress).toBe(25);
      expect(results[3].isExecuting).toBe(true);
      expect(results[4].progress).toBe(75);
      expect(results[5].taskDetected?.action).toBe('complete');
    });

    it('一行中包含多个模式时优先匹配任务操作', () => {
      // 理论上不太可能，但测试优先级
      const result = parser.parseLine('✅ Task completed Progress: 100%');
      // 任务完成应该被优先匹配
      expect(result.taskDetected?.action).toBe('complete');
    });
  });
});
