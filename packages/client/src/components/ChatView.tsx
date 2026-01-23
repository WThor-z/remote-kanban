import { useEffect, useMemo, useState } from 'react';
import { Parser, type Message } from '@opencode-vibe/protocol';
import { useOpencode } from '../hooks/useOpencode';

export const ChatView = () => {
  const { onData } = useOpencode();
  const parser = useMemo(() => new Parser(), []);
  const [messages, setMessages] = useState<Message[]>([]);

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
          messages.map((message, index) => (
            <div
              key={`${index}-${message.raw}`}
              data-testid="chat-message"
              className="rounded-lg border border-slate-700/60 bg-slate-800/60 px-3 py-2 text-slate-100"
            >
              <div className="whitespace-pre-wrap break-words">
                {message.content}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
