import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { InputBar } from '../InputBar';
import { useOpencode } from '../../hooks/useOpencode';
import { useKanban } from '../../hooks/useKanban';

const mockCreateTask = vi.fn();
const mockMoveTask = vi.fn();
const mockDeleteTask = vi.fn();

vi.mock('../../hooks/useOpencode', () => ({
  useOpencode: vi.fn(),
}));

vi.mock('../../hooks/useKanban', () => ({
  useKanban: vi.fn(),
}));

describe('InputBar Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (useOpencode as Mock).mockReturnValue({
      isConnected: true,
    });
    (useKanban as Mock).mockReturnValue({
      createTask: mockCreateTask,
      moveTask: mockMoveTask,
      deleteTask: mockDeleteTask,
    });
  });

  it('clears input after submission', () => {
    render(<InputBar />);

    const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
    const form = screen.getByTestId('input-bar-form');

    fireEvent.change(input, { target: { value: '/task add test' } });
    fireEvent.submit(form);

    expect(input.value).toBe('');
  });

  it('disables sending when disconnected', () => {
    (useOpencode as Mock).mockReturnValue({
      isConnected: false,
    });

    render(<InputBar />);

    const button = screen.getByTestId('input-bar-submit');
    expect(button).toBeDisabled();
  });

  it('does nothing for non-kanban commands', () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    
    render(<InputBar />);

    const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
    const form = screen.getByTestId('input-bar-form');

    fireEvent.change(input, { target: { value: 'ls' } });
    fireEvent.submit(form);

    // Should log a message about using AI Agent panel
    expect(consoleSpy).toHaveBeenCalled();
    expect(mockCreateTask).not.toHaveBeenCalled();
    expect(input.value).toBe('');
    
    consoleSpy.mockRestore();
  });

  describe('Kanban 指令拦截', () => {
    it('/task add 指令调用 createTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/task add 新任务' } });
      fireEvent.submit(form);

      expect(mockCreateTask).toHaveBeenCalledWith('新任务', undefined);
      expect(input.value).toBe('');
    });

    it('/todo 指令调用 createTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/todo 待办事项' } });
      fireEvent.submit(form);

      expect(mockCreateTask).toHaveBeenCalledWith('待办事项', undefined);
    });

    it('/task move 指令调用 moveTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/task move task-123 doing' } });
      fireEvent.submit(form);

      expect(mockMoveTask).toHaveBeenCalledWith('task-123', 'doing', undefined);
    });

    it('/task delete 指令调用 deleteTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/task delete task-456' } });
      fireEvent.submit(form);

      expect(mockDeleteTask).toHaveBeenCalledWith('task-456');
    });
  });
});
