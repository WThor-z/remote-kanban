import type { MemorySettings } from '../types.js';

const DEFAULT_TOKEN_BUDGET = 1200;
const DEFAULT_TOP_K = 8;

const parseBoolean = (raw: string | undefined, fallback: boolean): boolean => {
  if (raw == null || raw.trim() === '') {
    return fallback;
  }
  const normalized = raw.trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(normalized)) {
    return true;
  }
  if (['0', 'false', 'no', 'off'].includes(normalized)) {
    return false;
  }
  return fallback;
};

const parsePositiveInt = (raw: string | undefined, fallback: number): number => {
  if (!raw) {
    return fallback;
  }
  const value = Number.parseInt(raw, 10);
  if (!Number.isFinite(value) || value <= 0) {
    return fallback;
  }
  return value;
};

export const defaultMemorySettings = (): MemorySettings => ({
  enabled: true,
  gatewayStoreEnabled: true,
  rustStoreEnabled: true,
  autoWrite: true,
  promptInjection: true,
  tokenBudget: DEFAULT_TOKEN_BUDGET,
  retrievalTopK: DEFAULT_TOP_K,
  llmExtractEnabled: true,
});

export const memorySettingsFromEnv = (
  env: Record<string, string | undefined> = process.env
): MemorySettings => {
  const defaults = defaultMemorySettings();
  return {
    enabled: parseBoolean(env.MEMORY_ENABLE, defaults.enabled),
    gatewayStoreEnabled: parseBoolean(
      env.MEMORY_GATEWAY_STORE_ENABLE,
      defaults.gatewayStoreEnabled
    ),
    rustStoreEnabled: parseBoolean(
      env.MEMORY_RUST_STORE_ENABLE,
      defaults.rustStoreEnabled
    ),
    autoWrite: parseBoolean(env.MEMORY_AUTO_WRITE_ENABLE, defaults.autoWrite),
    promptInjection: parseBoolean(
      env.MEMORY_PROMPT_INJECTION_ENABLE,
      defaults.promptInjection
    ),
    tokenBudget: parsePositiveInt(env.MEMORY_INJECTION_TOKEN_BUDGET, defaults.tokenBudget),
    retrievalTopK: parsePositiveInt(env.MEMORY_RETRIEVAL_TOP_K, defaults.retrievalTopK),
    llmExtractEnabled: parseBoolean(
      env.MEMORY_LLM_EXTRACT_ENABLE,
      defaults.llmExtractEnabled
    ),
  };
};

export const mergeMemorySettings = (
  base: MemorySettings,
  patch?: Partial<MemorySettings> | null
): MemorySettings => {
  if (!patch) {
    return { ...base };
  }
  return {
    ...base,
    ...patch,
    tokenBudget:
      patch.tokenBudget && patch.tokenBudget > 0 ? patch.tokenBudget : base.tokenBudget,
    retrievalTopK:
      patch.retrievalTopK && patch.retrievalTopK > 0 ? patch.retrievalTopK : base.retrievalTopK,
  };
};

export const clampMemorySettings = (settings: MemorySettings): MemorySettings => ({
  ...settings,
  tokenBudget: Math.max(200, Math.min(settings.tokenBudget, 6000)),
  retrievalTopK: Math.max(1, Math.min(settings.retrievalTopK, 50)),
});
