import { useEffect, useMemo, useState } from 'react';
import { Layers, Server, Trash2, Plus, ArrowRight, RefreshCw, FolderSearch, FolderGit2 } from 'lucide-react';

import { useHosts } from '../../hooks/useHosts';
import { useProjects, type DiscoveredWorkspaceProject } from '../../hooks/useProjects';
import { useWorkspaces } from '../../hooks/useWorkspaces';
import {
  getConsoleLanguageCopy,
  type ConsoleLanguage,
} from '../../i18n/consoleLanguage';

const containsPathSeparator = (value: string): boolean => /[\\/]/.test(value);

const buildWorkspaceProjectPath = (workspaceRootPath: string, projectName: string): string => {
  const root = workspaceRootPath.trim();
  const name = projectName.trim();
  if (!root || !name) {
    return '';
  }

  const separator = root.includes('\\') ? '\\' : '/';
  const normalizedRoot = root.replace(/[\\/]+$/g, '');
  return `${normalizedRoot}${separator}${name}`;
};

interface WorkspaceEntryPageProps {
  selectedWorkspaceId: string;
  hasStaleStoredWorkspace: boolean;
  onSelectionChange: (workspaceId: string) => void;
  onContinue: () => void;
  allowProjectManagement?: boolean;
  language?: ConsoleLanguage;
  onLanguageToggle?: () => void;
}

