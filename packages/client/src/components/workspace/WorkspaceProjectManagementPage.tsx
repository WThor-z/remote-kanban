import { useMemo, useState } from 'react';
import { FolderGit2, FolderSearch, Plus } from 'lucide-react';

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

interface WorkspaceProjectManagementPageProps {
  workspaceId: string;
  language?: ConsoleLanguage;
}

export function WorkspaceProjectManagementPage({
  workspaceId,
  language = 'en',
}: WorkspaceProjectManagementPageProps) {
  const copy = getConsoleLanguageCopy(language).workspaceEntry;
  const { workspaces } = useWorkspaces();
  const selectedWorkspace = useMemo(
    () => workspaces.find((workspace) => workspace.id === workspaceId),
    [workspaceId, workspaces],
  );

  const {
    projects,
    isLoading,
    error,
    refresh,
    discoverWorkspaceProjects,
    createWorkspaceProject,
  } = useProjects({ workspaceId: workspaceId || undefined });

  const [newProjectName, setNewProjectName] = useState('');
  const [newProjectDefaultBranch, setNewProjectDefaultBranch] = useState('');
  const [newProjectRemoteUrl, setNewProjectRemoteUrl] = useState('');
  const [discoveredProjects, setDiscoveredProjects] = useState<DiscoveredWorkspaceProject[]>([]);
  const [localError, setLocalError] = useState<string | null>(null);
  const generatedProjectLocalPath = useMemo(
    () => buildWorkspaceProjectPath(selectedWorkspace?.rootPath ?? '', newProjectName),
    [newProjectName, selectedWorkspace?.rootPath],
  );

  const handleDiscoverProjects = async () => {
    if (!workspaceId) {
      setLocalError(copy.errors.selectWorkspaceFirst);
      return;
    }

    setLocalError(null);
    const discovered = await discoverWorkspaceProjects(workspaceId);
    setDiscoveredProjects(discovered);
  };

  const handleCreateProject = async () => {
    if (!workspaceId) {
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
    if (!selectedWorkspace?.rootPath.trim()) {
      setLocalError(copy.errors.workspaceRootUnavailable);
      return;
    }
    if (!generatedProjectLocalPath) {
      setLocalError(copy.errors.projectLocalPathRequired);
      return;
    }

    setLocalError(null);
    const created = await createWorkspaceProject(workspaceId, {
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
    await refresh();
  };

  return (
    <section className="tech-panel board-panel reveal reveal-2" data-testid="workspace-project-management-page">
      <div className="section-bar">
        <h2 className="section-title">{copy.projectsTitle}</h2>
        <div className="section-note">
          {selectedWorkspace
            ? `${copy.workspaceLabel}: ${selectedWorkspace.name}`
            : copy.projectsNote}
        </div>
      </div>

      {selectedWorkspace?.rootPath && (
        <div className="section-note">
          {copy.workspaceRootPathDisplay}: <span className="text-cyan-200">{selectedWorkspace.rootPath}</span>
        </div>
      )}

      {(localError || error) && (
        <div className="alert-error">{localError || error}</div>
      )}

      <div className="workspace-entry__grid">
        <div>
          <p className="section-note">{copy.projectsNote}</p>
          <div className="modal-footer workspace-entry__actions">
            <button
              type="button"
              className="tech-btn tech-btn-secondary"
              onClick={handleDiscoverProjects}
              disabled={!workspaceId || isLoading}
            >
              <FolderSearch size={14} /> {copy.discoverProjects}
            </button>
          </div>

          <div className="field">
            <label className="field-label" htmlFor="workspace-project-page-name">
              {copy.projectNameLabel} <span className="field-hint">({copy.fieldRequiredHint})</span>
            </label>
            <input
              id="workspace-project-page-name"
              className="glass-input"
              value={newProjectName}
              onChange={(event) => setNewProjectName(event.target.value)}
              placeholder="platform-api"
            />
          </div>

          <div className="field">
            <label className="field-label" htmlFor="workspace-project-page-path-preview">
              {copy.projectPathAutoLabel}
            </label>
            <input
              id="workspace-project-page-path-preview"
              className="glass-input"
              value={generatedProjectLocalPath}
              readOnly
            />
            <p className="section-note">{copy.projectPathAutoNote}</p>
          </div>

          <div className="field">
            <label className="field-label" htmlFor="workspace-project-page-branch">
              {copy.defaultBranchLabel} <span className="field-hint">({copy.fieldOptionalHint})</span>
            </label>
            <input
              id="workspace-project-page-branch"
              className="glass-input"
              value={newProjectDefaultBranch}
              onChange={(event) => setNewProjectDefaultBranch(event.target.value)}
              placeholder="main"
            />
          </div>

          <div className="field">
            <label className="field-label" htmlFor="workspace-project-page-remote">
              {copy.remoteUrlLabel} <span className="field-hint">({copy.fieldOptionalHint})</span>
            </label>
            <input
              id="workspace-project-page-remote"
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
            disabled={!workspaceId || isLoading}
          >
            <Plus size={14} /> {copy.createProject}
          </button>
        </div>

        <div>
          <h3 className="section-title">{copy.discoveredTitle}</h3>
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
      </div>
    </section>
  );
}
