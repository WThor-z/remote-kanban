import { useEffect, useMemo, useState } from 'react';
import { Parser, type Message, type MessageType } from '@opencode-vibe/protocol';
import { useOpencode } from '../hooks/useOpencode';

export const ChatView = () => {
  const { onData } = useOpencode();
  const parser = useMemo(() => new Parser(), []);
  const [messages, setMessages] = useState<Message[]>([]);

  const typeStyles: Record<MessageType, { label: string; className: string }> = {
    command: { label: 'COMMAND', className: 'bg-indigo-500/20 text-indigo-200' },
    log: { label: 'LOG', className: 'bg-amber-500/20 text-amber-200' },
    status: { label: 'STATUS', className: 'bg-emerald-500/20 text-emerald-200' },
    output: { label: 'OUTPUT', className: 'bg-slate-500/20 text-slate-200' },
  };

  useEffect(() => {
    const cleanup = onData((data) => {
      const parsed = parser.parse(data);
      setMessages((prev) => [...prev, parsed]);
    });

    return cleanup;
  }, [onData, parser]);

  return (
    <div
      data-testid="chat-view"
      className="w-full h-[500px] bg-slate-900/80 backdrop-blur-xl rounded-xl border border-slate-700/50 shadow-2xl overflow-hidden flex flex-col"
    >
      <div className="bg-slate-800/80 px-4 py-3 border-b border-slate-700/50 text-sm font-semibold tracking-wide text-slate-200">
        Opencode Feed
      </div>
      <div className="flex-1 overflow-auto p-4 space-y-3 text-sm">
        {messages.length === 0 ? (
          <div className="text-slate-400 italic">No messages yet.</div>
        ) : (
          messages.map((message, index) => {
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
          })
        )}
      </div>
    </div>
  );
};
