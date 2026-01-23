import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { InputBar } from '../InputBar';
import { useOpencode } from '../../hooks/useOpencode';

const mockWrite = vi.fn();

vi.mock('../../hooks/useOpencode', () => ({
  useOpencode: vi.fn(),
}));

describe('InputBar Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (useOpencode as Mock).mockReturnValue({
      isConnected: true,
      write: mockWrite,
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
});
