import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { NeuralUiDemo } from '../NeuralUiDemo';

describe('NeuralUiDemo', () => {
  it('renders the standalone UI demo shell', () => {
    render(<NeuralUiDemo />);

    expect(screen.getByTestId('neural-ui-demo')).toBeInTheDocument();
    expect(screen.getByText('Neural UI Demo')).toBeInTheDocument();
    expect(screen.getByText('Gateway Diagnostics')).toBeInTheDocument();
    expect(screen.getByText('Directive 指令流')).toBeInTheDocument();
  });

  it('adds a mock task when pressing inject button', () => {
    render(<NeuralUiDemo />);

    const before = screen.getAllByTestId('task-card').length;
    fireEvent.click(screen.getByRole('button', { name: /inject mock task/i }));
    const after = screen.getAllByTestId('task-card').length;

    expect(after).toBe(before + 1);
  });

  it('supports switching to lab-light skin', () => {
    render(<NeuralUiDemo />);

    const root = screen.getByTestId('neural-ui-demo');
    expect(root).not.toHaveClass('console-root--lab');

    fireEvent.click(screen.getByRole('button', { name: /switch to lab-light/i }));
    expect(root).toHaveClass('console-root--lab');
  });
});
