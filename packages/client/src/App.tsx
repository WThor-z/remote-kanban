import { useState, useEffect, useCallback, useMemo } from 'react';
import { useOpencode } from './hooks/useOpencode';
import { useKanban } from './hooks/useKanban';
import { useTaskSession } from './hooks/useTaskSession';
import { useTaskApi, type CreateTaskRequest } from './hooks/useTaskApi';
import { useTaskExecutor } from './hooks/useTaskExecutor';
import { Bot, Plus, Server, HardDrive, Plug, RefreshCw, Layers, ChevronDown } from 'lucide-react';
import { KanbanBoard } from './components/kanban/KanbanBoard';
import { TaskDetailPanel, CreateTaskModal } from './components/task';
import type { KanbanTask, AgentType } from '@opencode-vibe/protocol';
import { useGatewayInfo } from './hooks/useGatewayInfo';
import { useWorkspaces } from './hooks/useWorkspaces';
import { resolveApiBaseUrl, resolveGatewaySocketUrl } from './config/endpoints';
import { getConsoleLexiconSection } from './lexicon/consoleLexicon';
import { readStoredWorkspaceScope, storeWorkspaceScope } from './utils/workspaceScopeStorage';
import { filterBoardByVisibleTaskIds } from './utils/kanbanBoardFilter';

const SKIN_STORAGE_KEY = 'vk-console-skin';

const readStoredSkin = (): 'neural' | 'lab' => {
  if (typeof window === 'undefined') {
    return 'neural';
  }

  return window.localStorage.getItem(SKIN_STORAGE_KEY) === 'lab' ? 'lab' : 'neural';
};

