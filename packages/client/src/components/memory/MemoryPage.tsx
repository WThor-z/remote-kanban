import { useCallback, useEffect, useMemo, useState } from 'react';
import { Pencil, Plus, RefreshCw, Trash2 } from 'lucide-react';
import { MemoryItemEditor } from './MemoryItemEditor';
import { MemorySettingsPanel } from './MemorySettingsPanel';
import { useHosts } from '../../hooks/useHosts';
import { useProjects } from '../../hooks/useProjects';
import { useWorkspaceScope } from '../../context/workspaceScopeContext';
import {
  useMemoryApi,
  type MemoryCreateInput,
  type MemoryItem,
  type MemoryKind,
  type MemoryScope,
  type MemorySettings,
  type MemoryUpdateInput,
} from '../../hooks/useMemoryApi';

const formatDate = (raw: string): string => {
  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) {
    return raw;
  }
  return date.toLocaleString();
};

const pageSize = 20;

export function MemoryPage() {
  const { activeWorkspaceId } = useWorkspaceScope();
  const [hostId, setHostId] = useState('');
  const [projectId, setProjectId] = useState('');
  const [scope, setScope] = useState<MemoryScope | ''>('');
  const [kind, setKind] = useState<MemoryKind | ''>('');
  const [search, setSearch] = useState('');
  const [enabledOnly, setEnabledOnly] = useState(true);
  const [page, setPage] = useState(1);
  const [settings, setSettings] = useState<MemorySettings | null>(null);
  const [items, setItems] = useState<MemoryItem[]>([]);
  const [editorMode, setEditorMode] = useState<'create' | 'edit' | null>(null);
  const [editingItem, setEditingItem] = useState<MemoryItem | null>(null);

  const {
    isLoading,
    error,
    clearError,
    getSettings,
    patchSettings,
    listItems,
    createItem,
    updateItem,
    deleteItem,
  } = useMemoryApi();
  const { hosts } = useHosts();
  const { projects } = useProjects({ workspaceId: activeWorkspaceId || undefined });

  const offset = useMemo(() => (page - 1) * pageSize, [page]);

  useEffect(() => {
    if (hostId || hosts.length === 0) {
      return;
    }
    const preferredHost = hosts.find((host) => host.status === 'online') ?? hosts[0];
    setHostId(preferredHost.hostId);
  }, [hostId, hosts]);

  useEffect(() => {
    if (projectId || projects.length === 0) {
      return;
    }
    const preferredProject =
      (hostId && projects.find((project) => project.gatewayId === hostId)) ?? projects[0];
    if (preferredProject) {
      setProjectId(preferredProject.id);
    }
  }, [hostId, projectId, projects]);

  useEffect(() => {
    if (!projectId) {
      return;
    }
    if (!projects.some((project) => project.id === projectId)) {
      setProjectId('');
    }
  }, [projectId, projects]);

  const refresh = useCallback(async () => {
    const loadedSettings = await getSettings(hostId || undefined);
    if (loadedSettings) {
      setSettings(loadedSettings);
    }
    const loadedItems = await listItems({
      hostId: hostId || undefined,
      projectId: projectId || undefined,
      scope: scope || undefined,
      kind: kind || undefined,
      search: search || undefined,
      enabledOnly,
      limit: pageSize,
      offset,
    });
    setItems(loadedItems);
  }, [enabledOnly, getSettings, hostId, kind, listItems, offset, projectId, scope, search]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleSaveSettings = async (patch: Partial<MemorySettings>) => {
    const next = await patchSettings(patch, hostId || undefined);
    if (next) {
      setSettings(next);
    }
  };

  const handleCreate = async (input: MemoryCreateInput) => {
    const created = await createItem(input);
    if (created) {
      setEditorMode(null);
      setEditingItem(null);
      await refresh();
    }
  };

  const handleUpdate = async (id: string, patch: MemoryUpdateInput) => {
    const updated = await updateItem(id, patch);
    if (updated) {
      setEditorMode(null);
      setEditingItem(null);
      await refresh();
    }
  };

  const handleDelete = async (item: MemoryItem) => {
    const ok = await deleteItem(item.id, item.hostId);
    if (ok) {
      await refresh();
    }
  };

  return (
    <section className="tech-panel memory-panel reveal reveal-2">
      <div className="section-bar">
        <h2 className="section-title">Memory</h2>
        <div className="memory-actions">
          <button
            type="button"
            className="tech-btn tech-btn-secondary"
            onClick={refresh}
            disabled={isLoading}
          >
            <RefreshCw size={14} className={isLoading ? 'animate-spin' : ''} /> Refresh
          </button>
          <button
            type="button"
            className="tech-btn tech-btn-primary"
            onClick={() => {
              setEditorMode('create');
              setEditingItem(null);
            }}
          >
            <Plus size={14} /> New Memory
          </button>
        </div>
      </div>

      {error && (
        <div className="alert-error">
          <div>{error}</div>
          <button type="button" className="tech-btn tech-btn-secondary" onClick={clearError}>
            Dismiss
          </button>
        </div>
      )}

      <div className="memory-layout">
        <div className="info-block info-block--accent">
          <h3 className="info-title">Settings</h3>
          <MemorySettingsPanel settings={settings} isLoading={isLoading} onSave={handleSaveSettings} />
        </div>

        <div className="info-block">
          <h3 className="info-title">Filter</h3>
          <div className="memory-filter-grid">
            <label className="field">
              <span className="field-label">Host ID</span>
              <input className="glass-input" value={hostId} onChange={(event) => setHostId(event.target.value)} />
            </label>
            <label className="field">
              <span className="field-label">Project ID</span>
              <input
                className="glass-input"
                value={projectId}
                onChange={(event) => setProjectId(event.target.value)}
              />
            </label>
            <label className="field">
              <span className="field-label">Scope</span>
              <select className="glass-select" value={scope} onChange={(event) => setScope(event.target.value as MemoryScope | '')}>
                <option value="">all</option>
                <option value="project">project</option>
                <option value="host">host</option>
              </select>
            </label>
            <label className="field">
              <span className="field-label">Kind</span>
              <select className="glass-select" value={kind} onChange={(event) => setKind(event.target.value as MemoryKind | '')}>
                <option value="">all</option>
                <option value="preference">preference</option>
                <option value="constraint">constraint</option>
                <option value="fact">fact</option>
                <option value="workflow">workflow</option>
              </select>
            </label>
            <label className="field memory-filter-grid__search">
              <span className="field-label">Search</span>
              <input className="glass-input" value={search} onChange={(event) => setSearch(event.target.value)} />
            </label>
            <label className="memory-toggle">
              <input type="checkbox" checked={enabledOnly} onChange={(event) => setEnabledOnly(event.target.checked)} />
              <span>Enabled Only</span>
            </label>
          </div>

          <div className="memory-pager">
            <button
              type="button"
              className="tech-btn tech-btn-secondary"
              onClick={() => setPage((prev) => Math.max(1, prev - 1))}
              disabled={page <= 1 || isLoading}
            >
              Prev
            </button>
            <span className="section-note">Page {page}</span>
            <button
              type="button"
              className="tech-btn tech-btn-secondary"
              onClick={() => setPage((prev) => prev + 1)}
              disabled={isLoading || items.length < pageSize}
            >
              Next
            </button>
          </div>
        </div>

        {(editorMode === 'create' || editorMode === 'edit') && (
          <div className="info-block">
            <h3 className="info-title">{editorMode === 'create' ? 'Create Memory' : 'Edit Memory'}</h3>
            <MemoryItemEditor
              mode={editorMode}
              hostId={hostId}
              initial={editingItem}
              isLoading={isLoading}
              onCancel={() => {
                setEditorMode(null);
                setEditingItem(null);
              }}
              onCreate={handleCreate}
              onUpdate={handleUpdate}
            />
          </div>
        )}

        <div className="memory-list">
          {items.length === 0 ? (
            <div className="kanban-empty">No memory items matched the current filter.</div>
          ) : (
            items.map((item) => (
              <article key={item.id} className="memory-card">
                <header className="memory-card__head">
                  <div className="memory-card__chips">
                    <span className="command-chip">{item.scope}</span>
                    <span className="command-chip">{item.kind}</span>
                    {item.pinned && <span className="command-chip">pinned</span>}
                    {!item.enabled && <span className="command-chip">disabled</span>}
                  </div>
                  <div className="memory-card__tools">
                    <button
                      type="button"
                      className="task-card__icon-btn"
                      onClick={() => {
                        setEditorMode('edit');
                        setEditingItem(item);
                      }}
                      title="Edit memory item"
                    >
                      <Pencil size={15} />
                    </button>
                    <button
                      type="button"
                      className="task-card__icon-btn"
                      onClick={() => handleDelete(item)}
                      title="Delete memory item"
                    >
                      <Trash2 size={15} />
                    </button>
                  </div>
                </header>
                <p className="memory-card__content">{item.content}</p>
                <div className="memory-card__meta">
                  <span>host={item.hostId}</span>
                  {item.projectId && <span>project={item.projectId}</span>}
                  <span>source={item.source}</span>
                  <span>confidence={item.confidence.toFixed(2)}</span>
                  <span>hits={item.hitCount}</span>
                  <span>updated={formatDate(item.updatedAt)}</span>
                </div>
              </article>
            ))
          )}
        </div>
      </div>
    </section>
  );
}
