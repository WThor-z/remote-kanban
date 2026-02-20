import { useCallback, useEffect, useMemo, useState } from 'react';
import { RefreshCw, Server, Activity, ScrollText } from 'lucide-react';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';
import {
  useOpsApi,
  type OpsAuditEvent,
  type OpsExecutionItem,
  type OpsSummaryResponse,
} from '../../hooks/useOpsApi';

interface OpsConsolePageProps {
  workspaceId: string;
  language: ConsoleLanguage;
}

const COPY = {
  en: {
    title: 'Operations Console',
    subtitle: 'Unified control plane for hosts, executions, and audit telemetry',
    refresh: 'Refresh',
    loading: 'Loading...',
    updatedAt: 'Updated',
    hosts: 'Hosts',
    executions: 'Executions',
    memory: 'Memory',
    audit: 'Audit',
    hostTotal: 'Total',
    hostOnline: 'Online',
    hostBusy: 'Busy',
    hostOffline: 'Offline',
    executionTotal: 'Total',
    executionRunning: 'Running',
    executionCompleted: 'Completed',
    executionFailed: 'Failed',
    executionCancelled: 'Cancelled',
    statusFilter: 'Status',
    statusAll: 'All',
    tableExecution: 'Execution',
    tableTask: 'Task',
    tableHost: 'Host',
    tableStatus: 'Status',
    tableStarted: 'Started',
    tableDuration: 'Duration',
    tableEvents: 'Events',
    noExecutions: 'No executions matched the current filter.',
    auditAction: 'Action',
    auditActor: 'Actor',
    auditWhen: 'When',
    auditStatus: 'Result',
    noAudit: 'No audit records available.',
    dataError: 'Failed to load operations data',
    notAvailable: 'n/a',
  },
  zh: {
    title: '运维控制台',
    subtitle: '统一查看主机、执行与审计遥测',
    refresh: '刷新',
    loading: '加载中...',
    updatedAt: '更新时间',
    hosts: '主机',
    executions: '执行',
    memory: '记忆',
    audit: '审计',
    hostTotal: '总数',
    hostOnline: '在线',
    hostBusy: '忙碌',
    hostOffline: '离线',
    executionTotal: '总数',
    executionRunning: '运行中',
    executionCompleted: '完成',
    executionFailed: '失败',
    executionCancelled: '已取消',
    statusFilter: '状态',
    statusAll: '全部',
    tableExecution: '执行 ID',
    tableTask: '任务 ID',
    tableHost: '主机',
    tableStatus: '状态',
    tableStarted: '开始时间',
    tableDuration: '耗时',
    tableEvents: '事件数',
    noExecutions: '当前筛选下没有执行记录。',
    auditAction: '动作',
    auditActor: '操作者',
    auditWhen: '时间',
    auditStatus: '结果',
    noAudit: '暂无审计记录。',
    dataError: '运维数据加载失败',
    notAvailable: '无',
  },
} as const;

const statusOptions = ['all', 'running', 'completed', 'failed', 'cancelled'] as const;

const shorten = (value: string, size = 8): string => {
  if (!value) {
    return value;
  }
  if (value.length <= size * 2 + 1) {
    return value;
  }
  return `${value.slice(0, size)}...${value.slice(-size)}`;
};

const formatDateTime = (iso: string | null | undefined, fallback: string): string => {
  if (!iso) {
    return fallback;
  }
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleString();
};

const formatDuration = (durationMs: number | null, fallback: string): string => {
  if (durationMs == null) {
    return fallback;
  }
  if (durationMs < 1000) {
    return `${durationMs}ms`;
  }
  return `${(durationMs / 1000).toFixed(1)}s`;
};

