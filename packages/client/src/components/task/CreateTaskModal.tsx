/**
 * Create Task Modal
 *
 * Modal dialog for creating new tasks with a full form UI.
 * Supports creating a task and optionally starting it immediately.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { X, Plus, Play, Loader2, AlertCircle, ChevronDown, GitBranch, Bot, Cpu, Search, FolderGit2, Layers } from 'lucide-react';
import type { CreateTaskRequest } from '../../hooks/useTaskApi';
import type { AgentType } from '@opencode-vibe/protocol';
import { useModels } from '../../hooks/useModels';
import { useProjects } from '../../hooks/useProjects';
import { useWorkspaces } from '../../hooks/useWorkspaces';
import { getConsoleLexiconSection } from '../../lexicon/consoleLexicon';

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
  const copy = getConsoleLexiconSection('createTaskModal');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [agentType, setAgentType] = useState<AgentType>('opencode');
  const [isAgentOpen, setIsAgentOpen] = useState(false);
  const [baseBranch, setBaseBranch] = useState('main');
  const [workspaceId, setWorkspaceId] = useState('');
  const [isWorkspaceOpen, setIsWorkspaceOpen] = useState(false);
  const [projectId, setProjectId] = useState('');
  const [isProjectOpen, setIsProjectOpen] = useState(false);
  const [model, setModel] = useState<string>('');
  const [isModelOpen, setIsModelOpen] = useState(false);
  const [modelSearch, setModelSearch] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  const { workspaces, isLoading: workspacesLoading, hasWorkspaces } = useWorkspaces();
  const selectedWorkspace = workspaces.find(w => w.id === workspaceId);
  const { projects, isLoading: projectsLoading, hasProjects } = useProjects({ workspaceId: workspaceId || undefined });
  const selectedProject = projects.find(p => p.id === projectId);

  // Fetch available models for the selected project's bound gateway host
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

  // Fetch models when project changes
  useEffect(() => {
    if (selectedProject) {
      fetchModels(selectedProject.gatewayId);
    } else {
      clearModels();
      setModel('');
    }
    setModelSearch('');
  }, [selectedProject, fetchModels, clearModels]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setTitle('');
      setDescription('');
      setAgentType('opencode');
      setBaseBranch('main');
      setWorkspaceId('');
      setIsWorkspaceOpen(false);
      setProjectId('');
      setIsProjectOpen(false);
      setModel('');
      setIsModelOpen(false);
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
      setLocalError(copy.errors.titleRequired);
      return false;
    }
    if (!projectId) {
      setLocalError(copy.errors.projectRequired);
      return false;
    }
    setLocalError(null);
    return true;
  }, [copy.errors.projectRequired, copy.errors.titleRequired, title, projectId]);

  const handleCreate = async () => {
    if (!validateForm()) return;

    const data: CreateTaskRequest = {
      title: title.trim(),
      projectId,
      description: description.trim() || undefined,
      priority: 'medium', // Default priority
      agentType,
      baseBranch: baseBranch.trim() || 'main',
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
      projectId,
      description: description.trim() || undefined,
      priority: 'medium', // Default priority
      agentType,
      baseBranch: baseBranch.trim() || 'main',
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
    <div className="modal-overlay z-50" onClick={handleBackdropClick}>
      <div className="modal-overlay-inner">
        <div className="modal-shell modal-shell--create w-full" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
          <div className="modal-header flex items-center justify-between">
            <h2 className="modal-title">
              <Plus size={18} className="text-cyan-300" />
              {copy.title}
          </h2>
          <button
            type="button"
            onClick={onClose}
              className="tech-btn tech-btn-secondary px-2.5 py-1.5"
            aria-label="Close"
          >
              <X size={16} />
          </button>
          </div>

        {/* Form */}
          <div className="modal-body modal-body--scroll">
          {/* Error Display */}
          {displayError && (
              <div className="alert-error flex items-center gap-2">
              <AlertCircle size={16} />
              {displayError}
            </div>
          )}

          {/* Title */}
            <div className="field">
              <label htmlFor="task-title" className="field-label">
                {copy.fields.title} <span className="text-rose-400">*</span>
            </label>
            <input
              id="task-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={copy.placeholders.title}
                className="glass-input"
              autoFocus
              disabled={isLoading}
            />
          </div>

          {/* Description */}
            <div className="field">
              <label htmlFor="task-description" className="field-label">
                {copy.fields.description} <span className="field-hint">(optional)</span>
            </label>
            <textarea
              id="task-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={copy.placeholders.description}
              rows={4}
                className="glass-textarea"
              disabled={isLoading}
            />
          </div>

          {/* Agent Type */}
            <div className="field">
              <label className="field-label">
              <Bot size={14} className="inline mr-1.5" />
                {copy.fields.agent}
            </label>
              <div className="dropdown-wrap">
              <button
                type="button"
                onClick={() => setIsAgentOpen(!isAgentOpen)}
                  className="glass-select flex items-center justify-between"
                disabled={isLoading}
              >
                  <span className="text-cyan-200">{agentOptions.find(a => a.value === agentType)?.label}</span>
                  <ChevronDown size={16} className={`text-slate-400 transition-transform ${isAgentOpen ? 'rotate-180' : ''}`} />
              </button>
              {isAgentOpen && (
                  <div className="dropdown-panel">
                  {agentOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => {
                        setAgentType(option.value);
                        setIsAgentOpen(false);
                      }}
                        className={`dropdown-item ${agentType === option.value ? 'dropdown-item--active' : ''}`}
                    >
                        <div className="text-cyan-200">{option.label}</div>
                        <div className="dropdown-note">{option.description}</div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Base Branch */}
            <div className="field">
              <label htmlFor="task-branch" className="field-label">
              <GitBranch size={14} className="inline mr-1.5" />
                {copy.fields.branch}
            </label>
            <input
              id="task-branch"
              type="text"
              value={baseBranch}
              onChange={(e) => setBaseBranch(e.target.value)}
              placeholder={copy.placeholders.branch}
                className="glass-input"
              disabled={isLoading}
            />
          </div>

          {/* Workspace */}
            <div className="field">
              <label className="field-label">
              <Layers size={14} className="inline mr-1.5" />
                {copy.fields.workspace}
              {workspacesLoading && <span className="text-slate-500 ml-2">(loading...)</span>}
            </label>
              <div className="dropdown-wrap">
              <button
                type="button"
                onClick={() => setIsWorkspaceOpen(!isWorkspaceOpen)}
                  className="glass-select flex items-center justify-between"
                disabled={isLoading || !hasWorkspaces}
              >
                <span className={selectedWorkspace ? 'text-emerald-300' : 'text-slate-400'}>
                  {selectedWorkspace
                    ? selectedWorkspace.name
                    : (hasWorkspaces ? copy.placeholders.workspace : copy.errors.noWorkspacesAvailable)}
                </span>
                <ChevronDown size={16} className={`text-slate-400 transition-transform ${isWorkspaceOpen ? 'rotate-180' : ''}`} />
              </button>
              {isWorkspaceOpen && (
                  <div className="dropdown-panel dropdown-panel--scroll">
                    <button
                      type="button"
                      onClick={() => {
                        setWorkspaceId('');
                        setProjectId('');
                        setModel('');
                        setModelSearch('');
                        setIsWorkspaceOpen(false);
                        setIsProjectOpen(false);
                        clearModels();
                      }}
                      className={`dropdown-item ${!workspaceId ? 'dropdown-item--active' : ''}`}
                    >
                      <div className="text-slate-200">{copy.placeholders.workspaceAny}</div>
                      <div className="dropdown-note">{copy.placeholders.workspaceAnyHint}</div>
                    </button>
                  {workspaces.length > 0 ? (
                    workspaces.map((workspace) => (
                      <button
                        key={workspace.id}
                        type="button"
                        onClick={() => {
                          setWorkspaceId(workspace.id);
                          setProjectId('');
                          setModel('');
                          setModelSearch('');
                          setIsWorkspaceOpen(false);
                          setIsProjectOpen(false);
                          clearModels();
                        }}
                          className={`dropdown-item ${workspaceId === workspace.id ? 'dropdown-item--active' : ''}`}
                      >
                          <div className="text-emerald-300">{workspace.name}</div>
                          <div className="dropdown-note mt-0.5">
                          {workspace.rootPath}
                        </div>
                      </button>
                    ))
                  ) : (
                    <div className="px-3 py-2.5 text-slate-500 text-sm">
                      {copy.errors.noWorkspaces}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>

          {/* Project */}
            <div className="field">
              <label className="field-label">
              <FolderGit2 size={14} className="inline mr-1.5" />
                {copy.fields.project} <span className="text-rose-400">*</span>
              {projectsLoading && <span className="text-slate-500 ml-2">(loading...)</span>}
            </label>
              <div className="dropdown-wrap">
              <button
                type="button"
                onClick={() => setIsProjectOpen(!isProjectOpen)}
                  className="glass-select flex items-center justify-between"
                disabled={isLoading}
              >
                <span className={selectedProject ? 'text-emerald-300' : 'text-slate-400'}>
                  {selectedProject ? selectedProject.name : (hasProjects ? copy.placeholders.project : copy.errors.noProjectsAvailable)}
                </span>
                <ChevronDown size={16} className={`text-slate-400 transition-transform ${isProjectOpen ? 'rotate-180' : ''}`} />
              </button>
              {isProjectOpen && (
                  <div className="dropdown-panel dropdown-panel--scroll">
                  {projects.length > 0 ? (
                    projects.map((project) => (
                      <button
                        key={project.id}
                        type="button"
                        onClick={() => {
                          setProjectId(project.id);
                          setIsProjectOpen(false);
                        }}
                          className={`dropdown-item ${projectId === project.id ? 'dropdown-item--active' : ''}`}
                      >
                          <div className="text-emerald-300">{project.name}</div>
                          <div className="dropdown-note mt-0.5">
                          {project.localPath} | gateway: {project.gatewayId}
                        </div>
                      </button>
                    ))
                  ) : (
                    <div className="px-3 py-2.5 text-slate-500 text-sm">
                      {copy.errors.noProjects}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>

          {/* Model Selector - available after project selection */}
          {selectedProject && (
              <div className="field">
                <label className="field-label">
                <Cpu size={14} className="inline mr-1.5" />
                  {copy.fields.model}
                {modelsLoading && <span className="text-slate-500 ml-2">(loading...)</span>}
              </label>
                <div className="dropdown-wrap">
                <button
                  type="button"
                  onClick={() => setIsModelOpen(!isModelOpen)}
                    className="glass-select flex items-center justify-between"
                  disabled={isLoading || modelsLoading}
                >
                    <span className={model ? 'text-cyan-200' : 'text-slate-400'}>
                    {model 
                      ? (modelOptions.find(m => m.value === model)?.label || model)
                      : copy.placeholders.modelDefault}
                  </span>
                  <ChevronDown size={16} className={`text-slate-400 transition-transform ${isModelOpen ? 'rotate-180' : ''}`} />
                </button>
                {isModelOpen && (
                    <div className="dropdown-panel">
                    {/* Search input */}
                      <div className="p-2 border-b border-slate-700">
                      <div className="relative">
                        <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
                        <input
                          type="text"
                          value={modelSearch}
                          onChange={(e) => setModelSearch(e.target.value)}
                          placeholder={copy.placeholders.modelSearch}
                            className="glass-input pl-8 text-sm"
                          autoFocus
                        />
                      </div>
                    </div>
                    
                      <div className="dropdown-panel--scroll">
                      {/* Default option - only show when not searching */}
                      {!modelSearch.trim() && (
                        <button
                          type="button"
                          onClick={() => {
                            setModel('');
                            setIsModelOpen(false);
                            setModelSearch('');
                          }}
                            className={`dropdown-item ${!model ? 'dropdown-item--active' : ''}`}
                        >
                            <div className="text-slate-200">{copy.placeholders.modelDefault}</div>
                            <div className="dropdown-note">{copy.placeholders.modelDefaultHint}</div>
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
                              className={`dropdown-item ${model === option.value ? 'dropdown-item--active' : ''}`}
                          >
                              <div className="text-cyan-200">{option.model.name}</div>
                              <div className="dropdown-note">{option.provider} - {option.value}</div>
                          </button>
                        ))
                      ) : (
                        !modelsLoading && (
                            <div className="px-3 py-2.5 text-slate-500 text-sm">
                            {modelSearch.trim() ? copy.errors.noModelMatch : copy.errors.noModelFromHost}
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
          <div className="modal-footer">
          <button
            type="button"
            onClick={onClose}
              className="tech-btn tech-btn-secondary"
            disabled={isLoading}
          >
            {copy.actions.cancel}
          </button>

          <button
            type="button"
            onClick={handleCreate}
            disabled={isLoading || !title.trim() || !projectId}
              className="tech-btn tech-btn-secondary"
          >
            {isLoading ? (
              <Loader2 size={16} className="animate-spin" />
            ) : (
              <Plus size={16} />
            )}
            {copy.actions.create}
          </button>

          {onCreateAndStart && (
            <button
              type="button"
              onClick={handleCreateAndStart}
              disabled={isLoading || !title.trim() || !projectId}
                className="tech-btn tech-btn-primary"
            >
              {isLoading ? (
                <Loader2 size={16} className="animate-spin" />
              ) : (
                <Play size={16} />
              )}
              {copy.actions.dispatch}
            </button>
          )}
          </div>
        </div>
      </div>
    </div>
  );
}
