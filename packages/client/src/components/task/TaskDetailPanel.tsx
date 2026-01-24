import { useState, useRef, useEffect } from 'react';
import { X, Play, Square, Send, Loader2, CheckCircle, XCircle, Clock } from 'lucide-react';
import type { KanbanTask, TaskSessionHistory, AgentSessionStatus, ChatMessage } from '@opencode-vibe/protocol';

interface TaskDetailPanelProps {
  task: KanbanTask;
  history: TaskSessionHistory | null;
  status: AgentSessionStatus | null;
  isLoading: boolean;
  error: string | null;
  onClose: () => void;
  onExecute: (taskId: string) => void;
  onStop: (taskId: string) => void;
  onSendMessage: (taskId: string, content: string) => void;
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
  onClose,
  onExecute,
  onStop,
  onSendMessage,
}: TaskDetailPanelProps) {
  const [inputValue, setInputValue] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const currentStatus = status || 'idle';
  const statusInfo = statusConfig[currentStatus];

  // 自动滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history?.messages]);

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
      <div className="bg-slate-800 rounded-xl shadow-2xl border border-slate-700 w-full max-w-2xl max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-slate-700">
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

        {/* Messages Area */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* Task Description */}
          {task.description && (
            <div className="bg-slate-700/50 rounded-lg p-3 text-sm text-slate-300">
              <div className="text-xs text-slate-500 mb-1">任务描述</div>
              {task.description}
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

        {/* Actions Footer */}
        <div className="p-4 border-t border-slate-700 space-y-3">
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