function App() {
  const { isConnected, socket } = useOpencode();
  const { board, moveTask, deleteTask, requestSync } = useKanban();
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [skin, setSkin] = useState<'neural' | 'lab'>(readStoredSkin);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState(readStoredWorkspaceScope);
  const [isWorkspaceScopeOpen, setIsWorkspaceScopeOpen] = useState(false);
  const sharedCopy = getConsoleLexiconSection('shared');
  const appCopy = getConsoleLexiconSection('app');
  const createTaskModalCopy = getConsoleLexiconSection('createTaskModal');

  const gatewaySocketUrl = resolveGatewaySocketUrl();
  const apiBaseUrl = resolveApiBaseUrl();
  const {
    info: gatewayInfo,
    isLoading: gatewayInfoLoading,
    error: gatewayInfoError,
    refresh: refreshGatewayInfo,
  } = useGatewayInfo();
  const {
    workspaces,
    isLoading: workspacesLoading,
    hasWorkspaces,
  } = useWorkspaces();
  const activeWorkspace = workspaces.find((workspace) => workspace.id === activeWorkspaceId);

  // Rust API hook for task management
  const {
    isLoading: isTaskApiLoading,
    error: taskApiError,
    createTask,
    getTask,
    clearError: clearTaskApiError,
  } = useTaskApi();
  const {
    tasks: scopedTasks,
    fetchTasks: fetchScopedTasks,
  } = useTaskApi();

  // Task executor hook for isolated worktree execution
  const {
    currentSession,
    isExecuting,
    error: executorError,
    startExecution,
    stopExecution: stopIsolatedExecution,
    getExecutionStatus,
    cleanupWorktree,
    sendInput: sendInputToTask,
  } = useTaskExecutor();

  const {
    history,
    status,
    isLoading,
    error,
    selectTask,
    stopTask,
    sendMessage,
  } = useTaskSession({ socket, isConnected });

  // Keyboard shortcut "c" to open create modal
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ignore if user is typing in an input field
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }
      if (e.key === 'c' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        setIsCreateModalOpen(true);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Handle task creation via Rust API and sync to Kanban board
  const handleCreateTask = useCallback(async (data: CreateTaskRequest): Promise<boolean> => {
    const task = await createTask(data);
    if (task) {
      console.log('[App] Task created successfully via Rust API:', task);
      // Request sync to refresh kanban board with new task from REST API
      requestSync();
      if (activeWorkspaceId) {
        await fetchScopedTasks({ workspaceId: activeWorkspaceId });
      }
      return true;
    }
    return false;
  }, [activeWorkspaceId, createTask, fetchScopedTasks, requestSync]);

  // Handle task creation and immediate execution
  const handleCreateAndStartTask = useCallback(async (data: CreateTaskRequest): Promise<boolean> => {
    console.log('[App] handleCreateAndStartTask called with data:', data);
    console.log('[App] data.model:', data.model);
    
    const task = await createTask(data);
    if (task) {
      console.log('[App] Task created successfully, starting execution:', task);
      // Request sync to refresh kanban board
      requestSync();
      if (activeWorkspaceId) {
        await fetchScopedTasks({ workspaceId: activeWorkspaceId });
      }
      
      // Start execution with task-configured agent/model settings
      const executeRequest = {
        agentType: (data.agentType || 'opencode') as AgentType,
        baseBranch: data.baseBranch || 'main',
        model: data.model,
      };
      console.log('[App] Calling startExecution with:', executeRequest);
      
      const result = await startExecution(task.id, executeRequest);
      
      if (result) {
        console.log('[App] Task execution started:', result);
        await getExecutionStatus(task.id);
        return true;
      }
    }
    return false;
  }, [activeWorkspaceId, createTask, fetchScopedTasks, requestSync, startExecution, getExecutionStatus]);

  const handleCloseCreateModal = useCallback(() => {
    setIsCreateModalOpen(false);
    clearTaskApiError();
  }, [clearTaskApiError]);

  const visibleTaskIds = useMemo(() => {
    if (!activeWorkspaceId) {
      return null;
    }

    return new Set(scopedTasks.map((task) => task.id));
  }, [activeWorkspaceId, scopedTasks]);

  const filteredBoard = useMemo(
    () => filterBoardByVisibleTaskIds(board, visibleTaskIds),
    [board, visibleTaskIds],
  );

  // 获取正在执行的任务 ID 列表
  const executingTaskIds = Object.values(filteredBoard.tasks)
    .filter(task => task.status === 'doing')
    .map(task => task.id);

  const handleTaskClick = async (task: KanbanTask) => {
    setSelectedTask(task);
    selectTask(task.id);
    // Fetch execution status for the selected task
    await getExecutionStatus(task.id);
  };

  const handleCloseDetail = () => {
    setSelectedTask(null);
    selectTask(null);
  };

  const handleExecuteTask = async (taskId: string) => {
    // Fetch full task from REST API to get all fields including model
    // The Kanban board tasks (board.tasks) don't include model/agentType/baseBranch fields
    const fullTask = await getTask(taskId);
    
    if (fullTask) {
      // Start execution in isolated worktree via REST API
      const agentType = fullTask.agentType || 'opencode';
      const baseBranch = fullTask.baseBranch || 'main';
      const model = fullTask.model || undefined;
      
      console.log('[App] handleExecuteTask:', { taskId, agentType, baseBranch, model });
      
      const result = await startExecution(taskId, {
        agentType: agentType as AgentType,
        baseBranch,
        model,
      });
      if (result) {
        await getExecutionStatus(taskId);
      }
    } else {
      console.error('[App] handleExecuteTask: Failed to fetch task from REST API:', taskId);
    }
    // Socket.IO updates will come automatically via task:execution_event
  };

  const handleStopTask = async (taskId: string) => {
    // Stop via REST API
    await stopIsolatedExecution(taskId);
    // Also stop via Socket.IO
    stopTask(taskId);
  };

  const handleCleanupWorktree = async (taskId: string) => {
    const success = await cleanupWorktree(taskId);
    if (success) {
      console.log('[App] Worktree cleaned up for task:', taskId);
      // Refresh execution status
      await getExecutionStatus(taskId);
    }
  };

  const handleSendMessage = (taskId: string, content: string) => {
    sendMessage(taskId, content);
  };

  const isLabSkin = skin === 'lab';

  useEffect(() => {
    if (activeWorkspaceId && !workspaces.some((workspace) => workspace.id === activeWorkspaceId)) {
      setActiveWorkspaceId('');
    }
  }, [activeWorkspaceId, workspaces]);

  useEffect(() => {
    if (!activeWorkspaceId) {
      return;
    }

    void fetchScopedTasks({ workspaceId: activeWorkspaceId });
  }, [activeWorkspaceId, fetchScopedTasks]);

  useEffect(() => {
    if (!selectedTask || !visibleTaskIds) {
      return;
    }

    if (!visibleTaskIds.has(selectedTask.id)) {
      setSelectedTask(null);
      selectTask(null);
    }
  }, [selectTask, selectedTask, visibleTaskIds]);

  useEffect(() => {
    storeWorkspaceScope(activeWorkspaceId);
  }, [activeWorkspaceId]);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(SKIN_STORAGE_KEY, skin);
    }
  }, [skin]);

  return (
    <div className={`console-root ${isLabSkin ? 'console-root--lab' : ''}`}>
      <div className="console-shell">
        <section className="tech-panel command-panel reveal">
          <div className="command-panel__top">
            <div>
              <p className="tech-kicker inline-flex items-center gap-2">
                <Bot size={14} /> {appCopy.kicker}
              </p>
              <h1 className="tech-title">{appCopy.title}</h1>
              <p className="tech-subtle">
                {appCopy.subtitle}
              </p>

              <div className="command-panel__labels" aria-label="command lexicon">
                <span className="command-chip">{sharedCopy.chips.directive}</span>
                <span className="command-chip">{sharedCopy.chips.telemetry}</span>
                <span className="command-chip">{sharedCopy.chips.missionLane}</span>
              </div>
            </div>

            <div className={`status-beacon ${isConnected ? 'status-beacon--online' : 'status-beacon--offline'}`}>
              <span className="status-beacon__dot" aria-hidden="true" />
              {isConnected ? sharedCopy.status.gatewayOnline : sharedCopy.status.gatewayOffline}
            </div>
          </div>

          <div className="command-panel__actions">
            <div className="dropdown-wrap">
              <button
                type="button"
                className="glass-select flex items-center justify-between"
                onClick={() => setIsWorkspaceScopeOpen(!isWorkspaceScopeOpen)}
                disabled={!hasWorkspaces}
                title={appCopy.actions.workspaceScope}
                aria-label={appCopy.actions.workspaceScope}
              >
                <span className="flex items-center gap-2 text-slate-200">
                  <Layers size={14} className="text-cyan-300" />
                  {activeWorkspace ? activeWorkspace.name : createTaskModalCopy.placeholders.workspaceAny}
                  {workspacesLoading && <span className="text-xs text-slate-500">(loading...)</span>}
                </span>
                <ChevronDown size={14} className={`text-slate-400 transition-transform ${isWorkspaceScopeOpen ? 'rotate-180' : ''}`} />
              </button>
              {isWorkspaceScopeOpen && (
                <div className="dropdown-panel dropdown-panel--scroll">
                  <button
                    type="button"
                    className={`dropdown-item ${!activeWorkspaceId ? 'dropdown-item--active' : ''}`}
                    onClick={() => {
                      setActiveWorkspaceId('');
                      setIsWorkspaceScopeOpen(false);
                    }}
                  >
                    <div className="text-slate-200">{createTaskModalCopy.placeholders.workspaceAny}</div>
                    <div className="dropdown-note">{createTaskModalCopy.placeholders.workspaceAnyHint}</div>
                  </button>
                  {workspaces.map((workspace) => (
                    <button
                      key={workspace.id}
                      type="button"
                      className={`dropdown-item ${activeWorkspaceId === workspace.id ? 'dropdown-item--active' : ''}`}
                      onClick={() => {
                        setActiveWorkspaceId(workspace.id);
                        setIsWorkspaceScopeOpen(false);
                      }}
                    >
                      <div className="text-cyan-200">{workspace.name}</div>
                      <div className="dropdown-note">{workspace.rootPath}</div>
                    </button>
                  ))}
                </div>
              )}
            </div>
            <button
              type="button"
              onClick={() => setIsCreateModalOpen(true)}
              className="tech-btn tech-btn-primary"
              title="Create new task (Press 'c')"
            >
              <Plus size={16} /> {appCopy.actions.createTask}
            </button>
            <button
              type="button"
              onClick={refreshGatewayInfo}
              className="tech-btn tech-btn-secondary"
            >
              <RefreshCw size={14} className={gatewayInfoLoading ? 'animate-spin' : ''} /> {appCopy.actions.syncTelemetry}
            </button>
            <button
              type="button"
              className="tech-btn tech-btn-secondary"
              onClick={() => setSkin((prev) => (prev === 'lab' ? 'neural' : 'lab'))}
            >
              {isLabSkin ? sharedCopy.skin.backToNeural : sharedCopy.skin.switchToLab}
            </button>
          </div>
        </section>

        <section className="tech-panel gateway-panel reveal reveal-1">
          <div className="section-bar">
            <div className="flex items-center gap-2">
              <Server size={16} className="text-cyan-300" />
              <h2 className="section-title">{appCopy.sections.gatewayTitle}</h2>
              {gatewayInfo?.version && <span className="section-note">v{gatewayInfo.version}</span>}
            </div>
            <p className="section-note">{appCopy.sections.gatewayNote}</p>
          </div>

          <div className="gateway-grid">
            <div className="gateway-card">
              <div className="gateway-label">Socket</div>
              <div className="gateway-value gateway-value--mono">{gatewaySocketUrl}</div>
            </div>

            <div className="gateway-card">
              <div className="gateway-label">REST API</div>
              <div className="gateway-value gateway-value--mono">{apiBaseUrl}</div>
            </div>

            <div className="gateway-card">
              <div className="gateway-label">
                <Plug size={12} /> Worker
              </div>
              <div className="gateway-value gateway-value--mono">{gatewayInfo?.workerUrl || 'unknown'}</div>
            </div>

            <div className="gateway-card">
              <div className="gateway-label">
                <HardDrive size={12} /> Data Dir
              </div>
              <div className="gateway-value gateway-value--mono">{gatewayInfo?.dataDir || 'unknown'}</div>
            </div>
          </div>

          {gatewayInfoError && <div className="gateway-error">{gatewayInfoError}</div>}
        </section>

        <section className="tech-panel board-panel reveal reveal-2">
          <div className="section-bar">
              <h2 className="section-title">{appCopy.sections.boardTitle}</h2>
              <p className="section-note">
              {Object.keys(filteredBoard.tasks).length} {appCopy.sections.boardCounterSuffix}
              </p>
            </div>

          <KanbanBoard
            board={filteredBoard}
            onMoveTask={moveTask}
            onDeleteTask={deleteTask}
            onTaskClick={handleTaskClick}
            executingTaskIds={executingTaskIds}
          />
        </section>

      {selectedTask && (
        <TaskDetailPanel
          task={selectedTask}
          history={history}
          status={status}
          isLoading={isLoading || isExecuting}
          error={error || executorError}
          executionInfo={currentSession && currentSession.taskId === selectedTask.id ? {
            sessionId: currentSession.sessionId,
            worktreePath: currentSession.worktreePath,
            branch: currentSession.branch,
            state: currentSession.state,
          } : null}
          onClose={handleCloseDetail}
          onExecute={handleExecuteTask}
          onStop={handleStopTask}
          onSendMessage={handleSendMessage}
          onSendInput={sendInputToTask}
          onCleanupWorktree={handleCleanupWorktree}
        />
      )}

        <CreateTaskModal
          isOpen={isCreateModalOpen}
          onClose={handleCloseCreateModal}
          onCreate={handleCreateTask}
          onCreateAndStart={handleCreateAndStartTask}
          isLoading={isTaskApiLoading}
          error={taskApiError}
          defaultWorkspaceId={activeWorkspaceId || undefined}
        />
      </div>
    </div>
  )
}

export default App
