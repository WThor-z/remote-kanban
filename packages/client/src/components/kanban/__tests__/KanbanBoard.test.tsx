import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { KanbanBoard } from '../KanbanBoard';
import type { KanbanBoardState } from '@opencode-vibe/protocol';

describe('KanbanBoard', () => {
  const mockBoardState: KanbanBoardState = {
    tasks: {
      'task-1': {
        id: 'task-1',
        title: '待办任务',
        status: 'todo',
        createdAt: Date.now(),
      },
      'task-2': {
        id: 'task-2',
        title: '进行中任务',
        status: 'doing',
        createdAt: Date.now(),
      },
      'task-3': {
        id: 'task-3',
        title: '已完成任务',
        status: 'done',
        createdAt: Date.now(),
      },
    },
    columns: {
      todo: { id: 'todo', title: 'To Do', taskIds: ['task-1'] },
      doing: { id: 'doing', title: 'Doing', taskIds: ['task-2'] },
      done: { id: 'done', title: 'Done', taskIds: ['task-3'] },
    },
    columnOrder: ['todo', 'doing', 'done'],
  };

  it('渲染三列看板', () => {
    render(
      <KanbanBoard
        board={mockBoardState}
        onMoveTask={vi.fn()}
        onDeleteTask={vi.fn()}
      />
    );

    expect(screen.getByText('To Do')).toBeInTheDocument();
    expect(screen.getByText('Doing')).toBeInTheDocument();
    expect(screen.getByText('Done')).toBeInTheDocument();
    expect(screen.getByTestId('kanban-grid')).toHaveClass('kanban-grid');
  });

  it('在对应列中渲染任务', () => {
    render(
      <KanbanBoard
        board={mockBoardState}
        onMoveTask={vi.fn()}
        onDeleteTask={vi.fn()}
      />
    );

    expect(screen.getByText('待办任务')).toBeInTheDocument();
    expect(screen.getByText('进行中任务')).toBeInTheDocument();
    expect(screen.getByText('已完成任务')).toBeInTheDocument();
  });

  it('显示每列的任务数量', () => {
    render(
      <KanbanBoard
        board={mockBoardState}
        onMoveTask={vi.fn()}
        onDeleteTask={vi.fn()}
      />
    );

    // 每列都有 1 个任务
    const badges = screen.getAllByTestId('column-count');
    expect(badges).toHaveLength(3);
    badges.forEach((badge) => {
      expect(badge).toHaveTextContent('1');
    });
  });

  it('空看板时显示空状态提示', () => {
    const emptyBoard: KanbanBoardState = {
      tasks: {},
      columns: {
        todo: { id: 'todo', title: 'To Do', taskIds: [] },
        doing: { id: 'doing', title: 'Doing', taskIds: [] },
        done: { id: 'done', title: 'Done', taskIds: [] },
      },
      columnOrder: ['todo', 'doing', 'done'],
    };

    render(
      <KanbanBoard
        board={emptyBoard}
        onMoveTask={vi.fn()}
        onDeleteTask={vi.fn()}
      />
    );

    expect(screen.getAllByText('暂无任务')).toHaveLength(3);
  });
});
