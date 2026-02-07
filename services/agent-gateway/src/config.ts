import os from 'node:os';
import type { HostCapabilities } from './types.js';

export interface GatewayRuntimeConfig {
  serverUrl: string;
  hostId: string;
  authToken: string;
  capabilities: HostCapabilities;
  allowedProjectRoots: string[];
}

type EnvMap = Record<string, string | undefined>;

const DEFAULT_SERVER_URL = 'ws://127.0.0.1:8081';
const DEFAULT_AUTH_TOKEN = 'dev-token';
const DEFAULT_MAX_CONCURRENT = 2;

const parseAllowedProjectRoots = (raw: string | undefined): string[] => {
  if (!raw) {
    return [];
  }

  return raw
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
};

const sanitizeHostId = (value: string): string => {
  const sanitized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-_]+/g, '-')
    .replace(/^-+|-+$/g, '');

  return sanitized || 'gateway-host';
};

const parseMaxConcurrent = (raw: string | undefined): number => {
  const parsed = Number.parseInt(raw ?? '', 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return DEFAULT_MAX_CONCURRENT;
  }
  return parsed;
};

export const createGatewayConfig = (
  env: EnvMap = process.env,
  cwd: string = process.cwd(),
  hostName: string = os.hostname()
): GatewayRuntimeConfig => {
  const resolvedHostName = hostName.trim() || 'Gateway Host';
  const hostId = (env.GATEWAY_HOST_ID || '').trim() || `host-${sanitizeHostId(resolvedHostName)}`;

  return {
    serverUrl: (env.GATEWAY_SERVER_URL || '').trim() || DEFAULT_SERVER_URL,
    hostId,
    authToken: (env.GATEWAY_AUTH_TOKEN || '').trim() || DEFAULT_AUTH_TOKEN,
    capabilities: {
      name: (env.GATEWAY_HOST_NAME || '').trim() || resolvedHostName,
      agents: ['opencode'],
      maxConcurrent: parseMaxConcurrent(env.GATEWAY_MAX_CONCURRENT),
      cwd: (env.GATEWAY_CWD || '').trim() || cwd,
    },
    allowedProjectRoots: parseAllowedProjectRoots(env.GATEWAY_ALLOWED_PROJECT_ROOTS),
  };
};
