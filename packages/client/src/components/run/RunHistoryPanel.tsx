import { ArrowLeft, History, RefreshCcw, Clock } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTaskRuns, type RunStatus } from '../../hooks/useTaskRuns';
import { useRunEvents } from '../../hooks/useRunEvents';
import { ExecutionEventList } from '../execution/ExecutionEventList';
import { getConsoleLexiconSection } from '../../lexicon/consoleLexicon';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';

interface Props {
  taskId: string;
  language?: ConsoleLanguage;
}

export const RunHistoryPanel: React.FC<Props> = ({ taskId, language = 'en' }) => {
  const copy = getConsoleLexiconSection('runHistoryPanel', language);
  const statusConfig: Record<RunStatus, { label: string; color: string; dot: string }> = {
    initializing: { label: copy.statuses.initializing, color: 'text-slate-400', dot: 'bg-slate-500' },
    creating_worktree: {
      label: copy.statuses.creating_worktree,
      color: 'text-amber-400',
      dot: 'bg-amber-400',
    },
    starting: { label: copy.statuses.starting, color: 'text-amber-400', dot: 'bg-amber-400' },
    running: { label: copy.statuses.running, color: 'text-indigo-400', dot: 'bg-indigo-400' },
    paused: { label: copy.statuses.paused, color: 'text-amber-400', dot: 'bg-amber-400' },
    completed: { label: copy.statuses.completed, color: 'text-emerald-400', dot: 'bg-emerald-400' },
    failed: { label: copy.statuses.failed, color: 'text-rose-400', dot: 'bg-rose-400' },
    cancelled: { label: copy.statuses.cancelled, color: 'text-slate-400', dot: 'bg-slate-500' },
    cleaning_up: { label: copy.statuses.cleaning_up, color: 'text-slate-400', dot: 'bg-slate-500' },
  };
  const { runs, isLoading, error, refresh } = useTaskRuns(taskId);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [eventType, setEventType] = useState<string>('');
  const [agentEventType, setAgentEventType] = useState<string>('');

  const selectedRun = useMemo(
    () => runs.find((run) => run.id === selectedRunId) || null,
    [runs, selectedRunId],
  );

  useEffect(() => {
    if (eventType !== 'agent_event') {
      setAgentEventType('');
    }
  }, [eventType]);

  const {
    events,
    isLoading: isLoadingEvents,
    error: eventsError,
    hasMore,
    refresh: refreshEvents,
    loadMore,
  } = useRunEvents(
    taskId,
    selectedRunId,
    {
      eventType: eventType || undefined,
      agentEventType: agentEventType || undefined,
    },
  );

  return (
    <div className="log-shell">
      <div className="log-shell__head">
        {selectedRunId ? (
          <button
            type="button"
            onClick={() => setSelectedRunId(null)}
            className="mr-2 text-slate-500 hover:text-slate-300 transition-colors"
            aria-label={copy.meta.backToRunList}
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
        ) : (
          <History className="w-4 h-4 mr-2" />
        )}
        <span className="font-semibold">{selectedRunId ? copy.titleTimeline : copy.titleHistory}</span>
        <div className="ml-auto flex items-center gap-2 text-xs text-slate-500">
          {!selectedRunId && <span>{runs.length} {copy.meta.runsCountSuffix}</span>}
          <button
            type="button"
            onClick={() => (selectedRunId ? void refreshEvents() : void refresh())}
            className="p-1 text-slate-500 hover:text-slate-300 transition-colors"
            aria-label={selectedRunId ? copy.meta.refreshEvents : copy.meta.refreshRuns}
          >
            <RefreshCcw className={`w-3.5 h-3.5 ${(selectedRunId ? isLoadingEvents : isLoading) ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      <div className="log-shell__body space-y-3">
        {!selectedRunId && (
          <>
            {isLoading && runs.length === 0 && (
              <div className="flex items-center justify-center py-8 text-slate-500">
                <RefreshCcw className="w-4 h-4 mr-2 animate-spin" />
                {copy.labels.loadingRuns}
              </div>
            )}

            {!isLoading && runs.length === 0 && !error && (
              <div className="text-center py-8 text-slate-500">
                {copy.labels.noRuns}
              </div>
            )}

            {error && (
              <div className="bg-rose-500/10 border border-rose-500/20 rounded-lg p-3 text-sm text-rose-400">
                {error}
              </div>
            )}

            {runs.map((run) => {
              const statusInfo = statusConfig[run.status];
              return (
                <button
                  key={run.id}
                  type="button"
                  onClick={() => setSelectedRunId(run.id)}
                  className="run-list-item w-full text-left hover:border-slate-600 transition-colors"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2">
                      <span className={`w-2 h-2 rounded-full ${statusInfo.dot}`} />
                      <span className={`text-xs font-semibold ${statusInfo.color}`}>{statusInfo.label}</span>
                      <span className="text-xs text-slate-500">{formatDate(run.createdAt)}</span>
                    </div>
                    <div className="text-xs text-slate-500">
                      {run.durationMs ? formatDuration(run.durationMs) : copy.meta.durationUnknown}
                    </div>
                  </div>

                  <div className="mt-2 text-sm text-slate-200">
                    {run.promptPreview || copy.labels.noPrompt}
                  </div>

                  <div className="mt-2 flex flex-wrap items-center gap-3 text-xs text-slate-500">
                    <span>{copy.meta.agent}: {run.agentType}</span>
                    <span>{copy.meta.events}: {run.eventCount}</span>
                    <span className="flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      {run.startedAt ? formatDate(run.startedAt) : copy.labels.notStarted}
                    </span>
                  </div>
                </button>
              );
            })}
          </>
        )}

        {selectedRunId && (
          <>
            {selectedRun && (
              <div className="run-list-item">
                <div className="flex items-center justify-between text-xs text-slate-500">
                  <span>{copy.meta.run} {selectedRun.id.slice(0, 8)}</span>
                  <span>{formatDate(selectedRun.createdAt)}</span>
                </div>
                <div className="mt-2 text-sm text-slate-200">
                  {selectedRun.promptPreview || copy.labels.noPrompt}
                </div>
              </div>
            )}

            <div className="run-filter text-xs text-slate-400">
              <label className="flex items-center gap-2">
                <span>{copy.labels.eventFilterLabel}</span>
                <select
                  value={eventType}
                  onChange={(e) => setEventType(e.target.value)}
                  className="inline-select"
                >
                  <option value="">{copy.filters.all}</option>
                  <option value="agent_event">{copy.filters.agent}</option>
                  <option value="status_changed">{copy.filters.status}</option>
                  <option value="session_started">{copy.filters.sessionStarted}</option>
                  <option value="session_ended">{copy.filters.sessionEnded}</option>
                  <option value="progress">{copy.filters.progress}</option>
                </select>
              </label>

              <label className="flex items-center gap-2">
                <span>{copy.labels.agentEventFilterLabel}</span>
                <select
                  value={agentEventType}
                  onChange={(e) => setAgentEventType(e.target.value)}
                  disabled={eventType !== 'agent_event'}
                  className="inline-select disabled:opacity-50"
                >
                  <option value="">{copy.filters.all}</option>
                  <option value="thinking">{copy.filters.thinking}</option>
                  <option value="command">{copy.filters.command}</option>
                  <option value="file_change">{copy.filters.fileChange}</option>
                  <option value="tool_call">{copy.filters.toolCall}</option>
                  <option value="message">{copy.filters.message}</option>
                  <option value="error">{copy.filters.error}</option>
                  <option value="completed">{copy.filters.completed}</option>
                  <option value="raw_output">{copy.filters.rawOutput}</option>
                </select>
              </label>
            </div>

            {isLoadingEvents && events.length === 0 && (
              <div className="flex items-center justify-center py-8 text-slate-500">
                <RefreshCcw className="w-4 h-4 mr-2 animate-spin" />
                {copy.labels.loadingEvents}
              </div>
            )}

            {eventsError && (
              <div className="bg-rose-500/10 border border-rose-500/20 rounded-lg p-3 text-sm text-rose-400">
                {eventsError}
              </div>
            )}

            {!isLoadingEvents && events.length === 0 && !eventsError && (
              <div className="text-center py-6 text-slate-500">
                {copy.labels.noEvents}
              </div>
            )}

            {events.length > 0 && (
              <div className="run-list-item bg-slate-950/60">
                <ExecutionEventList events={events} language={language} />
              </div>
            )}

            {hasMore && (
              <button
                type="button"
                onClick={() => void loadMore()}
                className="w-full text-center text-xs text-indigo-400 hover:text-indigo-300 transition-colors py-2"
              >
                {copy.labels.loadMore}
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
};

function formatDate(value: string | null): string {
  if (!value) return '—';
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const totalSeconds = Math.floor(ms / 1000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);

  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}
