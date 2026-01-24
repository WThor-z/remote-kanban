import { describe, it, expect } from 'vitest';
import { parseCommand } from '../commandParser';

describe('parseCommand', () => {
  describe('识别 Kanban 指令', () => {
    it('解析 /task add 指令', () => {
      const result = parseCommand('/task add 新任务标题');
      
      expect(result).not.toBeNull();
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ title: '新任务标题' });
    });

    it('解析 /task add 带描述的指令', () => {
      const result = parseCommand('/task add 任务标题 -- 这是描述');
      
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ 
        title: '任务标题',
        description: '这是描述',
      });
    });

    // 简洁格式测试
    it('解析 /task <title> 简洁格式', () => {
      const result = parseCommand('/task 新任务');
      
      expect(result).not.toBeNull();
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ title: '新任务' });
    });

    it('解析 /task <title> | <description> 格式', () => {
      const result = parseCommand('/task 添加功能 | 详细描述');
      
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ 
        title: '添加功能',
        description: '详细描述',
      });
    });

    it('解析 /task <title> -- <description> 格式', () => {
      const result = parseCommand('/task 修复Bug -- 这是一个严重的问题');
      
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ 
        title: '修复Bug',
        description: '这是一个严重的问题',
      });
    });

    it('解析 /todo 指令 (别名)', () => {
      const result = parseCommand('/todo 待办事项');
      
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ title: '待办事项' });
    });

    it('解析 /task move 指令', () => {
      const result = parseCommand('/task move task-123 doing');
      
      expect(result?.type).toBe('kanban:move');
      expect(result?.payload).toEqual({ 
        taskId: 'task-123',
        targetStatus: 'doing',
      });
    });

    it('解析 /task done 指令 (快捷方式)', () => {
      const result = parseCommand('/task done task-123');
      
      expect(result?.type).toBe('kanban:move');
      expect(result?.payload).toEqual({ 
        taskId: 'task-123',
        targetStatus: 'done',
      });
    });

    it('解析 /task delete 指令', () => {
      const result = parseCommand('/task delete task-123');
      
      expect(result?.type).toBe('kanban:delete');
      expect(result?.payload).toEqual({ taskId: 'task-123' });
    });

    it('解析 /task rm 指令 (别名)', () => {
      const result = parseCommand('/task rm task-456');
      
      expect(result?.type).toBe('kanban:delete');
      expect(result?.payload).toEqual({ taskId: 'task-456' });
    });
  });

  describe('非 Kanban 指令', () => {
    it('普通命令返回 null', () => {
      expect(parseCommand('ls -la')).toBeNull();
      expect(parseCommand('echo hello')).toBeNull();
      expect(parseCommand('git status')).toBeNull();
    });

    it('无效的 /task 子命令返回 null', () => {
      expect(parseCommand('/task')).toBeNull();
      expect(parseCommand('/task add')).toBeNull(); // 缺少标题
    });

    it('/task <非子命令> 会被解析为创建任务', () => {
      // /task unknown 现在会创建标题为 "unknown" 的任务
      const result = parseCommand('/task unknown');
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ title: 'unknown' });
    });

    it('无效的 /task move 返回 null', () => {
      expect(parseCommand('/task move')).toBeNull();
      expect(parseCommand('/task move task-123')).toBeNull(); // 缺少目标状态
      expect(parseCommand('/task move task-123 invalid')).toBeNull(); // 无效状态
    });
  });

  describe('边界情况', () => {
    it('处理多余空格', () => {
      const result = parseCommand('  /task   add   标题  ');
      
      expect(result?.type).toBe('kanban:create');
      expect(result?.payload).toEqual({ title: '标题' });
    });

    it('大小写不敏感', () => {
      const result = parseCommand('/TASK ADD 大写指令');
      
      expect(result?.type).toBe('kanban:create');
    });
  });
});
