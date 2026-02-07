import type { ReactNode } from 'react';
import { FolderGit2, Layers, GitBranch } from 'lucide-react';
import type { Project } from '../../hooks/useProjects';

interface ProjectSidebarProps {
  projects: Project[];
  selectedProjectId: string;
  onSelect: (projectId: string) => void;
  counts: {
    total: number;
    unassigned: number;
    byProject: Record<string, number>;
  };
  isLoading?: boolean;
  error?: string | null;
}

export function ProjectSidebar({
  projects,
  selectedProjectId,
  onSelect,
  counts,
  isLoading = false,
  error,
}: ProjectSidebarProps) {
  const sortedProjects = [...projects].sort((a, b) => a.name.localeCompare(b.name));

  return (
    <div className="bg-slate-800/80 border border-slate-700/70 rounded-xl p-4 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-slate-200">
          <FolderGit2 size={16} className="text-amber-400" />
          <span className="font-semibold">Projects</span>
        </div>
        {isLoading && <span className="text-xs text-slate-500">Loading...</span>}
      </div>

      {error && (
        <div className="text-xs text-rose-400 bg-rose-500/10 border border-rose-500/20 rounded-lg p-2">
          {error}
        </div>
      )}

      <div className="space-y-2">
        <ProjectItem
          label="All Projects"
          icon={<Layers size={14} />}
          count={counts.total}
          active={selectedProjectId === 'all'}
          onClick={() => onSelect('all')}
        />
        <ProjectItem
          label="Unassigned"
          icon={<GitBranch size={14} />}
          count={counts.unassigned}
          active={selectedProjectId === 'none'}
          onClick={() => onSelect('none')}
        />
      </div>

      <div className="space-y-2">
        {sortedProjects.length === 0 && !isLoading && (
          <div className="text-xs text-slate-500">No projects registered.</div>
        )}
        {sortedProjects.map((project) => (
          <ProjectItem
            key={project.id}
            label={project.name}
            subLabel={project.localPath}
            count={counts.byProject[project.id] || 0}
            active={selectedProjectId === project.id}
            onClick={() => onSelect(project.id)}
          />
        ))}
      </div>
    </div>
  );
}

interface ProjectItemProps {
  label: string;
  subLabel?: string;
  icon?: ReactNode;
  count: number;
  active: boolean;
  onClick: () => void;
}

function ProjectItem({ label, subLabel, icon, count, active, onClick }: ProjectItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full text-left px-3 py-2 rounded-lg border transition-colors ${
        active
          ? 'bg-slate-700/80 border-slate-600 text-white'
          : 'bg-slate-900/40 border-slate-700/60 text-slate-300 hover:bg-slate-800/60'
      }`}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          {icon && <span className="text-slate-400">{icon}</span>}
          <span className="text-sm font-medium truncate">{label}</span>
        </div>
        <span className="text-xs text-slate-400 bg-slate-800/70 px-2 py-0.5 rounded-full">
          {count}
        </span>
      </div>
      {subLabel && (
        <div className="mt-1 text-xs text-slate-500 truncate">{subLabel}</div>
      )}
    </button>
  );
}
