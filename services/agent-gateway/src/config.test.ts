import { describe, it, expect } from 'vitest';
import path from 'node:path';
import { createGatewayConfig } from './config.js';

describe('createGatewayConfig', () => {
  it('uses local-friendly defaults when env is empty', () => {
    const config = createGatewayConfig({}, '/repo/project', 'worker-host');

    expect(config.serverUrl).toBe('ws://127.0.0.1:8081');
    expect(config.authToken).toBe('dev-token');
    expect(config.capabilities.cwd).toBe('/repo/project');
    expect(config.capabilities.maxConcurrent).toBe(2);
    expect(config.allowedProjectRoots).toEqual([]);
    expect(config.memory.enabled).toBe(true);
    expect(config.memory.tokenBudget).toBe(1200);
    expect(config.memoryDataDir).toBe(path.join('/repo/project', '.gateway-memory'));
  });

  it('parses comma separated allowed roots', () => {
    const config = createGatewayConfig(
      { GATEWAY_ALLOWED_PROJECT_ROOTS: ' /srv/a, ,/srv/b  ,   ' },
      '/repo/project',
      'worker-host'
    );

    expect(config.allowedProjectRoots).toEqual(['/srv/a', '/srv/b']);
  });

  it('falls back when max concurrent is invalid', () => {
    const config = createGatewayConfig(
      { GATEWAY_MAX_CONCURRENT: 'not-a-number' },
      '/repo/project',
      'worker-host'
    );

    expect(config.capabilities.maxConcurrent).toBe(2);
  });

  it('parses memory env overrides', () => {
    const config = createGatewayConfig(
      {
        MEMORY_ENABLE: 'false',
        MEMORY_PROMPT_INJECTION_ENABLE: 'false',
        MEMORY_INJECTION_TOKEN_BUDGET: '2048',
        MEMORY_RETRIEVAL_TOP_K: '12',
        MEMORY_DATA_DIR: '/tmp/memory-store',
      },
      '/repo/project',
      'worker-host'
    );

    expect(config.memory.enabled).toBe(false);
    expect(config.memory.promptInjection).toBe(false);
    expect(config.memory.tokenBudget).toBe(2048);
    expect(config.memory.retrievalTopK).toBe(12);
    expect(config.memoryDataDir).toBe('/tmp/memory-store');
  });
});
