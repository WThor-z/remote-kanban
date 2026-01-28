import { useState, useRef, useEffect } from 'react';
import { X, Play, Square, Send, Loader2, CheckCircle, XCircle, Clock, GitBranch, Trash2, MessageSquare, Terminal } from 'lucide-react';
import type { KanbanTask, TaskSessionHistory, AgentSessionStatus, ChatMessage } from '@opencode-vibe/protocol';
import { ExecutionLogPanel } from '../execution/ExecutionLogPanel';

interface ExecutionInfo {
  sessionId: string;
  worktreePath: string | null;
  branch: string | null;
  state: string;
}

interface TaskDetailPanelProps {
  task: KanbanTask;
  history: TaskSessionHistory | null;
  status: AgentSessionStatus | null;
  isLoading: boolean;
  error: string | null;
  executionInfo?: ExecutionInfo | null;
  onClose: () => void;
  onExecute: (taskId: string) => void;
  onStop: (taskId: string) => void;
  onSendMessage: (taskId: string, content: string) => void;
  onCleanupWorktree?: (taskId: string) => void;
}

const statusConfig: Record<AgentSessionStatus, { icon: React.ReactNode; label: string; color: string }> = {
  idle: { icon: <Clock size={16} />, label: '等待执行', color: 'text-slate-400' },
  starting: { icon: <Loader2 size={16} className="animate-spin" />, label: '启动中', color: 'text-amber-400' },
  running: { icon: <Loader2 size={16} className="animate-spin" />, label: '执行中', color: 'text-indigo-400' },
  paused: { icon: <Clock size={16} />, label: '已暂停', color: 'text-amber-400' },
  completed: { icon: <CheckCircle size={16} />, label: '已完成', color: 'text-emerald-400' },
  failed: { icon: <XCircle size={16} />, label: '失败', color: 'text-rose-400' },
  aborted: { icon: <XCircle size={16} />, label: '已中止', color: 'text-slate-400' },
};

