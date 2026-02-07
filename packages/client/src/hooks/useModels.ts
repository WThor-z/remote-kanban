/**
 * useModels Hook
 * 
 * Fetches available AI models from a connected Agent Gateway host.
 */

import { useCallback, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

/** Model capabilities */
export interface ModelCapabilities {
  temperature: boolean;
  reasoning: boolean;
  attachment: boolean;
  toolcall: boolean;
}

/** Model information */
export interface ModelInfo {
  id: string;
  providerId: string;
  name: string;
  capabilities?: ModelCapabilities;
}

/** Provider information */
export interface ProviderInfo {
  id: string;
  name: string;
  models: ModelInfo[];
}

/** Combined model identifier (provider/model format) */
export interface ModelOption {
  /** Full model ID in provider/model format */
  value: string;
  /** Display label */
  label: string;
  /** Provider name */
  provider: string;
  /** Model info */
  model: ModelInfo;
}

export interface UseModelsResult {
  /** List of providers with their models */
  providers: ProviderInfo[];
  /** Flattened list of model options for select components */
  modelOptions: ModelOption[];
  /** Whether models are being loaded */
  isLoading: boolean;
  /** Error message if fetch failed */
  error: string | null;
  /** Fetch models from a specific host */
  fetchModels: (hostId: string) => Promise<void>;
  /** Clear the models list */
  clearModels: () => void;
}

export const useModels = (): UseModelsResult => {
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const fetchModels = useCallback(async (hostId: string) => {
    if (!hostId || hostId === 'auto') {
      // Can't fetch models without a specific host
      setProviders([]);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/hosts/${hostId}/models`);
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.error || `Failed to fetch models (${response.status})`);
      }
      const data: ProviderInfo[] = await response.json();
      setProviders(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load models');
      setProviders([]);
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const clearModels = useCallback(() => {
    setProviders([]);
    setError(null);
  }, []);

  // Flatten providers into a list of model options
  const modelOptions: ModelOption[] = providers.flatMap(provider =>
    provider.models.map(model => ({
      value: `${provider.id}/${model.id}`,
      label: `${model.name} (${provider.name})`,
      provider: provider.name,
      model,
    }))
  );

  return {
    providers,
    modelOptions,
    isLoading,
    error,
    fetchModels,
    clearModels,
  };
};
