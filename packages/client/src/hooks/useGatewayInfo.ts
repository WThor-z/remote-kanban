import { useCallback, useEffect, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

export interface GatewayInfo {
  status: string;
  version: string;
  dataDir?: string;
  workerUrl?: string;
  repoPath?: string;
  featureFlags?: {
    multiTenant: boolean;
    orchestratorV1: boolean;
    memoryEnhanced: boolean;
  };
}

export const useGatewayInfo = () => {
  const [info, setInfo] = useState<GatewayInfo | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const fetchInfo = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/health`);
      if (!response.ok) {
        throw new Error(`Gateway health check failed (${response.status})`);
      }
      const payload = (await response.json()) as GatewayInfo;
      setInfo(payload);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load gateway info');
      setInfo(null);
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  useEffect(() => {
    fetchInfo();
  }, [fetchInfo]);

  return {
    info,
    isLoading,
    error,
    refresh: fetchInfo,
    baseUrl,
  };
};
