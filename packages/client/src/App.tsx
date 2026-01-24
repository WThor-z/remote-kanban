import { useState } from 'react';
import { useOpencode } from './hooks/useOpencode';
import { useKanban } from './hooks/useKanban';
import { useTaskSession } from './hooks/useTaskSession';
import { Bot } from 'lucide-react';
import { InputBar } from './components/InputBar';
import { KanbanBoard } from './components/kanban/KanbanBoard';
import { AgentPanel } from './components/agent';
import { TaskDetailPanel } from './components/task';
import type { KanbanTask } from '@opencode-vibe/protocol';

function App() {
  const { isConnected, socket } = useOpencode();
  const { board, moveTask, deleteTask } = useKanban();
  const [selectedTask, setSelectedTask] = useState<KanbanTask | null>(null);

  const {
    history,
    status,
    isLoading,
    error,
    selectTask,
    executeTask,
    stopTask,
    sendMessage,
  } = useTaskSession({ socket, isConnected });

  // 获取正在执行的任务 ID 列表
  const executingTaskIds = Object.values(board.tasks)
    .filter(task => task.status === 'doing')
    .map(task => task.id);

  const handleTaskClick = (task: KanbanTask) => {
    setSelectedTask(task);
    selectTask(task.id);
  };

  const handleCloseDetail = () => {
    setSelectedTask(null);
    selectTask(null);
  };

  const handleExecuteTask = (taskId: string) => {
    executeTask(taskId);
  };

  const handleStopTask = (taskId: string) => {
    stopTask(taskId);
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

        <div className={`inline-flex items-center px-4 py-2 rounded-full text-sm font-medium ${
          isConnected 
            ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' 
            : 'bg-rose-500/10 text-rose-400 border border-rose-500/20'
        }`}>
          <span className={`w-2 h-2 rounded-full mr-2 ${
            isConnected ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'
          }`}></span>
          {isConnected ? 'Connected to Server' : 'Disconnected'}
        </div>
      </div>

      {/* Agent Panel - Primary Interface */}
      <div className="w-full max-w-6xl">
        <AgentPanel />
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

      {/* Task Detail Panel (Modal) */}
      {selectedTask && (
        <TaskDetailPanel
          task={selectedTask}
          history={history}
          status={status}
          isLoading={isLoading}
          error={error}
          onClose={handleCloseDetail}
          onExecute={handleExecuteTask}
          onStop={handleStopTask}
          onSendMessage={handleSendMessage}
        />
      )}
    </div>
  )
}

export default App
