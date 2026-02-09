export const WORKSPACE_SCOPE_STORAGE_KEY = 'vk-active-workspace-scope';

export const readStoredWorkspaceScope = (): string => {
  if (typeof window === 'undefined') {
    return '';
  }

  return window.localStorage.getItem(WORKSPACE_SCOPE_STORAGE_KEY) || '';
};

export const storeWorkspaceScope = (scope: string): void => {
  if (typeof window === 'undefined') {
    return;
  }

  if (scope) {
    window.localStorage.setItem(WORKSPACE_SCOPE_STORAGE_KEY, scope);
  } else {
    window.localStorage.removeItem(WORKSPACE_SCOPE_STORAGE_KEY);
  }
};
