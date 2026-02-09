import { useMemo, useState } from 'react';
import type {
  MemoryCreateInput,
  MemoryItem,
  MemoryKind,
  MemoryScope,
  MemoryUpdateInput,
} from '../../hooks/useMemoryApi';

interface MemoryItemEditorProps {
  mode: 'create' | 'edit';
  hostId: string;
  initial?: MemoryItem | null;
  isLoading: boolean;
  onCancel: () => void;
  onCreate: (input: MemoryCreateInput) => Promise<void>;
  onUpdate: (id: string, patch: MemoryUpdateInput) => Promise<void>;
}

const kinds: MemoryKind[] = ['preference', 'constraint', 'fact', 'workflow'];
const scopes: MemoryScope[] = ['project', 'host'];

export function MemoryItemEditor({
  mode,
  hostId,
  initial,
  isLoading,
  onCancel,
  onCreate,
  onUpdate,
}: MemoryItemEditorProps) {
  const seed = useMemo(
    () => ({
      scope: initial?.scope ?? 'project',
      kind: initial?.kind ?? 'fact',
      content: initial?.content ?? '',
      tags: initial?.tags.join(', ') ?? '',
      confidence: initial?.confidence ?? 0.8,
      pinned: initial?.pinned ?? false,
      enabled: initial?.enabled ?? true,
      projectId: initial?.projectId ?? '',
      host: initial?.hostId ?? hostId,
    }),
    [hostId, initial]
  );

  const [scope, setScope] = useState<MemoryScope>(seed.scope);
  const [kind, setKind] = useState<MemoryKind>(seed.kind);
  const [content, setContent] = useState(seed.content);
  const [tags, setTags] = useState(seed.tags);
  const [confidence, setConfidence] = useState(seed.confidence);
  const [pinned, setPinned] = useState(seed.pinned);
  const [enabled, setEnabled] = useState(seed.enabled);
  const [projectId, setProjectId] = useState(seed.projectId);
  const [host, setHost] = useState(seed.host);

  const tagList = tags
    .split(',')
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0);

  const submit = async () => {
    if (!content.trim()) {
      return;
    }

    if (mode === 'create') {
      await onCreate({
        hostId: host.trim(),
        projectId: projectId.trim() || undefined,
        scope,
        kind,
        content: content.trim(),
        tags: tagList,
        confidence,
        pinned,
        enabled,
      });
      return;
    }

    if (!initial) {
      return;
    }

    await onUpdate(initial.id, {
      hostId: host.trim(),
      scope,
      kind,
      content: content.trim(),
      tags: tagList,
      confidence,
      pinned,
      enabled,
    });
  };

  return (
    <div className="memory-editor">
      <div className="memory-editor__grid">
        <label className="field">
          <span className="field-label">Host ID</span>
          <input className="glass-input" value={host} onChange={(event) => setHost(event.target.value)} />
        </label>
        <label className="field">
          <span className="field-label">Project ID</span>
          <input
            className="glass-input"
            value={projectId}
            onChange={(event) => setProjectId(event.target.value)}
            placeholder="Optional for scope=host"
          />
        </label>
        <label className="field">
          <span className="field-label">Scope</span>
          <select className="glass-select" value={scope} onChange={(event) => setScope(event.target.value as MemoryScope)}>
            {scopes.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="field-label">Kind</span>
          <select className="glass-select" value={kind} onChange={(event) => setKind(event.target.value as MemoryKind)}>
            {kinds.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="field-label">Tags (comma separated)</span>
          <input className="glass-input" value={tags} onChange={(event) => setTags(event.target.value)} />
        </label>
        <label className="field">
          <span className="field-label">Confidence (0-1)</span>
          <input
            className="glass-input"
            type="number"
            min={0}
            max={1}
            step={0.01}
            value={confidence}
            onChange={(event) => setConfidence(Number.parseFloat(event.target.value))}
          />
        </label>
      </div>

      <label className="field">
        <span className="field-label">Content</span>
        <textarea
          className="glass-textarea"
          value={content}
          onChange={(event) => setContent(event.target.value)}
          placeholder="Durable memory content..."
        />
      </label>

      <div className="memory-editor__flags">
        <label className="memory-toggle">
          <input type="checkbox" checked={pinned} onChange={(event) => setPinned(event.target.checked)} />
          <span>Pinned</span>
        </label>
        <label className="memory-toggle">
          <input type="checkbox" checked={enabled} onChange={(event) => setEnabled(event.target.checked)} />
          <span>Enabled</span>
        </label>
      </div>

      <div className="memory-editor__actions">
        <button type="button" className="tech-btn tech-btn-secondary" onClick={onCancel} disabled={isLoading}>
          Cancel
        </button>
        <button type="button" className="tech-btn tech-btn-primary" onClick={submit} disabled={isLoading || !content.trim()}>
          {mode === 'create' ? 'Create Memory' : 'Update Memory'}
        </button>
      </div>
    </div>
  );
}
