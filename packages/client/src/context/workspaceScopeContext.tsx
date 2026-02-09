import { createContext, useContext, type ReactNode } from 'react';

export interface WorkspaceScopeContextValue {
  activeWorkspaceId: string;
  setActiveWorkspaceId: (workspaceId: string) => void;
}

const WorkspaceScopeContext = createContext<WorkspaceScopeContextValue>({
  activeWorkspaceId: '',
  setActiveWorkspaceId: () => {},
});

interface WorkspaceScopeProviderProps {
  value: WorkspaceScopeContextValue;
  children: ReactNode;
}

export function WorkspaceScopeProvider({ value, children }: WorkspaceScopeProviderProps) {
  return (
    <WorkspaceScopeContext.Provider value={value}>
      {children}
    </WorkspaceScopeContext.Provider>
  );
}

export function useWorkspaceScope(): WorkspaceScopeContextValue {
  return useContext(WorkspaceScopeContext);
}
