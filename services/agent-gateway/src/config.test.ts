import { describe, it, expect } from 'vitest';
import { createGatewayConfig } from './config.js';

describe('createGatewayConfig', () => {
  it('uses local-friendly defaults when env is empty', () => {
    const config = createGatewayConfig({}, '/repo/project', 'worker-host');

    expect(config.serverUrl).toBe('ws://127.0.0.1:8081');
    expect(config.authToken).toBe('dev-token');
    expect(config.capabilities.cwd).toBe('/repo/project');
    expect(config.capabilities.maxConcurrent).toBe(2);
    expect(config.allowedProjectRoots).toEqual([]);
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
});