export function WorkspaceEntryPage({
  selectedWorkspaceId,
  hasStaleStoredWorkspace,
  onSelectionChange,
  onContinue,
  allowProjectManagement = false,
  language = 'en',
  onLanguageToggle,
}: WorkspaceEntryPageProps) {
  const {
    workspaces,
    isLoading: workspacesLoading,
    error: workspaceError,
    refresh: refreshWorkspaces,
    createWorkspace,
    deleteWorkspace,
  } = useWorkspaces();
  const {
    hosts,
    isLoading: hostsLoading,
    error: hostsError,
    refresh: refreshHosts,
  } = useHosts();
  const {
    projects,
    isLoading: projectsLoading,
    error: projectsError,
    refresh: refreshProjects,
    discoverWorkspaceProjects,
    createWorkspaceProject,
  } = useProjects({ workspaceId: selectedWorkspaceId || undefined });

  const [hostId, setHostId] = useState('');
  const [manualHostId, setManualHostId] = useState('');
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [newWorkspaceRootPath, setNewWorkspaceRootPath] = useState('');
  const [newProjectName, setNewProjectName] = useState('');
  const [newProjectDefaultBranch, setNewProjectDefaultBranch] = useState('');
  const [newProjectRemoteUrl, setNewProjectRemoteUrl] = useState('');
  const [discoveredProjects, setDiscoveredProjects] = useState<DiscoveredWorkspaceProject[]>([]);
  const [deleteConfirmName, setDeleteConfirmName] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);
  const copy = getConsoleLanguageCopy(language).workspaceEntry;
  const hostStatusLabels = {
    en: {
      online: 'online',
      offline: 'offline',
    },
    zh: {
      online: '在线',
      offline: '离线',
    },
  } as const;

  useEffect(() => {
    if (!hostId && hosts.length > 0) {
      setHostId(hosts[0].hostId);
    }
  }, [hostId, hosts]);

  const selectedWorkspace = useMemo(
    () => workspaces.find((workspace) => workspace.id === selectedWorkspaceId),
    [selectedWorkspaceId, workspaces],
  );
  const generatedProjectLocalPath = useMemo(
    () => buildWorkspaceProjectPath(selectedWorkspace?.rootPath ?? '', newProjectName),
    [newProjectName, selectedWorkspace?.rootPath],
  );

  useEffect(() => {
    setDeleteConfirmName('');
  }, [selectedWorkspaceId]);

  useEffect(() => {
    setDiscoveredProjects([]);
  }, [selectedWorkspaceId]);

  const handleCreateWorkspace = async () => {
    const effectiveHostId = manualHostId.trim() || hostId;

    if (!newWorkspaceName.trim()) {
      setLocalError(copy.errors.workspaceNameRequired);
      return;
    }
    if (!newWorkspaceRootPath.trim()) {
      setLocalError(copy.errors.workspaceRootPathRequired);
      return;
    }
    if (!effectiveHostId) {
      setLocalError(copy.errors.hostRequired);
      return;
    }

    setLocalError(null);
    const created = await createWorkspace({
      name: newWorkspaceName.trim(),
      rootPath: newWorkspaceRootPath.trim(),
      hostId: effectiveHostId,
    });

    if (!created) {
      return;
    }

    setNewWorkspaceName('');
    setNewWorkspaceRootPath('');
    onSelectionChange(created.id);
    await refreshWorkspaces();
  };

  const handleDiscoverProjects = async () => {
    if (!selectedWorkspace) {
      setLocalError(copy.errors.selectWorkspaceFirst);
      return;
    }

    setLocalError(null);
    const discovered = await discoverWorkspaceProjects(selectedWorkspace.id);
    setDiscoveredProjects(discovered);
  };

  const handleCreateProject = async () => {
    if (!selectedWorkspace) {
      setLocalError(copy.errors.selectWorkspaceFirst);
      return;
    }
    if (!newProjectName.trim()) {
      setLocalError(copy.errors.projectNameRequired);
      return;
    }
    if (containsPathSeparator(newProjectName.trim())) {
      setLocalError(copy.errors.projectNameInvalid);
      return;
    }
    if (!selectedWorkspace.rootPath.trim()) {
      setLocalError(copy.errors.workspaceRootUnavailable);
      return;
    }
    if (!generatedProjectLocalPath) {
      setLocalError(copy.errors.projectLocalPathRequired);
      return;
    }

    setLocalError(null);
    const created = await createWorkspaceProject(selectedWorkspace.id, {
      name: newProjectName.trim(),
      localPath: generatedProjectLocalPath,
      defaultBranch: newProjectDefaultBranch.trim() || undefined,
      remoteUrl: newProjectRemoteUrl.trim() || undefined,
    });
    if (!created) {
      return;
    }

    setNewProjectName('');
    setNewProjectDefaultBranch('');
    setNewProjectRemoteUrl('');
    await refreshProjects();
  };

  const handleDeleteWorkspace = async () => {
    if (!selectedWorkspace) {
      setLocalError(copy.errors.selectWorkspaceToDelete);
      return;
    }

    if (deleteConfirmName !== selectedWorkspace.name) {
      setLocalError(copy.errors.workspaceDeleteNameMismatch);
      return;
    }

    setLocalError(null);
    const deleted = await deleteWorkspace(selectedWorkspace.id, { confirmName: deleteConfirmName });
    if (!deleted) {
      return;
    }

    onSelectionChange('');
    setDeleteConfirmName('');
    await refreshWorkspaces();
  };

  const handleRefresh = async () => {
    await Promise.all([refreshHosts(), refreshWorkspaces()]);
  };

  const canContinue = Boolean(selectedWorkspace);
  const canDelete = Boolean(selectedWorkspace) && deleteConfirmName === selectedWorkspace?.name;

  return (
    <div className="console-root">
      <div className="console-shell">
        <section className="tech-panel reveal">
          <div className="section-bar">
            <div className="flex items-center gap-2">
              <Layers size={16} className="text-cyan-300" />
              <h1 className="section-title">{copy.title}</h1>
            </div>
            <div className="flex items-center gap-2">
              {onLanguageToggle && (
                <button
                  type="button"
                  className="tech-btn tech-btn-secondary"
                  onClick={onLanguageToggle}
                  aria-label={getConsoleLanguageCopy(language).language.switchButtonAria}
                >
                  {getConsoleLanguageCopy(language).language.switchButtonLabel}
                </button>
              )}
              <button type="button" className="tech-btn tech-btn-secondary" onClick={handleRefresh}>
                <RefreshCw size={14} /> {copy.refresh}
              </button>
            </div>
          </div>

          <p className="section-note">{copy.subtitle}</p>
          {hasStaleStoredWorkspace && (
            <div className="alert-error">{copy.staleWorkspaceNotice}</div>
          )}
          {(localError || workspaceError || hostsError || projectsError) && (
            <div className="alert-error">{localError || workspaceError || hostsError || projectsError}</div>
          )}

          <div className="field">
            <label className="field-label" htmlFor="workspace-entry-selected">
              {copy.workspaceLabel}
            </label>
            <select
              id="workspace-entry-selected"
              className="glass-select"
              value={selectedWorkspaceId}
              onChange={(event) => onSelectionChange(event.target.value)}
              disabled={workspacesLoading}
              aria-label={copy.workspaceSelectPlaceholder}
            >
              <option value="">{copy.workspaceSelectPlaceholder}</option>
              {workspaces.map((workspace) => (
                <option key={workspace.id} value={workspace.id}>
                  {workspace.name}
                </option>
              ))}
            </select>
          </div>

          <div className="modal-footer workspace-entry__actions">
            <button
              type="button"
              className="tech-btn tech-btn-primary"
              onClick={onContinue}
              disabled={!canContinue}
            >
              <ArrowRight size={14} /> {copy.continue}
            </button>
          </div>
        </section>

        <section className="tech-panel reveal reveal-1 workspace-entry__grid">
          <div>
            <h2 className="section-title">{copy.createWorkspaceTitle}</h2>
            <div className="field">
              <label className="field-label" htmlFor="workspace-entry-host">
                <Server size={14} className="inline mr-1.5" /> {copy.hostLabel}
              </label>
              <select
                id="workspace-entry-host"
                className="glass-select"
                value={hostId}
                onChange={(event) => setHostId(event.target.value)}
                disabled={hostsLoading}
              >
                <option value="">{copy.hostSelectPlaceholder}</option>
                {hosts.map((host) => (
                <option key={host.hostId} value={host.hostId}>
                    {host.name} ({hostStatusLabels[language][host.status as 'online' | 'offline'] || host.status})
                </option>
              ))}
              </select>
              <p className="section-note">{copy.hostAutoNote}</p>
            </div>
            <div className="field">
              <label className="field-label" htmlFor="workspace-entry-manual-host">
                {copy.manualHostLabel}
              </label>
              <input
                id="workspace-entry-manual-host"
                className="glass-input"
                value={manualHostId}
                onChange={(event) => setManualHostId(event.target.value)}
                placeholder="host-custom-1"
              />
            </div>
            <div className="field">
              <label className="field-label" htmlFor="workspace-entry-name">
                {copy.workspaceNameLabel}
              </label>
              <input
                id="workspace-entry-name"
                className="glass-input"
                value={newWorkspaceName}
                onChange={(event) => setNewWorkspaceName(event.target.value)}
                placeholder="Platform Workspace"
              />
            </div>
            <div className="field">
              <label className="field-label" htmlFor="workspace-entry-root-path">
                {copy.rootPathLabel}
              </label>
              <input
                id="workspace-entry-root-path"
                className="glass-input"
                value={newWorkspaceRootPath}
                onChange={(event) => setNewWorkspaceRootPath(event.target.value)}
                placeholder="C:\\projects\\platform"
              />
            </div>
            <button type="button" className="tech-btn tech-btn-secondary" onClick={handleCreateWorkspace}>
              <Plus size={14} /> {copy.createWorkspace}
            </button>
          </div>

          <div>
            <h2 className="section-title">{copy.deleteWorkspaceTitle}</h2>
            <p className="section-note">{copy.deleteWorkspaceNote}</p>
            <div className="field">
              <label className="field-label" htmlFor="workspace-entry-delete-name">
                {copy.deleteWorkspaceInputLabel}
              </label>
              <input
                id="workspace-entry-delete-name"
                className="glass-input"
                value={deleteConfirmName}
                onChange={(event) => setDeleteConfirmName(event.target.value)}
                placeholder={selectedWorkspace?.name || copy.deleteWorkspaceInputPlaceholder}
                disabled={!selectedWorkspace}
              />
            </div>
            <button
              type="button"
              className="tech-btn tech-btn-danger"
              onClick={handleDeleteWorkspace}
              disabled={!canDelete}
              aria-label={copy.deleteWorkspaceButtonAria}
            >
              <Trash2 size={14} /> {copy.deleteWorkspace}
            </button>
          </div>
        </section>

        {allowProjectManagement && (
          <section className="tech-panel reveal reveal-2 workspace-entry__grid">
            <div>
              <h2 className="section-title">{copy.projectsTitle}</h2>
              <p className="section-note">{copy.projectsNote}</p>
              {selectedWorkspace?.rootPath && (
                <p className="section-note">
                  {copy.workspaceRootPathDisplay}: <span className="text-cyan-200">{selectedWorkspace.rootPath}</span>
                </p>
              )}
              <div className="modal-footer workspace-entry__actions">
                <button
                  type="button"
                  className="tech-btn tech-btn-secondary"
                  onClick={handleDiscoverProjects}
                  disabled={!selectedWorkspace || projectsLoading}
                >
                  <FolderSearch size={14} /> {copy.discoverProjects}
                </button>
              </div>

              <div className="field">
                <label className="field-label" htmlFor="workspace-entry-project-name">
                  {copy.projectNameLabel} <span className="field-hint">({copy.fieldRequiredHint})</span>
                </label>
                <input
                  id="workspace-entry-project-name"
                  className="glass-input"
                  value={newProjectName}
                  onChange={(event) => setNewProjectName(event.target.value)}
                  placeholder="platform-api"
                />
              </div>
              <div className="field">
                <label className="field-label" htmlFor="workspace-entry-project-path-preview">
                  {copy.projectPathAutoLabel}
                </label>
                <input
                  id="workspace-entry-project-path-preview"
                  className="glass-input"
                  value={generatedProjectLocalPath}
                  readOnly
                />
                <p className="section-note">{copy.projectPathAutoNote}</p>
              </div>
              <div className="field">
                <label className="field-label" htmlFor="workspace-entry-project-branch">
                  {copy.defaultBranchLabel} <span className="field-hint">({copy.fieldOptionalHint})</span>
                </label>
                <input
                  id="workspace-entry-project-branch"
                  className="glass-input"
                  value={newProjectDefaultBranch}
                  onChange={(event) => setNewProjectDefaultBranch(event.target.value)}
                  placeholder="main"
                />
              </div>
              <div className="field">
                <label className="field-label" htmlFor="workspace-entry-project-remote">
                  {copy.remoteUrlLabel} <span className="field-hint">({copy.fieldOptionalHint})</span>
                </label>
                <input
                  id="workspace-entry-project-remote"
                  className="glass-input"
                  value={newProjectRemoteUrl}
                  onChange={(event) => setNewProjectRemoteUrl(event.target.value)}
                  placeholder="https://example.com/repo.git"
                />
              </div>
              <button
                type="button"
                className="tech-btn tech-btn-secondary"
                onClick={handleCreateProject}
                disabled={!selectedWorkspace || projectsLoading}
              >
                <Plus size={14} /> {copy.createProject}
              </button>
            </div>

            <div>
              <h2 className="section-title">{copy.discoveredTitle}</h2>
              <p className="section-note">{copy.discoveredNote}</p>
              <div className="workspace-project-list" data-testid="workspace-project-list">
                {discoveredProjects.length > 0 ? (
                  discoveredProjects.map((project) => (
                    <div key={project.localPath} className="dropdown-item">
                      <div className="flex items-center gap-2 text-cyan-200">
                        <FolderGit2 size={14} />
                        <span>{project.name}</span>
                        {project.registeredProjectId ? <span className="command-chip">{copy.registeredTag}</span> : null}
                      </div>
                      <div className="dropdown-note">{project.localPath}</div>
                    </div>
                  ))
                ) : (
                  <div className="dropdown-item">
                    <div className="dropdown-note">{copy.discoveredEmpty}</div>
                  </div>
                )}
              </div>
              {projects.length > 0 && (
                <p className="section-note">{copy.registeredProjectsCount} {projects.length}</p>
              )}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
