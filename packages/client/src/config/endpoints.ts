const resolveEnvValue = (viteKey: string, nodeKey: string): string | undefined => {
  if (typeof process !== 'undefined') {
    const value = (process as { env?: Record<string, string> }).env?.[nodeKey];
    if (value) {
      return value;
    }
  }

  if (typeof import.meta !== 'undefined') {
    const value = (import.meta as { env?: Record<string, string> }).env?.[viteKey];
    if (value) {
      return value;
    }
  }

  return undefined;
};

export const resolveGatewaySocketUrl = () => {
  return (
    resolveEnvValue('VITE_OPENCODE_SOCKET_URL', 'OPENCODE_SOCKET_URL') ||
    'http://localhost:8080'
  );
};

export const resolveLegacySocketUrl = () => {
  return (
    resolveEnvValue('VITE_LEGACY_SOCKET_URL', 'LEGACY_SOCKET_URL') ||
    'http://localhost:3000'
  );
};

export const resolveApiBaseUrl = () => {
  return (
    resolveEnvValue('VITE_RUST_API_URL', 'RUST_API_URL') ||
    'http://localhost:8081'
  );
};