export function OpsConsolePage({ workspaceId, language }: OpsConsolePageProps) {
  const copy = COPY[language];
  const { isLoading, error, clearError, getSummary, listExecutions, listAudit } = useOpsApi();
  const [summary, setSummary] = useState<OpsSummaryResponse | null>(null);
  const [executions, setExecutions] = useState<OpsExecutionItem[]>([]);
  const [auditEvents, setAuditEvents] = useState<OpsAuditEvent[]>([]);
  const [statusFilter, setStatusFilter] = useState<(typeof statusOptions)[number]>('all');
  const [isRefreshing, setIsRefreshing] = useState(false);

  const effectiveStatusFilter = useMemo(
    () => (statusFilter === 'all' ? undefined : statusFilter),
    [statusFilter]
  );

  const loadData = useCallback(async () => {
    setIsRefreshing(true);
    clearError();

    const summaryResponse = await getSummary();
    const executionsResponse = await listExecutions({
      limit: 30,
      status: effectiveStatusFilter,
      workspaceId: workspaceId || undefined,
    });
    const auditResponse = await listAudit({ limit: 20 });

    if (summaryResponse) {
      setSummary(summaryResponse);
    }
    if (executionsResponse) {
      setExecutions(executionsResponse.items);
    }
    if (auditResponse) {
      setAuditEvents(auditResponse.items);
    }
    setIsRefreshing(false);
  }, [
    clearError,
    effectiveStatusFilter,
    getSummary,
    listAudit,
    listExecutions,
    workspaceId,
  ]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  return (
    <section className="tech-panel board-panel reveal reveal-2">
      <div className="section-bar">
        <div className="flex items-center gap-2">
          <Activity size={16} className="text-cyan-300" />
          <h2 className="section-title">{copy.title}</h2>
        </div>
        <div className="flex items-center gap-3">
          <span className="section-note">
            {copy.updatedAt}: {formatDateTime(summary?.updatedAt, copy.notAvailable)}
          </span>
          <button
            type="button"
            className="tech-btn tech-btn-secondary"
            onClick={() => void loadData()}
            disabled={isRefreshing}
          >
            <RefreshCw size={14} className={isRefreshing ? 'animate-spin' : ''} /> {copy.refresh}
          </button>
        </div>
      </div>
      <p className="section-note" style={{ marginBottom: '1rem' }}>
        {copy.subtitle}
      </p>

      <div
        style={{
          display: 'grid',
          gap: '0.75rem',
          gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
          marginBottom: '1rem',
        }}
      >
        <div className="gateway-card">
          <div className="gateway-label">
            <Server size={12} /> {copy.hosts}
          </div>
          <div className="gateway-value">
            {copy.hostTotal}: {summary?.hosts.total ?? 0}
          </div>
          <div className="section-note">
            {copy.hostOnline} {summary?.hosts.online ?? 0} / {copy.hostBusy}{' '}
            {summary?.hosts.busy ?? 0} / {copy.hostOffline} {summary?.hosts.offline ?? 0}
          </div>
        </div>

        <div className="gateway-card">
          <div className="gateway-label">
            <Activity size={12} /> {copy.executions}
          </div>
          <div className="gateway-value">
            {copy.executionTotal}: {summary?.executions.total ?? 0}
          </div>
          <div className="section-note">
            {copy.executionRunning} {summary?.executions.running ?? 0} / {copy.executionCompleted}{' '}
            {summary?.executions.completed ?? 0}
          </div>
        </div>

        <div className="gateway-card">
          <div className="gateway-label">{copy.memory}</div>
          <div className="gateway-value">
            {summary?.memory.enabled ? 'Enabled' : 'Disabled'}
          </div>
          <div className="section-note">
            token={summary?.memory.tokenBudget ?? 0}, topK={summary?.memory.retrievalTopK ?? 0}
          </div>
        </div>

        <div className="gateway-card">
          <div className="gateway-label">
            <ScrollText size={12} /> {copy.audit}
          </div>
          <div className="gateway-value">{summary?.audit.total ?? 0}</div>
          <div className="section-note">{copy.audit}</div>
        </div>
      </div>

      <div className="section-bar" style={{ marginBottom: '0.5rem' }}>
        <h3 className="section-title">{copy.executions}</h3>
        <label className="section-note">
          {copy.statusFilter}:{' '}
          <select
            value={statusFilter}
            onChange={(event) =>
              setStatusFilter(event.target.value as (typeof statusOptions)[number])
            }
            style={{
              marginLeft: '0.35rem',
              background: 'rgba(15, 23, 42, 0.55)',
              border: '1px solid rgba(148, 163, 184, 0.35)',
              borderRadius: '8px',
              padding: '0.2rem 0.45rem',
              color: '#e2e8f0',
            }}
          >
            {statusOptions.map((status) => (
              <option key={status} value={status}>
                {status === 'all' ? copy.statusAll : status}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div style={{ overflowX: 'auto', marginBottom: '1rem' }}>
        <table style={{ width: '100%', fontSize: '0.85rem' }}>
          <thead>
            <tr style={{ textAlign: 'left', color: '#94a3b8' }}>
              <th>{copy.tableExecution}</th>
              <th>{copy.tableTask}</th>
              <th>{copy.tableHost}</th>
              <th>{copy.tableStatus}</th>
              <th>{copy.tableStarted}</th>
              <th>{copy.tableDuration}</th>
              <th>{copy.tableEvents}</th>
            </tr>
          </thead>
          <tbody>
            {executions.length === 0 ? (
              <tr>
                <td colSpan={7} style={{ color: '#94a3b8', padding: '0.8rem 0' }}>
                  {copy.noExecutions}
                </td>
              </tr>
            ) : (
              executions.map((execution) => (
                <tr key={execution.executionId} style={{ borderTop: '1px solid rgba(51, 65, 85, 0.6)' }}>
                  <td className="gateway-value gateway-value--mono">{shorten(execution.executionId)}</td>
                  <td className="gateway-value gateway-value--mono">{shorten(execution.taskId)}</td>
                  <td className="gateway-value gateway-value--mono">
                    {execution.hostId ?? copy.notAvailable}
                  </td>
                  <td>{execution.status}</td>
                  <td>{formatDateTime(execution.startedAt, copy.notAvailable)}</td>
                  <td>{formatDuration(execution.durationMs, copy.notAvailable)}</td>
                  <td>{execution.eventCount}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <div className="section-bar" style={{ marginBottom: '0.5rem' }}>
        <h3 className="section-title">{copy.audit}</h3>
      </div>
      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', fontSize: '0.85rem' }}>
          <thead>
            <tr style={{ textAlign: 'left', color: '#94a3b8' }}>
              <th>{copy.auditAction}</th>
              <th>{copy.auditActor}</th>
              <th>{copy.tableHost}</th>
              <th>{copy.auditStatus}</th>
              <th>{copy.auditWhen}</th>
            </tr>
          </thead>
          <tbody>
            {auditEvents.length === 0 ? (
              <tr>
                <td colSpan={5} style={{ color: '#94a3b8', padding: '0.8rem 0' }}>
                  {copy.noAudit}
                </td>
              </tr>
            ) : (
              auditEvents.map((event) => (
                <tr key={event.id} style={{ borderTop: '1px solid rgba(51, 65, 85, 0.6)' }}>
                  <td>{event.action}</td>
                  <td>{event.actor}</td>
                  <td className="gateway-value gateway-value--mono">{event.hostId ?? copy.notAvailable}</td>
                  <td>{event.status ?? copy.notAvailable}</td>
                  <td>{formatDateTime(event.timestamp, copy.notAvailable)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {(isLoading || isRefreshing) && (
        <div className="section-note" style={{ marginTop: '0.75rem' }}>
          {copy.loading}
        </div>
      )}
      {error && (
        <div className="gateway-error" style={{ marginTop: '0.75rem' }}>
          {copy.dataError}: {error}
        </div>
      )}
    </section>
  );
}
