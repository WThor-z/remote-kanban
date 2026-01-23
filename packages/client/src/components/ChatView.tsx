import { useEffect, useMemo, useState } from 'react';
import { Parser, type LogLevel, type Message, type MessageType } from '@opencode-vibe/protocol';
import { useOpencode } from '../hooks/useOpencode';

type ChatTypeFilter = 'all' | MessageType;
type LogLevelFilter = 'all' | LogLevel;

export const ChatView = () => {
  const { onData } = useOpencode();
  const parser = useMemo(() => new Parser(), []);
  const [messages, setMessages] = useState<Message[]>([]);
  const [activeType, setActiveType] = useState<ChatTypeFilter>('all');
  const [activeLevel, setActiveLevel] = useState<LogLevelFilter>('all');

  const typeStyles: Record<MessageType, { label: string; className: string }> = {
    command: { label: 'COMMAND', className: 'bg-indigo-500/20 text-indigo-200' },
    log: { label: 'LOG', className: 'bg-amber-500/20 text-amber-200' },
    status: { label: 'STATUS', className: 'bg-emerald-500/20 text-emerald-200' },
    output: { label: 'OUTPUT', className: 'bg-slate-500/20 text-slate-200' },
  };

  const typeFilters: Array<{ value: ChatTypeFilter; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'command', label: 'Command' },
    { value: 'log', label: 'Log' },
    { value: 'status', label: 'Status' },
    { value: 'output', label: 'Output' },
  ];

  const levelFilters: Array<{ value: LogLevelFilter; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'debug', label: 'Debug' },
    { value: 'info', label: 'Info' },
    { value: 'warn', label: 'Warn' },
    { value: 'error', label: 'Error' },
  ];

  const groupLabels: Record<MessageType, string> = {
    command: 'Commands',
    log: 'Logs',
    status: 'Status Updates',
    output: 'Output',
  };

  useEffect(() => {
    const cleanup = onData((data) => {
      const parsed = parser.parse(data);
      setMessages((prev) => [...prev, parsed]);
    });

    return cleanup;
  }, [onData, parser]);

  useEffect(() => {
    if (activeType !== 'log' && activeLevel !== 'all') {
      setActiveLevel('all');
    }
  }, [activeType, activeLevel]);

  const visibleMessages = messages.filter((message) => {
    const resolvedType: MessageType = message.type ?? 'output';

    if (activeType !== 'all' && resolvedType !== activeType) {
      return false;
    }

    if (activeType === 'log' && activeLevel !== 'all') {
      return message.level === activeLevel;
    }

    return true;
  });

  const renderMessage = (message: Message, index: number) => {
    const resolvedType: MessageType = message.type ?? 'output';
    const resolvedStyle = typeStyles[resolvedType];

    return (
      <div
        key={`${index}-${message.raw}`}
        data-testid="chat-message"
        className="rounded-lg border border-slate-700/60 bg-slate-800/60 px-3 py-2 text-slate-100"
      >
        <div className="flex items-center gap-2 mb-2 text-[11px] uppercase tracking-[0.2em]">
          <span className={`rounded-full px-2 py-0.5 ${resolvedStyle.className}`}>
            {resolvedStyle.label}
          </span>
          {resolvedType === 'log' && message.level ? (
            <span className="rounded-full px-2 py-0.5 bg-rose-500/20 text-rose-200">
              {message.level}
            </span>
          ) : null}
        </div>
        <div className="whitespace-pre-wrap break-words text-sm">
          {message.content}
        </div>
      </div>
    );
  };

  const typeOrder: MessageType[] = ['command', 'status', 'log', 'output'];

  return (
    <div
      data-testid="chat-view"
      className="w-full h-[500px] bg-slate-900/80 backdrop-blur-xl rounded-xl border border-slate-700/50 shadow-2xl overflow-hidden flex flex-col"
    >
      <div className="bg-slate-800/80 px-4 py-3 border-b border-slate-700/50">
        <div className="flex flex-wrap items-center justify-between gap-3 text-sm font-semibold tracking-wide text-slate-200">
          <span>Opencode Feed</span>
          <div className="flex flex-wrap gap-2">
            {typeFilters.map((filter) => (
              <button
                key={filter.value}
                type="button"
                onClick={() => setActiveType(filter.value)}
                className={`rounded-full px-3 py-1 text-xs font-semibold transition ${
                  activeType === filter.value
                    ? 'bg-slate-200 text-slate-900'
                    : 'bg-slate-700/70 text-slate-200 hover:bg-slate-600'
                }`}
              >
                {filter.label}
              </button>
            ))}
          </div>
        </div>
        {activeType === 'log' ? (
          <div className="mt-3 flex flex-wrap gap-2">
            {levelFilters.map((filter) => (
              <button
                key={filter.value}
                type="button"
                onClick={() => setActiveLevel(filter.value)}
                className={`rounded-full px-3 py-1 text-xs font-semibold transition ${
                  activeLevel === filter.value
                    ? 'bg-amber-200 text-slate-900'
                    : 'bg-amber-500/10 text-amber-200 hover:bg-amber-500/20'
                }`}
              >
                {filter.label}
              </button>
            ))}
          </div>
        ) : null}
      </div>
      <div className="flex-1 overflow-auto p-4 space-y-3 text-sm">
        {visibleMessages.length === 0 ? (
          <div className="text-slate-400 italic">No messages yet.</div>
        ) : activeType === 'all' ? (
          typeOrder.map((type) => {
            const grouped = visibleMessages.filter((message) => (message.type ?? 'output') === type);
            if (grouped.length === 0) {
              return null;
            }

            return (
              <div key={type} className="space-y-2">
                <div
                  data-testid={`chat-group-${type}`}
                  className="text-xs font-semibold uppercase tracking-[0.25em] text-slate-400"
                >
                  {groupLabels[type]}
                </div>
                {grouped.map(renderMessage)}
              </div>
            );
          })
        ) : (
          visibleMessages.map(renderMessage)
        )}
      </div>
    </div>
  );
};
