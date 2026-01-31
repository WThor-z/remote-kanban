import { useState, useEffect, useCallback } from 'react';
import { useOpencode } from './hooks/useOpencode';
import { useKanban } from './hooks/useKanban';
import { useTaskSession } from './hooks/useTaskSession';
import { useTaskApi, type CreateTaskRequest } from './hooks/useTaskApi';
import { useTaskExecutor } from './hooks/useTaskExecutor';
import { Bot, Plus, Server, HardDrive, Plug, ChevronDown, ChevronUp, RefreshCw } from 'lucide-react';
import { InputBar } from './components/InputBar';
import { KanbanBoard } from './components/kanban/KanbanBoard';
import { AgentPanel } from './components/agent';
import { TaskDetailPanel, CreateTaskModal } from './components/task';
import type { KanbanTask, AgentType } from '@opencode-vibe/protocol';
import { useGatewayInfo } from './hooks/useGatewayInfo';
import { resolveApiBaseUrl, resolveGatewaySocketUrl, resolveLegacySocketUrl } from './config/endpoints';

function App() {
  const { isConnected, socket } = useOpencode();
  const { board, moveTask, deleteTask, requestSync } = useKanban();
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [showLegacyAgent, setShowLegacyAgent] = useState(false);

  const gatewaySocketUrl = resolveGatewaySocketUrl();
  const legacySocketUrl = resolveLegacySocketUrl();
  const apiBaseUrl = resolveApiBaseUrl();
  const {
    info: gatewayInfo,
    isLoading: gatewayInfoLoading,
    error: gatewayInfoError,
    refresh: refreshGatewayInfo,
  } = useGatewayInfo();

  // Rust API hook for task management
  const {
    isLoading: isTaskApiLoading,
    error: taskApiError,
    createTask,
    clearError: clearTaskApiError,
  } = useTaskApi();

  // Task executor hook for isolated worktree execution
  const {
    currentSession,
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
      return true;
    }
    return false;
  }, [createTask, requestSync]);

  const handleCloseCreateModal = useCallback(() => {
    setIsCreateModalOpen(false);
    clearTaskApiError();
  }, [clearTaskApiError]);

  // 获取正在执行的任务 ID 列表
  const executingTaskIds = Object.values(board.tasks)
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
    const task = board.tasks[taskId];
    if (task) {
      // Start execution in isolated worktree via REST API
      const agentType = (task as unknown as { agentType?: string }).agentType || 'opencode';
      const baseBranch = (task as unknown as { baseBranch?: string }).baseBranch || 'main';
      await startExecution(taskId, {
        agentType: agentType as AgentType,
        baseBranch,
      });
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

  return (
    <div className="min-h-screen bg-slate-900 text-white flex flex-col items-center p-6 gap-8">
      <div className="w-full max-w-6xl bg-slate-800 p-8 rounded-xl shadow-2xl border border-slate-700 text-center space-y-6">
        <div className="flex justify-center">
          <div className="bg-indigo-600 p-4 rounded-full shadow-lg shadow-indigo-500/20">
            <Bot size={48} className="text-white" />
          </div>
        </div>

        <h1 className="text-4xl font-bold bg-gradient-to-r from-indigo-400 to-cyan-400 bg-clip-text text-transparent">
          OpenCode Vibe Kanban
        </h1>

        <p className="text-slate-400 text-lg">
          AI-Powered Development with Visual Task Management
        </p>

        {/* Create Task Button */}
        <button
          type="button"
          onClick={() => setIsCreateModalOpen(true)}
          className="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg font-medium transition-colors shadow-lg shadow-indigo-500/20"
          title="Create new task (Press 'c')"
        >
          <Plus size={18} />
          New Task
        </button>

        <div className={`inline-flex items-center px-4 py-2 rounded-full text-sm font-medium ${
          isConnected 
            ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' 
            : 'bg-rose-500/10 text-rose-400 border border-rose-500/20'
        }`}>
          <span className={`w-2 h-2 rounded-full mr-2 ${
            isConnected ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'
          }`}></span>
          {isConnected ? 'Gateway Connected' : 'Gateway Disconnected'}
        </div>
      </div>

      {/* Gateway Status */}
      <div className="w-full max-w-6xl bg-slate-800/60 border border-slate-700/60 rounded-xl p-4">
        <div className="flex flex-wrap items-center justify-between gap-3 mb-3">
          <div className="flex items-center gap-2 text-slate-200">
            <Server size={18} className="text-indigo-400" />
            <span className="font-semibold">Gateway Status</span>
            {gatewayInfo?.version && (
              <span className="text-xs text-slate-400">v{gatewayInfo.version}</span>
            )}
          </div>
          <button
            type="button"
            onClick={refreshGatewayInfo}
            className="inline-flex items-center gap-2 px-3 py-1.5 text-xs font-semibold rounded-full bg-slate-700/70 text-slate-200 hover:bg-slate-600"
          >
            <RefreshCw size={12} className={gatewayInfoLoading ? 'animate-spin' : ''} />
            Refresh
          </button>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 text-sm">
          <div className="bg-slate-900/40 border border-slate-700/60 rounded-lg p-3">
            <div className="text-xs text-slate-400 mb-1">Socket</div>
            <div className="text-slate-200 break-all font-mono text-xs">{gatewaySocketUrl}</div>
          </div>
          <div className="bg-slate-900/40 border border-slate-700/60 rounded-lg p-3">
            <div className="text-xs text-slate-400 mb-1">REST API</div>
            <div className="text-slate-200 break-all font-mono text-xs">{apiBaseUrl}</div>
          </div>
          <div className="bg-slate-900/40 border border-slate-700/60 rounded-lg p-3">
            <div className="flex items-center gap-1 text-xs text-slate-400 mb-1">
              <Plug size={12} /> Worker
            </div>
            <div className="text-slate-200 break-all font-mono text-xs">{gatewayInfo?.workerUrl || 'unknown'}</div>
          </div>
          <div className="bg-slate-900/40 border border-slate-700/60 rounded-lg p-3">
            <div className="flex items-center gap-1 text-xs text-slate-400 mb-1">
              <HardDrive size={12} /> Data Dir
            </div>
            <div className="text-slate-200 break-all font-mono text-xs">{gatewayInfo?.dataDir || 'unknown'}</div>
          </div>
        </div>
        {gatewayInfoError && (
          <div className="mt-3 text-xs text-rose-400">{gatewayInfoError}</div>
        )}
      </div>

      {/* Kanban Board */}
      <div className="w-full max-w-6xl">
        <KanbanBoard 
          board={board} 
          onMoveTask={moveTask} 
          onDeleteTask={deleteTask}
          onTaskClick={handleTaskClick}
          executingTaskIds={executingTaskIds}
        />
      </div>

      <InputBar />

      {/* Legacy Agent Console */}
      <div className="w-full max-w-6xl">
        <button
          type="button"
          onClick={() => setShowLegacyAgent(!showLegacyAgent)}
          className="w-full flex items-center justify-between px-4 py-3 rounded-xl bg-slate-800/70 border border-slate-700/60 text-slate-200"
        >
          <div>
            <div className="text-sm font-semibold">Legacy Agent Console</div>
            <div className="text-xs text-slate-400">Requires legacy TS server at {legacySocketUrl}</div>
          </div>
          {showLegacyAgent ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
        </button>
        {showLegacyAgent && (
          <div className="mt-3">
            <AgentPanel />
          </div>
        )}
      </div>

      {/* Task Detail Panel (Modal) */}
      {selectedTask && (
        <TaskDetailPanel
          task={selectedTask}
          history={history}
          status={status}
          isLoading={isLoading}
          error={error}
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

      {/* Create Task Modal */}
      <CreateTaskModal
        isOpen={isCreateModalOpen}
        onClose={handleCloseCreateModal}
        onCreate={handleCreateTask}
        isLoading={isTaskApiLoading}
        error={taskApiError}
      />
    </div>
  )
}

export default App