export function TaskDetailPanel({
  task,
  history,
  status,
  isLoading,
  error,
  executionInfo,
  onClose,
  onExecute,
  onStop,
  onSendMessage,
  onCleanupWorktree,
}: TaskDetailPanelProps) {
  const [inputValue, setInputValue] = useState('');
  const [activeTab, setActiveTab] = useState<'chat' | 'logs'>('chat');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const currentStatus = status || 'idle';
  const statusInfo = statusConfig[currentStatus];

  // Auto-switch to logs when execution starts
  useEffect(() => {
    if (status === 'starting' || status === 'running') {
      setActiveTab('logs');
    }
  }, [status]);

  // 自动滚动到底部
  useEffect(() => {
    if (activeTab === 'chat') {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [history?.messages, activeTab]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputValue.trim()) return;
    onSendMessage(task.id, inputValue.trim());
    setInputValue('');
  };

  const isRunning = currentStatus === 'starting' || currentStatus === 'running';
  const canExecute = currentStatus === 'idle' || currentStatus === 'completed' || currentStatus === 'failed' || currentStatus === 'aborted';

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-slate-800 rounded-xl shadow-2xl border border-slate-700 w-full max-w-2xl h-[80vh] flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-slate-700 bg-slate-800 z-20">
          <div className="flex-1 min-w-0">
            <h2 className="text-lg font-semibold text-white truncate">{task.title}</h2>
            <div className={`flex items-center gap-1.5 text-sm ${statusInfo.color}`}>
              {statusInfo.icon}
              <span>{statusInfo.label}</span>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-2 text-slate-400 hover:text-white transition-colors"
            aria-label="关闭"
          >
            <X size={20} />
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-slate-700 bg-slate-900/30">
          <button
            onClick={() => setActiveTab('chat')}
            className={`flex-1 px-4 py-3 text-sm font-medium flex items-center justify-center gap-2 transition-colors ${
              activeTab === 'chat'
                ? 'text-indigo-400 border-b-2 border-indigo-400 bg-slate-800/50'
                : 'text-slate-400 hover:text-slate-300 hover:bg-slate-800/30'
            }`}
          >
            <MessageSquare size={16} />
            Chat
          </button>
          <button
            onClick={() => setActiveTab('logs')}
            className={`flex-1 px-4 py-3 text-sm font-medium flex items-center justify-center gap-2 transition-colors ${
              activeTab === 'logs'
                ? 'text-indigo-400 border-b-2 border-indigo-400 bg-slate-800/50'
                : 'text-slate-400 hover:text-slate-300 hover:bg-slate-800/30'
            }`}
          >
            <Terminal size={16} />
            Logs
          </button>
        </div>

        {/* Content Area */}
        <div className="flex-1 overflow-hidden relative bg-slate-900/20">
          {/* Chat View */}
          <div className={`absolute inset-0 flex flex-col overflow-y-auto p-4 space-y-4 ${activeTab === 'chat' ? 'z-10' : 'z-0 hidden'}`}>
            {/* Task Description */}
            {task.description && (
              <div className="bg-slate-700/50 rounded-lg p-3 text-sm text-slate-300">
                <div className="text-xs text-slate-500 mb-1">任务描述</div>
                {task.description}
              </div>
            )}

            {/* Worktree Info */}
            {executionInfo && executionInfo.worktreePath && (
              <div className="bg-indigo-500/10 border border-indigo-500/20 rounded-lg p-3 text-sm">
                <div className="flex items-center gap-2 text-indigo-400 mb-2">
                  <GitBranch size={14} />
                  <span className="font-medium">隔离执行环境</span>
                </div>
                <div className="space-y-1 text-slate-300">
                  <div className="flex items-center gap-2">
                    <span className="text-slate-500">分支:</span>
                    <code className="bg-slate-700 px-1.5 py-0.5 rounded text-xs">{executionInfo.branch}</code>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-slate-500">状态:</span>
                    <span className={executionInfo.state.includes('running') ? 'text-indigo-400' : 
                      executionInfo.state.includes('completed') ? 'text-emerald-400' : 
                      executionInfo.state.includes('failed') ? 'text-rose-400' : 'text-slate-400'}>
                      {executionInfo.state}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-slate-500">
                    <span>路径:</span>
                    <code className="bg-slate-700 px-1.5 py-0.5 rounded">{executionInfo.worktreePath}</code>
                  </div>
                </div>
                {/* Cleanup button for completed/failed sessions */}
                {onCleanupWorktree && (executionInfo.state.includes('completed') || executionInfo.state.includes('failed') || executionInfo.state.includes('cancelled')) && (
                  <button
                    type="button"
                    onClick={() => onCleanupWorktree(task.id)}
                    className="mt-2 flex items-center gap-1.5 text-xs text-rose-400 hover:text-rose-300 transition-colors"
                  >
                    <Trash2 size={12} />
                    清理 Worktree
                  </button>
                )}
              </div>
            )}

            {/* No History Yet */}
            {!history && !isLoading && (
              <div className="text-center py-8 text-slate-500">
                <p>点击"开始执行"让 AI 处理这个任务</p>
              </div>
            )}

            {/* Loading */}
            {isLoading && !history && (
              <div className="flex items-center justify-center py-8">
                <Loader2 size={24} className="animate-spin text-indigo-400" />
              </div>
            )}

            {/* Messages */}
            {history?.messages.map((message) => (
              <MessageBubble key={message.id} message={message} />
            ))}

            {/* Error */}
            {error && (
              <div className="bg-rose-500/10 border border-rose-500/20 rounded-lg p-3 text-sm text-rose-400">
                {error}
              </div>
            )}

            <div ref={messagesEndRef} />
          </div>

          {/* Logs View */}
          <div className={`absolute inset-0 ${activeTab === 'logs' ? 'z-10' : 'z-0 hidden'}`}>
            <ExecutionLogPanel taskId={task.id} />
          </div>
        </div>

        {/* Actions Footer */}
        <div className="p-4 border-t border-slate-700 bg-slate-800 z-20 space-y-3">
          {/* Control Buttons */}
          <div className="flex gap-2">
            {canExecute && (
              <button
                type="button"
                onClick={() => onExecute(task.id)}
                disabled={isLoading}
                className="flex items-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg font-medium transition-colors disabled:opacity-50"
              >
                <Play size={16} />
                开始执行
              </button>
            )}
            {isRunning && (
              <button
                type="button"
                onClick={() => onStop(task.id)}
                className="flex items-center gap-2 px-4 py-2 bg-rose-600 hover:bg-rose-500 text-white rounded-lg font-medium transition-colors"
              >
                <Square size={16} />
                停止
              </button>
            )}
          </div>

          {/* Message Input (only when running) */}
          {isRunning && (
            <form onSubmit={handleSubmit} className="flex gap-2">
              <input
                type="text"
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                placeholder="发送消息给 AI..."
                className="flex-1 bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
              <button
                type="submit"
                disabled={!inputValue.trim()}
                className="p-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg transition-colors disabled:opacity-50"
              >
                <Send size={18} />
              </button>
            </form>
          )}
        </div>
      </div>
    </div>
  );
}

// Message Bubble Component
function MessageBubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';

  return (
    <div
      className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}
    >
      <div
        className={`max-w-[80%] rounded-lg p-3 ${
          isUser
            ? 'bg-indigo-600 text-white'
            : isSystem
            ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
            : 'bg-slate-700 text-slate-200'
        }`}
      >
        <div className="text-sm whitespace-pre-wrap break-words">
          {message.content}
        </div>
        <div className={`text-xs mt-1 ${isUser ? 'text-indigo-200' : 'text-slate-500'}`}>
          {new Date(message.timestamp).toLocaleTimeString()}
        </div>
      </div>
    </div>
  );
}
