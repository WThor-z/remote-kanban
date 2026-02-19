import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Activity,
  AlertTriangle,
  Database,
  PlayCircle,
  RefreshCw,
  Send,
  Server,
  Square,
  TableProperties,
} from 'lucide-react';
import type { OrchestratorExecutionEvent } from '@opencode-vibe/protocol';
import { useHosts } from '../../hooks/useHosts';
import {
  useOpsApi,
  type OpsAuditEvent,
  type OpsExecution,
  type OpsSummary,
} from '../../hooks/useOpsApi';
import { MemoryPage } from '../memory/MemoryPage';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';

interface OpsConsolePageProps {
  language?: ConsoleLanguage;
}

const formatDateTime = (value: string | null | undefined): string => {
  if (!value) {
    return '-';
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
};

const STATUS_OPTIONS = ['', 'running', 'completed', 'failed', 'cancelled'];

export function OpsConsolePage({ language = 'en' }: OpsConsolePageProps) {
  const [activeTab, setActiveTab] = useState<'hosts' | 'executions' | 'audit' | 'memory'>('hosts');
  const [statusFilter, setStatusFilter] = useState('');
  const [hostFilter, setHostFilter] = useState('');
  const [selectedExecutionId, setSelectedExecutionId] = useState<string | null>(null);
  const [inputDraft, setInputDraft] = useState('');

  const [executionEvents, setExecutionEvents] = useState<OrchestratorExecutionEvent[]>([]);
  const [summary, setSummary] = useState<OpsSummary | null>(null);
  const [executions, setExecutions] = useState<OpsExecution[]>([]);
  const [auditItems, setAuditItems] = useState<OpsAuditEvent[]>([]);

  const { hosts, isLoading: hostsLoading, error: hostsError, refresh: refreshHosts } = useHosts();
  const {
    isLoading,
    error,
    clearError,
    fetchSummary,
    listExecutions,
    listAuditEvents,
    getExecutionEvents,
    stopExecution,
    sendExecutionInput,
  } = useOpsApi();

  const selectedExecution = useMemo(
    () => executions.find((item) => item.executionId === selectedExecutionId) ?? null,
    [executions, selectedExecutionId]
  );

  const refreshSummary = useCallback(async () => {
    const payload = await fetchSummary();
    if (payload) {
      setSummary(payload);
    }
  }, [fetchSummary]);

  const refreshExecutions = useCallback(async () => {
    const payload = await listExecutions({
      limit: 60,
      status: statusFilter || undefined,
      hostId: hostFilter || undefined,
    });
    if (payload) {
      setExecutions(payload.items);
      if (!selectedExecutionId && payload.items.length > 0) {
        setSelectedExecutionId(payload.items[0].executionId);
      }
      if (selectedExecutionId && !payload.items.some((item) => item.executionId === selectedExecutionId)) {
        setSelectedExecutionId(payload.items[0]?.executionId ?? null);
      }
    }
  }, [hostFilter, listExecutions, selectedExecutionId, statusFilter]);

  const refreshAudit = useCallback(async () => {
    const payload = await listAuditEvents({ limit: 80 });
    if (payload) {
      setAuditItems(payload.items);
    }
  }, [listAuditEvents]);

  const refreshAll = useCallback(async () => {
    await Promise.all([refreshHosts(), refreshSummary(), refreshExecutions(), refreshAudit()]);
  }, [refreshAudit, refreshExecutions, refreshHosts, refreshSummary]);

  const refreshExecutionEvents = useCallback(
    async (executionId: string) => {
      const payload = await getExecutionEvents(executionId, 120);
      if (payload) {
        setExecutionEvents(payload.events);
      } else {
        setExecutionEvents([]);
      }
    },
    [getExecutionEvents]
  );

  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  useEffect(() => {
    void refreshExecutions();
  }, [refreshExecutions]);

  useEffect(() => {
    if (!selectedExecutionId) {
      setExecutionEvents([]);
      return;
    }
    void refreshExecutionEvents(selectedExecutionId);
  }, [refreshExecutionEvents, selectedExecutionId]);

  const handleStopExecution = useCallback(async () => {
    if (!selectedExecutionId) {
      return;
    }
    const ok = await stopExecution(selectedExecutionId);
    if (ok) {
      await refreshExecutions();
      await refreshAudit();
      await refreshSummary();
      await refreshExecutionEvents(selectedExecutionId);
    }
  }, [refreshAudit, refreshExecutionEvents, refreshExecutions, refreshSummary, selectedExecutionId, stopExecution]);

  const handleSendInput = useCallback(async () => {
    if (!selectedExecutionId || !inputDraft.trim()) {
      return;
    }
    const ok = await sendExecutionInput(selectedExecutionId, inputDraft.trim());
    if (ok) {
      setInputDraft('');
      await refreshExecutionEvents(selectedExecutionId);
      await refreshAudit();
    }
  }, [inputDraft, refreshAudit, refreshExecutionEvents, selectedExecutionId, sendExecutionInput]);

  if (activeTab === 'memory') {
    return (
      <div className="ops-memory-shell">
        <section className="tech-panel ops-panel reveal reveal-2">
          <div className="section-bar">
            <h2 className="section-title">Ops Console</h2>
            <div className="memory-actions">
              <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('hosts')}>
                <Server size={14} /> Hosts
              </button>
              <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('executions')}>
                <PlayCircle size={14} /> Executions
              </button>
              <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('audit')}>
                <TableProperties size={14} /> Audit
              </button>
              <button type="button" className="tech-btn tech-btn-primary">
                <Database size={14} /> Memory
              </button>
            </div>
          </div>
        </section>
        <MemoryPage language={language} />
      </div>
    );
  }

  return (
    <section className="tech-panel ops-panel reveal reveal-2">
      <div className="section-bar">
        <div className="flex items-center gap-2">
          <Activity size={16} className="text-cyan-300" />
          <h2 className="section-title">Ops Console</h2>
        </div>
        <div className="memory-actions">
          <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('hosts')}>
            <Server size={14} /> Hosts
          </button>
          <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('executions')}>
            <PlayCircle size={14} /> Executions
          </button>
          <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('audit')}>
            <TableProperties size={14} /> Audit
          </button>
          <button type="button" className="tech-btn tech-btn-secondary" onClick={() => setActiveTab('memory')}>
            <Database size={14} /> Memory
          </button>
          <button type="button" className="tech-btn tech-btn-primary" onClick={() => void refreshAll()}>
            <RefreshCw size={14} className={isLoading || hostsLoading ? 'animate-spin' : ''} /> Refresh
          </button>
        </div>
      </div>

      {summary && (
        <div className="ops-summary-grid">
          <article className="ops-summary-card">
            <h3 className="info-title">Hosts</h3>
            <p className="ops-summary-value">{summary.hosts.total}</p>
            <p className="section-note">
              online {summary.hosts.online} / busy {summary.hosts.busy} / offline {summary.hosts.offline}
            </p>
          </article>
          <article className="ops-summary-card">
            <h3 className="info-title">Executions</h3>
            <p className="ops-summary-value">{summary.executions.total}</p>
            <p className="section-note">
              running {summary.executions.running} / failed {summary.executions.failed}
            </p>
          </article>
          <article className="ops-summary-card">
            <h3 className="info-title">Memory</h3>
            <p className="ops-summary-value">{summary.memory.enabled ? 'enabled' : 'disabled'}</p>
            <p className="section-note">
              topK {summary.memory.retrievalTopK} / budget {summary.memory.tokenBudget}
            </p>
          </article>
          <article className="ops-summary-card">
            <h3 className="info-title">Updated</h3>
            <p className="ops-summary-value ops-summary-value--small">{formatDateTime(summary.updatedAt)}</p>
          </article>
        </div>
      )}

      {(error || hostsError) && (
        <div className="alert-error">
          <div>{error ?? hostsError}</div>
          <button type="button" className="tech-btn tech-btn-secondary" onClick={clearError}>
            Dismiss
          </button>
        </div>
      )}

      {activeTab === 'hosts' && (
        <div className="ops-list">
          {hosts.length === 0 ? (
            <div className="kanban-empty">No hosts connected.</div>
          ) : (
            hosts.map((host) => (
              <article key={host.hostId} className="ops-row-card">
                <div>
                  <h3 className="ops-row-title">{host.name}</h3>
                  <p className="section-note">{host.hostId}</p>
                </div>
                <div className="ops-row-meta">
                  <span className="command-chip">{host.status}</span>
                  <span className="section-note">agents: {host.capabilities.agents.join(', ') || '-'}</span>
                  <span className="section-note">active: {host.activeTasks.length}</span>
                  <span className="section-note">heartbeat: {host.lastHeartbeat}s</span>
                </div>
              </article>
            ))
          )}
        </div>
      )}

      {activeTab === 'executions' && (
        <div className="ops-execution-layout">
          <div className="info-block">
            <div className="run-filter">
              <label className="field">
                <span className="field-label">Status</span>
                <select
                  className="glass-select"
                  value={statusFilter}
                  onChange={(event) => setStatusFilter(event.target.value)}
                >
                  {STATUS_OPTIONS.map((status) => (
                    <option key={status || 'all'} value={status}>
                      {status || 'all'}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span className="field-label">Host</span>
                <input
                  className="glass-input"
                  value={hostFilter}
                  onChange={(event) => setHostFilter(event.target.value)}
                  placeholder="host-id"
                />
              </label>
            </div>

            <div className="ops-list">
              {executions.length === 0 ? (
                <div className="kanban-empty">No executions matched current filters.</div>
              ) : (
                executions.map((execution) => (
                  <button
                    type="button"
                    key={execution.executionId}
                    className={`ops-row-card ops-row-card--button ${
                      selectedExecutionId === execution.executionId ? 'ops-row-card--active' : ''
                    }`}
                    onClick={() => setSelectedExecutionId(execution.executionId)}
                  >
                    <div>
                      <h3 className="ops-row-title">{execution.status}</h3>
                      <p className="section-note">{execution.executionId}</p>
                    </div>
                    <div className="ops-row-meta">
                      <span className="section-note">task {execution.taskId}</span>
                      {execution.hostId && <span className="section-note">host {execution.hostId}</span>}
                      <span className="section-note">{formatDateTime(execution.createdAt)}</span>
                    </div>
                  </button>
                ))
              )}
            </div>
          </div>

          <div className="info-block">
            {selectedExecution ? (
              <div className="ops-execution-detail">
                <div className="section-bar">
                  <h3 className="section-title">Execution Detail</h3>
                  <div className="memory-actions">
                    <button type="button" className="tech-btn tech-btn-danger" onClick={() => void handleStopExecution()}>
                      <Square size={14} /> Stop
                    </button>
                    <button
                      type="button"
                      className="tech-btn tech-btn-secondary"
                      onClick={() => void refreshExecutionEvents(selectedExecution.executionId)}
                    >
                      <RefreshCw size={14} /> Events
                    </button>
                  </div>
                </div>

                <div className="memory-card__meta">
                  <span>execution={selectedExecution.executionId}</span>
                  <span>task={selectedExecution.taskId}</span>
                  <span>host={selectedExecution.hostId ?? '-'}</span>
                  <span>trace={selectedExecution.traceId ?? '-'}</span>
                </div>

                {selectedExecution.error && (
                  <div className="alert-error">
                    <AlertTriangle size={14} /> {selectedExecution.error}
                  </div>
                )}

                <div className="log-input-bar">
                  <input
                    className="log-input"
                    value={inputDraft}
                    onChange={(event) => setInputDraft(event.target.value)}
                    placeholder="Send runtime input..."
                  />
                  <button type="button" className="tech-btn tech-btn-secondary" onClick={() => void handleSendInput()}>
                    <Send size={14} /> Send
                  </button>
                </div>

                <div className="ops-events-list">
                  {executionEvents.length === 0 ? (
                    <div className="kanban-empty">No events loaded.</div>
                  ) : (
                    executionEvents.map((event) => (
                      <article key={event.payload.id} className="run-list-item">
                        <div className="memory-card__meta">
                          <span>seq {event.seq}</span>
                          <span>{new Date(event.ts).toLocaleString()}</span>
                          <span>{event.payload.event_type}</span>
                        </div>
                        {'content' in event.payload ? (
                          <p className="memory-card__content">{String(event.payload.content ?? '')}</p>
                        ) : null}
                      </article>
                    ))
                  )}
                </div>
              </div>
            ) : (
              <div className="kanban-empty">Choose one execution to inspect and control.</div>
            )}
          </div>
        </div>
      )}

      {activeTab === 'audit' && (
        <div className="ops-list">
          {auditItems.length === 0 ? (
            <div className="kanban-empty">No audit events available.</div>
          ) : (
            auditItems.map((item) => (
              <article key={item.id} className="ops-row-card">
                <div>
                  <h3 className="ops-row-title">{item.action}</h3>
                  <p className="section-note">
                    {formatDateTime(item.ts)} / actor {item.actor} / org {item.orgId}
                  </p>
                </div>
                <div className="ops-row-meta">
                  {item.status && <span className="command-chip">{item.status}</span>}
                  {item.executionId && <span className="section-note">execution {item.executionId}</span>}
                  {item.taskId && <span className="section-note">task {item.taskId}</span>}
                </div>
              </article>
            ))
          )}
        </div>
      )}
    </section>
  );
}
