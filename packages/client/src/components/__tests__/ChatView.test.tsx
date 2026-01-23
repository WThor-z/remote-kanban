import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { ChatView } from '../ChatView';
import { useOpencode } from '../../hooks/useOpencode';

const mockOnData = vi.fn();

vi.mock('../../hooks/useOpencode', () => ({
  useOpencode: vi.fn(),
}));

describe('ChatView Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (useOpencode as Mock).mockReturnValue({
      onData: mockOnData,
    });
  });

  it('renders empty state when there are no messages', () => {
    mockOnData.mockImplementation(() => vi.fn());
    render(<ChatView />);

    expect(screen.getByText('No messages yet.')).toBeInTheDocument();
  });

  it('adds a message when output is received', () => {
    mockOnData.mockImplementation((callback: (data: string) => void) => {
      callback('hello chat');
      return vi.fn();
    });

    render(<ChatView />);

    expect(screen.getByText('hello chat')).toBeInTheDocument();
    expect(screen.getByText('OUTPUT')).toBeInTheDocument();
  });

  it('renders status label for status messages', () => {
    mockOnData.mockImplementation((callback: (data: string) => void) => {
      callback('STATUS: Ready');
      return vi.fn();
    });

    render(<ChatView />);

    expect(screen.getByText('STATUS')).toBeInTheDocument();
    expect(screen.getByText('Ready')).toBeInTheDocument();
  });

  it('filters messages by type', () => {
    mockOnData.mockImplementation((callback: (data: string) => void) => {
      callback('STATUS: Ready');
      callback('hello chat');
      return vi.fn();
    });

    render(<ChatView />);

    fireEvent.click(screen.getByRole('button', { name: 'Status' }));

    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.queryByText('hello chat')).not.toBeInTheDocument();
  });

  it('filters log messages by level', () => {
    mockOnData.mockImplementation((callback: (data: string) => void) => {
      callback('[ERROR] Boom');
      callback('[INFO] All good');
      return vi.fn();
    });

    render(<ChatView />);

    fireEvent.click(screen.getByRole('button', { name: 'Log' }));
    fireEvent.click(screen.getByRole('button', { name: 'Error' }));

    expect(screen.getByText('Boom')).toBeInTheDocument();
    expect(screen.queryByText('All good')).not.toBeInTheDocument();
  });

  it('groups messages by type when showing all', () => {
    mockOnData.mockImplementation((callback: (data: string) => void) => {
      callback('STATUS: Ready');
      callback('hello chat');
      return vi.fn();
    });

    render(<ChatView />);

    expect(screen.getByTestId('chat-group-status')).toBeInTheDocument();
    expect(screen.getByTestId('chat-group-output')).toBeInTheDocument();
  });
});
