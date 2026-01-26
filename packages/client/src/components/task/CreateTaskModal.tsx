/**
 * Create Task Modal
 *
 * Modal dialog for creating new tasks with a full form UI.
 * Supports creating a task and optionally starting it immediately.
 */

import { useState, useEffect, useCallback } from 'react';
import { X, Plus, Play, Loader2, AlertCircle, ChevronDown, GitBranch, Bot } from 'lucide-react';
import type { TaskPriority, CreateTaskRequest } from '../../hooks/useTaskApi';
import type { AgentType } from '@opencode-vibe/protocol';

interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (data: CreateTaskRequest) => Promise<boolean>;
  onCreateAndStart?: (data: CreateTaskRequest) => Promise<boolean>;
  isLoading?: boolean;
  error?: string | null;
}

const priorityOptions: { value: TaskPriority; label: string; color: string }[] = [
  { value: 'low', label: 'Low', color: 'text-slate-400' },
  { value: 'medium', label: 'Medium', color: 'text-amber-400' },
  { value: 'high', label: 'High', color: 'text-rose-400' },
];

const agentOptions: { value: AgentType; label: string; description: string }[] = [
  { value: 'opencode', label: 'OpenCode', description: 'SST OpenCode AI Agent' },
  { value: 'claude-code', label: 'Claude Code', description: 'Anthropic Claude Code' },
  { value: 'gemini-cli', label: 'Gemini CLI', description: 'Google Gemini CLI' },
  { value: 'codex', label: 'Codex', description: 'OpenAI Codex' },
];

