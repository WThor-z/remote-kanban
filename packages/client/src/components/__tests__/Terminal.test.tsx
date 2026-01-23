import { render, screen, waitFor } from '@testing-library/react';
import { Terminal } from '../Terminal';
import { useOpencode } from '../../hooks/useOpencode';
import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';

// Mock mocks
const mockTermWrite = vi.fn();
const mockTermOnData = vi.fn();
const mockTermDispose = vi.fn();
const mockTermOpen = vi.fn();
const mockTermLoadAddon = vi.fn();
const mockFit = vi.fn();

// Mock xterm
vi.mock('xterm', () => {
  return {
    Terminal: class {
      open = mockTermOpen;
      loadAddon = mockTermLoadAddon;
      write = mockTermWrite;
      onData = mockTermOnData;
      dispose = mockTermDispose;
      options = {};
    },
  };
});

// Mock xterm-addon-fit
vi.mock('xterm-addon-fit', () => {
  return {
    FitAddon: class {
      fit = mockFit;
      dispose = vi.fn();
    },
  };
});

// Mock useOpencode
const mockHookWrite = vi.fn();
const mockHookOnData = vi.fn();

vi.mock('../../hooks/useOpencode', () => ({
  useOpencode: vi.fn(),
}));

describe('Terminal Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (useOpencode as Mock).mockReturnValue({
      isConnected: true,
      write: mockHookWrite,
      onData: mockHookOnData,
    });
  });

  it('renders terminal container', () => {
    render(<Terminal />);
    expect(screen.getByTestId('terminal-container')).toBeInTheDocument();
  });

  it('initializes xterm on mount', async () => {
    render(<Terminal />);
    expect(mockTermOpen).toHaveBeenCalled();
    expect(mockTermLoadAddon).toHaveBeenCalled();
    await waitFor(() => {
      expect(mockFit).toHaveBeenCalled();
    });
  });

  it('writes to terminal when receiving data from server', () => {
    // Setup the mock to simulate receiving data immediately upon subscription
    mockHookOnData.mockImplementation((callback: (data: string) => void) => {
      callback('test output');
      return vi.fn(); // cleanup
    });

    render(<Terminal />);
    
    expect(mockHookOnData).toHaveBeenCalled();
    expect(mockTermWrite).toHaveBeenCalledWith('test output');
  });

  it('sends input to server when typing in terminal', () => {
    render(<Terminal />);
    
    // Simulate terminal input
    // term.onData(callback) is called in useEffect
    expect(mockTermOnData).toHaveBeenCalled();
    const onDataCallback = mockTermOnData.mock.calls[0][0]; 
    
    onDataCallback('test input');

    expect(mockHookWrite).toHaveBeenCalledWith('test input');
  });
});
