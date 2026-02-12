import React, { useEffect, useRef } from 'react';
import { useExecutionEvents } from '../../hooks/useExecutionEvents';
import { Terminal } from 'lucide-react';
import { ExecutionEventList } from './ExecutionEventList';
import { getConsoleLexiconSection } from '../../lexicon/consoleLexicon';
import type { ConsoleLanguage } from '../../i18n/consoleLanguage';

interface Props {
  taskId: string;
  onSendInput?: (taskId: string, content: string) => Promise<boolean>;
  isRunning?: boolean;
  language?: ConsoleLanguage;
}

export const ExecutionLogPanel: React.FC<Props> = ({ taskId, onSendInput, isRunning, language = 'en' }) => {
  const copy = getConsoleLexiconSection('executionLogPanel', language);
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
    <div className="log-shell">
      <div className="log-shell__head">
        <Terminal className="w-4 h-4 mr-2" />
        <span className="font-semibold">{copy.header}</span>
        <div className="ml-auto flex items-center space-x-2 text-xs text-slate-500">
          <div className={`w-2 h-2 rounded-full ${isRunning ? 'bg-green-500 animate-pulse' : 'bg-slate-600'}`}></div>
          <span>{isRunning ? copy.live : copy.offline}</span>
        </div>
      </div>
      <div className="log-shell__body">
        {events.length === 0 && (
          <div className="text-center text-slate-600 italic py-8">
            {copy.empty}
          </div>
        )}
        <ExecutionEventList events={events} language={language} />
        <div ref={bottomRef} />
      </div>

      {/* Input Area */}
      {isRunning && onSendInput && (
        <div className="log-input-bar">
          <form onSubmit={handleInputSubmit} className="flex gap-2 items-center w-full">
            <span className="text-green-500 font-mono pl-1">$</span>
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              className="log-input"
              placeholder={copy.inputPlaceholder}
              autoComplete="off"
            />
          </form>
        </div>
      )}
    </div>
  );
};

