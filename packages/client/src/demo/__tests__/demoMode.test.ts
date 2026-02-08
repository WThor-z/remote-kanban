import { describe, expect, it } from 'vitest';
import { shouldUseUiDemo } from '../demoMode';

describe('shouldUseUiDemo', () => {
  it('returns true for /ui-demo pathname', () => {
    expect(shouldUseUiDemo('/ui-demo', '')).toBe(true);
  });

  it('returns true for ?demo=ui query', () => {
    expect(shouldUseUiDemo('/', '?demo=ui')).toBe(true);
  });

  it('returns false for regular app paths', () => {
    expect(shouldUseUiDemo('/', '')).toBe(false);
    expect(shouldUseUiDemo('/tasks', '?tab=1')).toBe(false);
  });
});
