import { useState, useRef, useEffect } from 'react';
import { X, Play, Square, Send, Loader2, CheckCircle, XCircle, Clock, GitBranch, Trash2, MessageSquare, Terminal, History } from 'lucide-react';
import type { KanbanTask, TaskSessionHistory, AgentSessionStatus } from '@opencode-vibe/protocol';
import { ExecutionLogPanel } from '../execution/ExecutionLogPanel';
import { RunHistoryPanel } from '../run/RunHistoryPanel';
import { useTaskRuns, type ChatMessage } from '../../hooks/useTaskRuns';
import { CONSOLE_LEXICON } from '../../lexicon/consoleLexicon';

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
  onSendInput?: (taskId: string, content: string) => Promise<boolean>;
}

const copy = CONSOLE_LEXICON.taskDetailPanel;

const statusConfig: Record<AgentSessionStatus, { icon: React.ReactNode; label: string; color: string }> = {
  idle: { icon: <Clock size={16} />, label: copy.statusLabels.idle, color: 'text-slate-400' },
  starting: { icon: <Loader2 size={16} className="animate-spin" />, label: copy.statusLabels.starting, color: 'text-amber-400' },
  running: { icon: <Loader2 size={16} className="animate-spin" />, label: copy.statusLabels.running, color: 'text-indigo-400' },
  paused: { icon: <Clock size={16} />, label: copy.statusLabels.paused, color: 'text-amber-400' },
  completed: { icon: <CheckCircle size={16} />, label: copy.statusLabels.completed, color: 'text-emerald-400' },
  failed: { icon: <XCircle size={16} />, label: copy.statusLabels.failed, color: 'text-rose-400' },
  aborted: { icon: <XCircle size={16} />, label: copy.statusLabels.aborted, color: 'text-slate-400' },
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
  onSendInput,
}: TaskDetailPanelProps) {
  const [inputValue, setInputValue] = useState('');
  const [activeTab, setActiveTab] = useState<'chat' | 'logs' | 'runs'>('chat');
  const [persistedMessages, setPersistedMessages] = useState<ChatMessage[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const currentStatus = status || 'idle';
  const statusInfo = statusConfig[currentStatus];

  // Load runs to get the most recent run's messages
  const { runs, loadMessages } = useTaskRuns(task.id);

  // Auto-switch to logs when execution starts
  useEffect(() => {
    if (status === 'starting' || status === 'running') {
      setActiveTab('logs');
    }
  }, [status]);

  // Load persisted messages from most recent run when no active session
  useEffect(() => {
    const loadPersistedMessages = async () => {
      // Only load persisted messages if there's no active history and there are runs
      if (!history?.messages?.length && runs.length > 0) {
        const mostRecentRun = runs[0]; // runs are sorted by created_at descending
        const messages = await loadMessages(mostRecentRun.id);
        setPersistedMessages(messages);
      } else {
        setPersistedMessages([]);
      }
    };

    void loadPersistedMessages();
  }, [history?.messages?.length, runs, loadMessages]);

  // Use active history messages or persisted messages
  const displayMessages = history?.messages?.length ? history.messages : persistedMessages;

  // 自动滚动到底部
  useEffect(() => {
    if (activeTab === 'chat') {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [displayMessages, activeTab]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputValue.trim()) return;
    onSendMessage(task.id, inputValue.trim());
    setInputValue('');
  };

  const isRunning = currentStatus === 'starting' || currentStatus === 'running';
  const canExecute = currentStatus === 'idle' || currentStatus === 'completed' || currentStatus === 'failed' || currentStatus === 'aborted';

  return (
    <div className="modal-overlay z-50">
      <div className="modal-overlay-inner">
        <div className="modal-shell modal-shell--detail">
        {/* Header */}
          <div className="modal-header">
            <div className="detail-head">
              <div className="flex-1 min-w-0">
                <h2 className="modal-title truncate">{task.title}</h2>
                <div className={`detail-status ${statusInfo.color}`}>
                  {statusInfo.icon}
                  <span>{statusInfo.label}</span>
                </div>
              </div>
              <button
                type="button"
                onClick={onClose}
                className="tech-btn tech-btn-secondary px-2.5 py-1.5"
                aria-label="关闭"
              >
                <X size={16} />
              </button>
            </div>
          </div>

        {/* Tabs */}
          <div className="detail-tabs">
          <button
            onClick={() => setActiveTab('chat')}
              className={`detail-tab ${activeTab === 'chat' ? 'detail-tab--active' : ''}`}
          >
            <MessageSquare size={16} />
            Chat
          </button>
          <button
            onClick={() => setActiveTab('logs')}
              className={`detail-tab ${activeTab === 'logs' ? 'detail-tab--active' : ''}`}
          >
            <Terminal size={16} />
            Logs
          </button>
          <button
            onClick={() => setActiveTab('runs')}
              className={`detail-tab ${activeTab === 'runs' ? 'detail-tab--active' : ''}`}
          >
            <History size={16} />
            Runs
          </button>
          </div>

        {/* Content Area */}
          <div className="detail-content">
          {/* Chat View */}
            <div className={`detail-content-scroll ${activeTab === 'chat' ? 'z-10' : 'z-0 hidden'}`}>
            {/* Task Description */}
            {task.description && (
                <div className="info-block">
                  <div className="info-title">{copy.blocks.missionBrief}</div>
                {task.description}
              </div>
            )}

            {/* Worktree Info */}
            {executionInfo && executionInfo.worktreePath && (
                <div className="info-block info-block--accent text-sm">
                  <div className="flex items-center gap-2 text-cyan-300 mb-2">
                  <GitBranch size={14} />
                  <span className="font-medium">{copy.blocks.isolatedWorktree}</span>
                </div>
                  <div className="space-y-1 text-slate-300">
                  <div className="flex items-center gap-2">
                    <span className="text-slate-500">{copy.blocks.branch}</span>
                      <code className="bg-slate-900/70 px-1.5 py-0.5 rounded text-xs">{executionInfo.branch}</code>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-slate-500">{copy.blocks.state}</span>
                      <span className={executionInfo.state.includes('running') ? 'text-cyan-300' : 
                      executionInfo.state.includes('completed') ? 'text-emerald-400' : 
                      executionInfo.state.includes('failed') ? 'text-rose-400' : 'text-slate-400'}>
                      {executionInfo.state}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-slate-500">
                    <span>{copy.blocks.path}</span>
                      <code className="bg-slate-900/70 px-1.5 py-0.5 rounded">{executionInfo.worktreePath}</code>
                  </div>
                </div>
                {/* Cleanup button for completed/failed sessions */}
                {onCleanupWorktree && (executionInfo.state.includes('completed') || executionInfo.state.includes('failed') || executionInfo.state.includes('cancelled')) && (
                  <button
                    type="button"
                    onClick={() => onCleanupWorktree(task.id)}
                      className="mt-2 flex items-center gap-1.5 text-xs text-rose-300 hover:text-rose-200 transition-colors"
                  >
                    <Trash2 size={12} />
                    {copy.actions.cleanupWorktree}
                  </button>
                )}
              </div>
            )}

            {/* No History Yet */}
            {!history && !isLoading && (
              <div className="text-center py-8 text-slate-500">
                <p>{copy.blocks.noHistory}</p>
              </div>
            )}

            {/* Loading */}
            {isLoading && !history && (
              <div className="flex items-center justify-center py-8">
                <Loader2 size={24} className="animate-spin text-indigo-400" />
              </div>
            )}

            {/* Messages */}
            {displayMessages.map((message) => (
              <MessageBubble key={message.id} message={message as any} />
            ))}

            {/* Error */}
            {error && (
                <div className="alert-error">
                {error}
              </div>
            )}

            <div ref={messagesEndRef} />
            </div>

          {/* Logs View */}
            <div className={`absolute inset-0 p-3 ${activeTab === 'logs' ? 'z-10' : 'z-0 hidden'}`}>
            <ExecutionLogPanel 
              taskId={task.id} 
              onSendInput={onSendInput}
              isRunning={isRunning}
            />
          </div>

          {/* Runs View */}
            <div className={`absolute inset-0 p-3 ${activeTab === 'runs' ? 'z-10' : 'z-0 hidden'}`}>
            <RunHistoryPanel taskId={task.id} />
          </div>
          </div>

        {/* Actions Footer */}
          <div className="modal-footer flex-col items-stretch gap-3">
          {/* Control Buttons */}
          <div className="flex gap-2">
            {canExecute && (
              <button
                type="button"
                onClick={() => onExecute(task.id)}
                disabled={isLoading}
                  className="tech-btn tech-btn-primary"
              >
                <Play size={16} />
                {copy.actions.execute}
              </button>
            )}
            {isRunning && (
              <button
                type="button"
                onClick={() => onStop(task.id)}
                  className="tech-btn tech-btn-danger"
              >
                <Square size={16} />
                {copy.actions.stop}
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
                placeholder={copy.placeholders.sendDirective}
                  className="glass-input"
              />
              <button
                type="submit"
                disabled={!inputValue.trim()}
                  className="tech-btn tech-btn-primary px-3"
              >
                <Send size={18} />
              </button>
            </form>
          )}
          </div>
        </div>
      </div>
    </div>
  );
}

// Message Bubble Component
function MessageBubble({ message }: { message: ChatMessage & { isStreaming?: boolean } }) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';
  const isStreaming = message.isStreaming;

  return (
    <div className={`message-row ${isUser ? 'message-row--user' : 'message-row--assistant'}`}>
      <div
        className={`message-bubble ${
          isUser
            ? 'message-bubble--user'
            : isSystem
            ? 'message-bubble--system'
            : 'message-bubble--assistant'
        }`}
      >
        <div className="text-sm whitespace-pre-wrap break-words">
          {isStreaming ? (
            <span className="flex items-center gap-2">
              <Loader2 size={14} className="animate-spin" />
              {message.content}
            </span>
          ) : (
            message.content
          )}
        </div>
        <div className="message-time">
          {new Date(message.timestamp).toLocaleTimeString()}
        </div>
      </div>
    </div>
  );
}
