import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TaskCard } from '../TaskCard';
import type { KanbanTask } from '@opencode-vibe/protocol';

describe('TaskCard', () => {
  const mockTask: KanbanTask = {
    id: 'task-1',
    title: '测试任务',
    status: 'todo',
    description: '这是描述',
    createdAt: Date.now(),
  };

  it('渲染任务标题', () => {
    render(<TaskCard task={mockTask} />);
    expect(screen.getByText('测试任务')).toBeInTheDocument();
  });

  it('渲染任务描述', () => {
    render(<TaskCard task={mockTask} />);
    expect(screen.getByText('这是描述')).toBeInTheDocument();
  });

  it('无描述时不渲染描述区域', () => {
    const taskWithoutDesc = { ...mockTask, description: undefined };
    render(<TaskCard task={taskWithoutDesc} />);
    expect(screen.queryByTestId('task-description')).not.toBeInTheDocument();
  });

  it('点击删除按钮触发 onDelete', () => {
    const onDelete = vi.fn();
    render(<TaskCard task={mockTask} onDelete={onDelete} />);
    
    const deleteButton = screen.getByRole('button', { name: /删除/i });
    fireEvent.click(deleteButton);
    
    expect(onDelete).toHaveBeenCalledWith('task-1');
  });

  it('应用正确的状态颜色', () => {
    const { rerender } = render(<TaskCard task={mockTask} />);
    expect(screen.getByTestId('task-card')).toHaveClass('border-l-slate-400');

    rerender(<TaskCard task={{ ...mockTask, status: 'doing' }} />);
    expect(screen.getByTestId('task-card')).toHaveClass('border-l-amber-400');

    rerender(<TaskCard task={{ ...mockTask, status: 'done' }} />);
    expect(screen.getByTestId('task-card')).toHaveClass('border-l-emerald-400');
  });
});
