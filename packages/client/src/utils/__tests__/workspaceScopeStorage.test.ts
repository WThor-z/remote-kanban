import { beforeEach, describe, expect, it } from 'vitest';

import {
  WORKSPACE_SCOPE_STORAGE_KEY,
  readStoredWorkspaceScope,
  storeWorkspaceScope,
} from '../workspaceScopeStorage';

describe('workspaceScopeStorage', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('reads stored workspace scope value', () => {
    window.localStorage.setItem(WORKSPACE_SCOPE_STORAGE_KEY, 'ws-1');
    expect(readStoredWorkspaceScope()).toBe('ws-1');
  });

  it('returns empty string when scope is missing', () => {
    expect(readStoredWorkspaceScope()).toBe('');
  });

  it('stores non-empty workspace scope', () => {
    storeWorkspaceScope('ws-2');
    expect(window.localStorage.getItem(WORKSPACE_SCOPE_STORAGE_KEY)).toBe('ws-2');
  });

  it('removes key when scope is empty', () => {
    window.localStorage.setItem(WORKSPACE_SCOPE_STORAGE_KEY, 'ws-3');
    storeWorkspaceScope('');
    expect(window.localStorage.getItem(WORKSPACE_SCOPE_STORAGE_KEY)).toBeNull();
  });
});
