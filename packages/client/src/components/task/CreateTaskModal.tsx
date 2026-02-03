/**
 * Create Task Modal
 *
 * Modal dialog for creating new tasks with a full form UI.
 * Supports creating a task and optionally starting it immediately.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { X, Plus, Play, Loader2, AlertCircle, ChevronDown, GitBranch, Bot, Server, Cpu, Search } from 'lucide-react';
import type { CreateTaskRequest } from '../../hooks/useTaskApi';
import type { AgentType } from '@opencode-vibe/protocol';
import { useHosts, AUTO_HOST } from '../../hooks/useHosts';
import { useModels } from '../../hooks/useModels';

interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (data: CreateTaskRequest) => Promise<boolean>;
  onCreateAndStart?: (data: CreateTaskRequest) => Promise<boolean>;
  isLoading?: boolean;
  error?: string | null;
}

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
  const [agentType, setAgentType] = useState<AgentType>('opencode');
  const [isAgentOpen, setIsAgentOpen] = useState(false);
  const [baseBranch, setBaseBranch] = useState('main');
  const [targetHost, setTargetHost] = useState<string>(AUTO_HOST);
  const [isHostOpen, setIsHostOpen] = useState(false);
  const [model, setModel] = useState<string>('');
  const [isModelOpen, setIsModelOpen] = useState(false);
  const [modelSearch, setModelSearch] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  // Fetch available hosts
  const { hosts, isLoading: hostsLoading, getHostsForAgent } = useHosts();
  const availableHosts = getHostsForAgent(agentType);
  
  // Fetch available models for selected host
  const { modelOptions, isLoading: modelsLoading, fetchModels, clearModels } = useModels();

  // Filter models by search term
  const filteredModelOptions = useMemo(() => {
    if (!modelSearch.trim()) return modelOptions;
    const search = modelSearch.toLowerCase();
    return modelOptions.filter(option => 
      option.model.name.toLowerCase().includes(search) ||
      option.provider.toLowerCase().includes(search) ||
      option.value.toLowerCase().includes(search)
    );
  }, [modelOptions, modelSearch]);

  // Fetch models when host changes
  useEffect(() => {
    if (targetHost && targetHost !== AUTO_HOST) {
      fetchModels(targetHost);
    } else {
      clearModels();
      setModel('');
    }
    setModelSearch('');
  }, [targetHost, fetchModels, clearModels]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setTitle('');
      setDescription('');
      setAgentType('opencode');
      setBaseBranch('main');
      setTargetHost(AUTO_HOST);
      setModel('');
      setModelSearch('');
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
      priority: 'medium', // Default priority
      agentType,
      baseBranch: baseBranch.trim() || 'main',
      targetHost: targetHost !== AUTO_HOST ? targetHost : undefined,
      model: model || undefined,
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
      priority: 'medium', // Default priority
      agentType,
      baseBranch: baseBranch.trim() || 'main',
      targetHost: targetHost !== AUTO_HOST ? targetHost : undefined,
      model: model || undefined,
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

          {/* Target Host */}
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1.5">
              <Server size={14} className="inline mr-1.5" />
              Target Host
              {hostsLoading && <span className="text-slate-500 ml-2">(loading...)</span>}
            </label>
            <div className="relative">
              <button
                type="button"
                onClick={() => setIsHostOpen(!isHostOpen)}
                className="w-full flex items-center justify-between bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
                disabled={isLoading}
              >
                <span className={targetHost === AUTO_HOST ? 'text-slate-400' : 'text-emerald-400'}>
                  {targetHost === AUTO_HOST 
                    ? 'Auto (Local or Remote)' 
                    : (hosts.find(h => h.hostId === targetHost)?.name || targetHost)}
                </span>
                <ChevronDown size={16} className={`text-slate-400 transition-transform ${isHostOpen ? 'rotate-180' : ''}`} />
              </button>
              {isHostOpen && (
                <div className="absolute top-full left-0 right-0 mt-1 bg-slate-700 border border-slate-600 rounded-lg shadow-xl z-10 overflow-hidden max-h-48 overflow-y-auto">
                  {/* Auto option */}
                  <button
                    type="button"
                    onClick={() => {
                      setTargetHost(AUTO_HOST);
                      setIsHostOpen(false);
                    }}
                    className={`w-full px-3 py-2.5 text-left hover:bg-slate-600 transition-colors ${
                      targetHost === AUTO_HOST ? 'bg-slate-600' : ''
                    }`}
                  >
                    <div className="text-slate-400">Auto (Local or Remote)</div>
                    <div className="text-xs text-slate-500">Automatically select best available host</div>
                  </button>
                  
                  {/* Available remote hosts */}
                  {availableHosts.length > 0 ? (
                    availableHosts.map((host) => (
                      <button
                        key={host.hostId}
                        type="button"
                        onClick={() => {
                          setTargetHost(host.hostId);
                          setIsHostOpen(false);
                        }}
                        className={`w-full px-3 py-2.5 text-left hover:bg-slate-600 transition-colors ${
                          targetHost === host.hostId ? 'bg-slate-600' : ''
                        }`}
                      >
                        <div className="flex items-center gap-2">
                          <span className={`w-2 h-2 rounded-full ${
                            host.status === 'online' ? 'bg-emerald-400' : 
                            host.status === 'busy' ? 'bg-amber-400' : 'bg-slate-400'
                          }`} />
                          <span className="text-emerald-400">{host.name}</span>
                          <span className="text-xs text-slate-500">({host.hostId})</span>
                        </div>
                        <div className="text-xs text-slate-400 mt-0.5">
                          {host.capabilities.agents.join(', ')} | {host.activeTasks.length}/{host.capabilities.maxConcurrent} tasks
                        </div>
                      </button>
                    ))
                  ) : (
                    <div className="px-3 py-2.5 text-slate-500 text-sm">
                      No remote hosts available for {agentType}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>

          {/* Model Selector - only show when a specific host is selected */}
          {targetHost !== AUTO_HOST && (
            <div>
              <label className="block text-sm font-medium text-slate-300 mb-1.5">
                <Cpu size={14} className="inline mr-1.5" />
                Model
                {modelsLoading && <span className="text-slate-500 ml-2">(loading...)</span>}
              </label>
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setIsModelOpen(!isModelOpen)}
                  className="w-full flex items-center justify-between bg-slate-700 border border-slate-600 rounded-lg px-3 py-2.5 text-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  disabled={isLoading || modelsLoading}
                >
                  <span className={model ? 'text-cyan-400' : 'text-slate-400'}>
                    {model 
                      ? (modelOptions.find(m => m.value === model)?.label || model)
                      : 'Default (auto-select)'}
                  </span>
                  <ChevronDown size={16} className={`text-slate-400 transition-transform ${isModelOpen ? 'rotate-180' : ''}`} />
                </button>
                {isModelOpen && (
                  <div className="absolute top-full left-0 right-0 mt-1 bg-slate-700 border border-slate-600 rounded-lg shadow-xl z-10 overflow-hidden">
                    {/* Search input */}
                    <div className="p-2 border-b border-slate-600">
                      <div className="relative">
                        <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
                        <input
                          type="text"
                          value={modelSearch}
                          onChange={(e) => setModelSearch(e.target.value)}
                          placeholder="Search models..."
                          className="w-full bg-slate-600 border border-slate-500 rounded px-3 py-1.5 pl-8 text-sm text-white placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                          autoFocus
                        />
                      </div>
                    </div>
                    
                    <div className="max-h-48 overflow-y-auto">
                      {/* Default option - only show when not searching */}
                      {!modelSearch.trim() && (
                        <button
                          type="button"
                          onClick={() => {
                            setModel('');
                            setIsModelOpen(false);
                            setModelSearch('');
                          }}
                          className={`w-full px-3 py-2.5 text-left hover:bg-slate-600 transition-colors ${
                            !model ? 'bg-slate-600' : ''
                          }`}
                        >
                          <div className="text-slate-400">Default (auto-select)</div>
                          <div className="text-xs text-slate-500">Use the model configured on the gateway</div>
                        </button>
                      )}
                      
                      {/* Available models */}
                      {filteredModelOptions.length > 0 ? (
                        filteredModelOptions.map((option) => (
                          <button
                            key={option.value}
                            type="button"
                            onClick={() => {
                              setModel(option.value);
                              setIsModelOpen(false);
                              setModelSearch('');
                            }}
                            className={`w-full px-3 py-2.5 text-left hover:bg-slate-600 transition-colors ${
                              model === option.value ? 'bg-slate-600' : ''
                            }`}
                          >
                            <div className="text-cyan-400">{option.model.name}</div>
                            <div className="text-xs text-slate-400">{option.provider} - {option.value}</div>
                          </button>
                        ))
                      ) : (
                        !modelsLoading && (
                          <div className="px-3 py-2.5 text-slate-500 text-sm">
                            {modelSearch.trim() ? 'No matching models' : 'No models available from this host'}
                          </div>
                        )
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}
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
