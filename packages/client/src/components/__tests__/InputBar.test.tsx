import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { InputBar } from '../InputBar';
import { useOpencode } from '../../hooks/useOpencode';
import { useKanban } from '../../hooks/useKanban';

const mockWrite = vi.fn();
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
      write: mockWrite,
    });
    (useKanban as Mock).mockReturnValue({
      createTask: mockCreateTask,
      moveTask: mockMoveTask,
      deleteTask: mockDeleteTask,
    });
  });

  it('sends input with a carriage return', () => {
    render(<InputBar />);

    const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
    const form = screen.getByTestId('input-bar-form');

    fireEvent.change(input, { target: { value: 'ls' } });
    fireEvent.submit(form);

    expect(mockWrite).toHaveBeenCalledWith('ls\r');
    expect(input.value).toBe('');
  });

  it('disables sending when disconnected', () => {
    (useOpencode as Mock).mockReturnValue({
      isConnected: false,
      write: mockWrite,
    });

    render(<InputBar />);

    const button = screen.getByTestId('input-bar-submit');
    expect(button).toBeDisabled();

    fireEvent.submit(screen.getByTestId('input-bar-form'));
    expect(mockWrite).not.toHaveBeenCalled();
  });

  describe('Kanban 指令拦截', () => {
    it('/task add 指令调用 createTask 而非 write', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/task add 新任务' } });
      fireEvent.submit(form);

      expect(mockCreateTask).toHaveBeenCalledWith('新任务', undefined);
      expect(mockWrite).not.toHaveBeenCalled();
      expect(input.value).toBe('');
    });

    it('/todo 指令调用 createTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/todo 待办事项' } });
      fireEvent.submit(form);

      expect(mockCreateTask).toHaveBeenCalledWith('待办事项', undefined);
      expect(mockWrite).not.toHaveBeenCalled();
    });

    it('/task move 指令调用 moveTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/task move task-123 doing' } });
      fireEvent.submit(form);

      expect(mockMoveTask).toHaveBeenCalledWith('task-123', 'doing', undefined);
      expect(mockWrite).not.toHaveBeenCalled();
    });

    it('/task delete 指令调用 deleteTask', () => {
      render(<InputBar />);

      const input = screen.getByTestId('input-bar-input') as HTMLInputElement;
      const form = screen.getByTestId('input-bar-form');

      fireEvent.change(input, { target: { value: '/task delete task-456' } });
      fireEvent.submit(form);

      expect(mockDeleteTask).toHaveBeenCalledWith('task-456');
      expect(mockWrite).not.toHaveBeenCalled();
    });
  });
});
