import React, { useEffect, useRef } from 'react';
import { useExecutionEvents } from '../../hooks/useExecutionEvents';
import { Terminal } from 'lucide-react';
import { ExecutionEventList } from './ExecutionEventList';

interface Props {
  taskId: string;
  onSendInput?: (taskId: string, content: string) => Promise<boolean>;
  isRunning?: boolean;
}

export const ExecutionLogPanel: React.FC<Props> = ({ taskId, onSendInput, isRunning }) => {
  const { events } = useExecutionEvents(taskId);
  const bottomRef = useRef<HTMLDivElement>(null);
  const [input, setInput] = React.useState('');

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [events]);

  const handleInputSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input || !onSendInput) return;
    
    await onSendInput(taskId, input);
    setInput('');
  };

  return (
    <div className="flex flex-col h-full bg-slate-900 text-slate-300 font-mono text-sm overflow-hidden rounded-lg border border-slate-800">
      <div className="flex items-center px-4 py-2 bg-slate-950 border-b border-slate-800">
        <Terminal className="w-4 h-4 mr-2" />
        <span className="font-semibold">Execution Logs</span>
        <div className="ml-auto flex items-center space-x-2 text-xs text-slate-500">
          <div className={`w-2 h-2 rounded-full ${isRunning ? 'bg-green-500 animate-pulse' : 'bg-slate-600'}`}></div>
          <span>{isRunning ? 'Live' : 'Offline'}</span>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-4">
        {events.length === 0 && (
          <div className="text-center text-slate-600 italic py-8">
            Waiting for execution logs...
          </div>
        )}
        <ExecutionEventList events={events} />
        <div ref={bottomRef} />
      </div>

      {/* Input Area */}
      {isRunning && onSendInput && (
        <div className="p-2 bg-slate-950 border-t border-slate-800">
          <form onSubmit={handleInputSubmit} className="flex gap-2 items-center">
            <span className="text-green-500 font-mono pl-2">❯</span>
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              className="flex-1 bg-transparent border-none text-slate-200 font-mono text-sm focus:ring-0 focus:outline-none placeholder-slate-600 px-2 py-1"
              placeholder="Type input for agent..."
              autoComplete="off"
            />
          </form>
        </div>
      )}
    </div>
  );
};

