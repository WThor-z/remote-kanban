import { Terminal, Cpu, FileCode, AlertCircle, CheckCircle, MessageSquare } from 'lucide-react';
import type { ExecutionEvent, AgentEvent } from '@opencode-vibe/protocol';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';

interface Props {
  events: ExecutionEvent[];
  className?: string;
  language?: ConsoleLanguage;
}

const copyByLanguage = {
  en: {
    statusPrefix: 'STATUS',
    sessionStarted: 'Execution session started on branch',
    sessionEnded: 'Session ended with status',
    thinking: 'Thinking',
    taskCompletedSuccess: 'Task Completed Successfully',
    taskCompletedFailed: 'Task Completed Failed',
  },
  zh: {
    statusPrefix: '状态',
    sessionStarted: '执行会话已在分支启动',
    sessionEnded: '会话结束状态',
    thinking: '思考',
    taskCompletedSuccess: '任务执行成功',
    taskCompletedFailed: '任务执行失败',
  },
} as const;

const runtimeStatusLabelByLanguage = {
  en: {
    idle: 'idle',
    starting: 'starting',
    running: 'running',
    paused: 'paused',
    completed: 'completed',
    failed: 'failed',
    aborted: 'aborted',
  },
  zh: {
    idle: '待执行',
    starting: '启动中',
    running: '执行中',
    paused: '已暂停',
    completed: '已完成',
    failed: '失败',
    aborted: '已中止',
  },
} as const;

export const ExecutionEventList: React.FC<Props> = ({ events, className, language = 'en' }) => (
  <div className={`space-y-3 ${className || ''}`}>
    {events.map((event) => (
      <EventItem key={event.id} event={event} language={language} />
    ))}
  </div>
);

const EventItem: React.FC<{ event: ExecutionEvent; language: ConsoleLanguage }> = ({ event, language }) => {
  const copy = copyByLanguage[language];
  if (event.event_type === 'agent_event') {
    if (!event.event) {
      console.warn('Malformed agent_event received:', event);
      return null;
    }
    return <AgentEventItem event={event.event} timestamp={event.timestamp} language={language} />;
  }
  if (event.event_type === 'status_changed') {
    const mappedStatus =
      runtimeStatusLabelByLanguage[language][event.new_status as keyof typeof runtimeStatusLabelByLanguage.en] ||
      event.new_status;
    return (
      <div className="flex items-center text-xs text-slate-500 py-1 border-t border-b border-slate-800/50 bg-slate-900/50">
        <span className="mr-3 font-mono opacity-50">{formatTime(event.timestamp)}</span>
        <span className="text-blue-400 font-semibold uppercase tracking-wider">{copy.statusPrefix}: {mappedStatus}</span>
      </div>
    );
  }
  if (event.event_type === 'session_started') {
    return (
      <div className="text-xs text-blue-400 py-2 flex items-center gap-2">
        <Terminal className="w-3 h-3" />
        {copy.sessionStarted} <span className="font-bold">{event.branch}</span>
      </div>
    );
  }
  if (event.event_type === 'session_ended') {
    return (
      <div className="text-xs text-slate-400 py-2">
        {copy.sessionEnded} <span className="font-semibold">{event.status}</span>
      </div>
    );
  }
  if (event.event_type === 'progress') {
    return (
      <div className="text-xs text-slate-500 py-1">
        {formatTime(event.timestamp)} · {event.message}
      </div>
    );
  }
  return null;
};

const AgentEventItem: React.FC<{ event: AgentEvent; timestamp: string; language: ConsoleLanguage }> = ({
  event,
  timestamp,
  language,
}) => {
  const copy = copyByLanguage[language];
  const time = formatTime(timestamp);

  switch (event.type) {
    case 'thinking':
      return (
        <div className="flex items-start text-yellow-500/80 bg-yellow-500/5 p-2 rounded border border-yellow-500/10">
          <span className="text-xs text-slate-600 mr-3 mt-0.5 select-none">{time}</span>
          <div className="flex-1 min-w-0">
            <div className="flex items-center mb-1 font-semibold text-xs uppercase tracking-wide opacity-70">
              <Cpu className="w-3 h-3 mr-1.5" /> {copy.thinking}
            </div>
            <div className="whitespace-pre-wrap break-words opacity-90">{event.content}</div>
          </div>
        </div>
      );
    case 'command':
      return (
        <div className="flex items-start group">
          <span className="text-xs text-slate-600 mr-3 mt-0.5 select-none">{time}</span>
          <div className="flex-1 min-w-0">
            <div className="flex items-center text-green-400 font-bold font-mono">
              <span className="mr-2 text-slate-500 select-none">$</span>
              {event.command}
            </div>
            {event.output && (
              <div className="mt-1 text-slate-400 pl-4 border-l-2 border-slate-700 font-mono text-xs whitespace-pre-wrap">
                {event.output}
              </div>
            )}
          </div>
        </div>
      );
    case 'error':
      return (
        <div className="flex items-start text-red-400 bg-red-500/10 p-2 rounded border border-red-500/20">
          <span className="text-xs text-slate-600 mr-3 mt-0.5 select-none">{time}</span>
          <AlertCircle className="w-4 h-4 mr-2 mt-0.5 flex-shrink-0" />
          <span className="break-words font-medium">{event.message}</span>
        </div>
      );
    case 'file_change':
      return (
        <div className="flex items-center text-blue-400 pl-16">
          <FileCode className="w-4 h-4 mr-2" />
          <span className="uppercase text-xs font-bold mr-2">{event.action}:</span>
          <span className="font-mono">{event.path}</span>
        </div>
      );
    case 'message':
      return (
        <div className="flex items-start text-slate-300">
          <span className="text-xs text-slate-600 mr-3 mt-0.5 select-none">{time}</span>
          <MessageSquare className="w-4 h-4 mr-2 mt-0.5 text-slate-500" />
          <div className="flex-1 whitespace-pre-wrap">{event.content}</div>
        </div>
      );
    case 'completed':
      return (
        <div className={`flex items-center py-3 border-t border-slate-700 mt-4 font-bold ${event.success ? 'text-green-500' : 'text-red-500'}`}>
          <span className="text-xs text-slate-600 mr-3 font-normal">{time}</span>
          <CheckCircle className="w-5 h-5 mr-2" />
          <span>{event.success ? copy.taskCompletedSuccess : copy.taskCompletedFailed}</span>
        </div>
      );
    case 'raw_output':
      return (
        <div className="flex items-start text-slate-400/80 font-mono text-xs hover:text-slate-300">
          <span className="text-xs text-slate-700 mr-3 select-none w-[60px] text-right">{time}</span>
          <div className="whitespace-pre-wrap break-all">{event.content}</div>
        </div>
      );
    default:
      return null;
  }
};

function formatTime(timestamp: string): string {
  try {
    return new Date(timestamp).toLocaleTimeString([], {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return '';
  }
}