export function CreateTaskModal({
  isOpen,
  onClose,
  onCreate,
  onCreateAndStart,
  isLoading = false,
  error,
}: CreateTaskModalProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState<TaskPriority>('medium');
  const [isPriorityOpen, setIsPriorityOpen] = useState(false);
  const [agentType, setAgentType] = useState<AgentType>('opencode');
  const [isAgentOpen, setIsAgentOpen] = useState(false);
  const [baseBranch, setBaseBranch] = useState('main');
  const [localError, setLocalError] = useState<string | null>(null);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setTitle('');
      setDescription('');
      setPriority('medium');
      setAgentType('opencode');
      setBaseBranch('main');
      setLocalError(null);
    }
  }, [isOpen]);

  // Handle keyboard shortcut to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const validateForm = useCallback((): boolean => {
    if (!title.trim()) {
      setLocalError('Title is required');
      return false;
    }
    setLocalError(null);
    return true;
  }, [title]);

  const handleCreate = async () => {
    if (!validateForm()) return;

    const data: CreateTaskRequest = {
      title: title.trim(),
      description: description.trim() || undefined,
      priority,
      agentType,
      baseBranch: baseBranch.trim() || 'main',
    };

    const success = await onCreate(data);
    if (success) {
      onClose();
    }
  };

  const handleCreateAndStart = async () => {
    if (!validateForm() || !onCreateAndStart) return;

    const data: CreateTaskRequest = {
      title: title.trim(),
      description: description.trim() || undefined,
      priority,
      agentType,
      baseBranch: baseBranch.trim() || 'main',
    };

    const success = await onCreateAndStart(data);
    if (success) {
      onClose();
    }
  };

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  const selectedPriority = priorityOptions.find(p => p.value === priority)!;
  const displayError = localError || error;

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
      onClick={handleBackdropClick}
    >
      <div className="bg-slate-800 rounded-xl shadow-2xl border border-slate-700 w-full max-w-lg">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-slate-700">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Plus size={20} className="text-indigo-400" />
            Create New Task
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="p-2 text-slate-400 hover:text-white transition-colors rounded-lg hover:bg-slate-700"
            aria-label="Close"
          >
            <X size={20} />
          </button>
        </div>

        {/* Form */}
        <div className="p-4 space-y-4">
          {/* Error Display */}
          {displayError && (
            <div className="flex items-center gap-2 p-3 bg-rose-500/10 border border-rose-500/20 rounded-lg text-rose-400 text-sm">
              <AlertCircle size={16} />
              {displayError}
            </div>
          )}

          {/* Title */}
          <div>
            <label htmlFor="task-title" className="block text-sm font-medium text-slate-300 mb-1.5">
              Title <span className="text-rose-400">*</span>
            </label>
            <input
              id="task-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Enter task title..."
              className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
              autoFocus
              disabled={isLoading}
            />
          </div>

          {/* Description */}
          <div>
            <label htmlFor="task-description" className="block text-sm font-medium text-slate-300 mb-1.5">
              Description <span className="text-slate-500">(optional)</span>
            </label>
            <textarea
              id="task-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Describe the task in detail..."
              rows={4}
              className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent resize-none"
              disabled={isLoading}
            />
          </div>

          {/* Priority */}
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1.5">
              Priority
            </label>
            <div className="relative">
              <button
                type="button"
                onClick={() => setIsPriorityOpen(!isPriorityOpen)}
                className="w-full flex items-center justify-between bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
                disabled={isLoading}
              >
                <span className={selectedPriority.color}>{selectedPriority.label}</span>
                <ChevronDown size={16} className={`text-slate-400 transition-transform ${isPriorityOpen ? 'rotate-180' : ''}`} />
              </button>
              {isPriorityOpen && (
                <div className="absolute top-full left-0 right-0 mt-1 bg-slate-700 border border-slate-600 rounded-lg shadow-xl z-10 overflow-hidden">
                  {priorityOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => {
                        setPriority(option.value);
                        setIsPriorityOpen(false);
                      }}
                      className={`w-full px-3 py-2.5 text-left hover:bg-slate-600 transition-colors ${option.color} ${
                        priority === option.value ? 'bg-slate-600' : ''
                      }`}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Agent Type */}
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1.5">
              <Bot size={14} className="inline mr-1.5" />
              Agent
            </label>
            <div className="relative">
              <button
                type="button"
                onClick={() => setIsAgentOpen(!isAgentOpen)}
                className="w-full flex items-center justify-between bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
                disabled={isLoading}
              >
                <span className="text-indigo-400">{agentOptions.find(a => a.value === agentType)?.label}</span>
                <ChevronDown size={16} className={`text-slate-400 transition-transform ${isAgentOpen ? 'rotate-180' : ''}`} />
              </button>
              {isAgentOpen && (
                <div className="absolute top-full left-0 right-0 mt-1 bg-slate-700 border border-slate-600 rounded-lg shadow-xl z-10 overflow-hidden">
                  {agentOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => {
                        setAgentType(option.value);
                        setIsAgentOpen(false);
                      }}
                      className={`w-full px-3 py-2.5 text-left hover:bg-slate-600 transition-colors ${
                        agentType === option.value ? 'bg-slate-600' : ''
                      }`}
                    >
                      <div className="text-indigo-400">{option.label}</div>
                      <div className="text-xs text-slate-400">{option.description}</div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Base Branch */}
          <div>
            <label htmlFor="task-branch" className="block text-sm font-medium text-slate-300 mb-1.5">
              <GitBranch size={14} className="inline mr-1.5" />
              Base Branch
            </label>
            <input
              id="task-branch"
              type="text"
              value={baseBranch}
              onChange={(e) => setBaseBranch(e.target.value)}
              placeholder="main"
              className="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
              disabled={isLoading}
            />
          </div>
        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-end gap-3 p-4 border-t border-slate-700">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-slate-300 hover:text-white transition-colors"
            disabled={isLoading}
          >
            Cancel
          </button>

          <button
            type="button"
            onClick={handleCreate}
            disabled={isLoading || !title.trim()}
            className="flex items-center gap-2 px-4 py-2 bg-slate-600 hover:bg-slate-500 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLoading ? (
              <Loader2 size={16} className="animate-spin" />
            ) : (
              <Plus size={16} />
            )}
            Create
          </button>

          {onCreateAndStart && (
            <button
              type="button"
              onClick={handleCreateAndStart}
              disabled={isLoading || !title.trim()}
              className="flex items-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isLoading ? (
                <Loader2 size={16} className="animate-spin" />
              ) : (
                <Play size={16} />
              )}
              Create & Start
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
