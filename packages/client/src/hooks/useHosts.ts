/**
 * useHosts Hook
 * 
 * Fetches and manages the list of connected Agent Gateway hosts.
 */

import { useCallback, useEffect, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

/** Host connection status */
export type HostConnectionStatus = 'online' | 'busy' | 'offline';

/** Host capabilities */
export interface HostCapabilities {
  name: string;
  agents: string[];
  maxConcurrent: number;
  cwd: string;
  labels?: Record<string, string>;
}

/** Host status information */
export interface HostStatus {
  hostId: string;
  name: string;
  status: HostConnectionStatus;
  capabilities: HostCapabilities;
  activeTasks: string[];
  lastHeartbeat: number;
  connectedAt: number;
}

/** Special host option for automatic selection */
export const AUTO_HOST = 'auto';

export interface UseHostsResult {
  /** List of connected hosts */
  hosts: HostStatus[];
  /** Whether hosts are being loaded */
  isLoading: boolean;
  /** Error message if fetch failed */
  error: string | null;
  /** Refresh the hosts list */
  refresh: () => Promise<void>;
  /** Get hosts that support a specific agent type */
  getHostsForAgent: (agentType: string) => HostStatus[];
  /** Check if any host is available */
  hasAvailableHosts: boolean;
}

export const useHosts = (): UseHostsResult => {
  const [hosts, setHosts] = useState<HostStatus[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const fetchHosts = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/hosts`);
      if (!response.ok) {
        throw new Error(`Failed to fetch hosts (${response.status})`);
      }
      const data = await response.json();
      // API returns { hosts: [...] }
      const hostList = Array.isArray(data) ? data : (data.hosts || []);
      setHosts(hostList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load hosts');
      setHosts([]);
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  useEffect(() => {
    fetchHosts();
    // Refresh every 30 seconds
    const interval = setInterval(fetchHosts, 30000);
    return () => clearInterval(interval);
  }, [fetchHosts]);

  const getHostsForAgent = useCallback((agentType: string): HostStatus[] => {
    return hosts.filter(
      host => host.capabilities.agents.includes(agentType) && host.status !== 'offline'
    );
  }, [hosts]);

  const hasAvailableHosts = hosts.some(h => h.status === 'online');

  return {
    hosts,
    isLoading,
    error,
    refresh: fetchHosts,
    getHostsForAgent,
    hasAvailableHosts,
  };
};
