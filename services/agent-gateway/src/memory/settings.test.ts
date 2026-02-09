import { describe, expect, it } from 'vitest';
import {
  clampMemorySettings,
  defaultMemorySettings,
  memorySettingsFromEnv,
  mergeMemorySettings,
} from './settings.js';

describe('memory settings helpers', () => {
  it('parses env overrides', () => {
    const settings = memorySettingsFromEnv({
      MEMORY_ENABLE: 'false',
      MEMORY_INJECTION_TOKEN_BUDGET: '2048',
      MEMORY_RETRIEVAL_TOP_K: '12',
      MEMORY_LLM_EXTRACT_ENABLE: 'false',
    });

    expect(settings.enabled).toBe(false);
    expect(settings.tokenBudget).toBe(2048);
    expect(settings.retrievalTopK).toBe(12);
    expect(settings.llmExtractEnabled).toBe(false);
  });

  it('merges partial patch without losing base defaults', () => {
    const merged = mergeMemorySettings(defaultMemorySettings(), {
      enabled: false,
      tokenBudget: 3000,
    });

    expect(merged.enabled).toBe(false);
    expect(merged.tokenBudget).toBe(3000);
    expect(merged.promptInjection).toBe(true);
  });

  it('clamps numeric values', () => {
    const clamped = clampMemorySettings({
      ...defaultMemorySettings(),
      tokenBudget: 99999,
      retrievalTopK: 0,
    });

    expect(clamped.tokenBudget).toBe(6000);
    expect(clamped.retrievalTopK).toBe(1);
  });
});
