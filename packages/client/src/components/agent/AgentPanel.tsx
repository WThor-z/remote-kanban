import { useState, useRef, useEffect } from 'react';
import { useAgent } from '../../hooks/useAgent';
import { Bot, Play, Square, Trash2, ChevronDown, ChevronUp } from 'lucide-react';
import type { AgentType } from '@opencode-vibe/protocol';

const AGENT_OPTIONS: { value: AgentType; label: string }[] = [
  { value: 'opencode', label: 'OpenCode' },
  { value: 'claude-code', label: 'Claude Code' },
  { value: 'codex', label: 'Codex' },
  { value: 'gemini-cli', label: 'Gemini CLI' },
];

export const AgentPanel: React.FC = () => {
  const {
    sessions,
    activeSession,
    outputLog,
    startAgent,
    stopAgent,
    clearOutput,
    isRunning,
  } = useAgent();

  const [selectedAgent, setSelectedAgent] = useState<AgentType>('opencode');
  const [prompt, setPrompt] = useState('');
  const [isExpanded, setIsExpanded] = useState(true);
  const outputEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new output arrives
  useEffect(() => {
    outputEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [outputLog]);

  const handleStart = () => {
    if (!prompt.trim()) return;
    startAgent(selectedAgent, prompt.trim());
    setPrompt('');
  };

  const handleStop = () => {
    if (activeSession) {
      stopAgent(activeSession.id);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleStart();
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running':
      case 'starting':
        return 'text-emerald-400';
      case 'completed':
        return 'text-blue-400';
      case 'failed':
        return 'text-rose-400';
      case 'aborted':
        return 'text-amber-400';
      default:
        return 'text-slate-400';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'running':
      case 'starting':
        return <span className="animate-pulse">●</span>;
      case 'completed':
        return '✓';
      case 'failed':
        return '✕';
      case 'aborted':
        return '○';
      default:
        return '○';
    }
  };

  return (
    <div className="bg-slate-800/50 border border-slate-700/50 rounded-xl overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-slate-700/60 bg-slate-800/80">
        <div className="flex items-center gap-2">
          <Bot size={18} className="text-indigo-400" />
          <span className="text-sm font-semibold text-slate-200">AI Agent</span>
          {isRunning && (
            <span className="px-2 py-0.5 text-xs bg-emerald-500/20 text-emerald-400 rounded-full">
              Running
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="text-xs font-semibold px-3 py-1 rounded-full bg-slate-700/70 text-slate-200 hover:bg-slate-600 flex items-center gap-1"
        >
          {isExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          {isExpanded ? 'Collapse' : 'Expand'}
        </button>
      </div>

      {isExpanded && (
        <div className="p-4 space-y-4">
          {/* Controls */}
          <div className="flex gap-3">
            <select
              value={selectedAgent}
              onChange={(e) => setSelectedAgent(e.target.value as AgentType)}
              disabled={isRunning}
              className="px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:opacity-50"
            >
              {AGENT_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>

            <input
              type="text"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter your prompt..."
              disabled={isRunning}
              className="flex-1 px-3 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-sm text-slate-200 placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:opacity-50"
            />

            {isRunning ? (
              <button
                type="button"
                onClick={handleStop}
                className="px-4 py-2 bg-rose-600 hover:bg-rose-700 text-white rounded-lg text-sm font-medium flex items-center gap-2 transition-colors"
              >
                <Square size={14} />
                Stop
              </button>
            ) : (
              <button
                type="button"
                onClick={handleStart}
                disabled={!prompt.trim()}
                className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:bg-slate-600 disabled:cursor-not-allowed text-white rounded-lg text-sm font-medium flex items-center gap-2 transition-colors"
              >
                <Play size={14} />
                Start
              </button>
            )}
          </div>

          {/* Output Log */}
          <div className="bg-slate-900/50 rounded-lg border border-slate-700/50 h-48 overflow-y-auto font-mono text-xs">
            {outputLog.length === 0 ? (
              <div className="p-4 text-slate-500 text-center">
                No output yet. Start an agent to see output here.
              </div>
            ) : (
              <div className="p-3 space-y-1">
                {outputLog.map((entry, idx) => (
                  <div
                    key={idx}
                    className={`whitespace-pre-wrap break-all ${
                      entry.type === 'stderr'
                        ? 'text-rose-400'
                        : entry.type === 'system'
                        ? 'text-amber-400'
                        : 'text-slate-300'
                    }`}
                  >
                    {entry.data}
                  </div>
                ))}
                <div ref={outputEndRef} />
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex justify-between items-center">
            <button
              type="button"
              onClick={clearOutput}
              className="text-xs text-slate-400 hover:text-slate-200 flex items-center gap-1"
            >
              <Trash2 size={12} />
              Clear Output
            </button>

            {/* Session History */}
            {sessions.length > 0 && (
              <div className="text-xs text-slate-400">
                {sessions.length} session{sessions.length !== 1 ? 's' : ''}
                {activeSession && (
                  <span className={`ml-2 ${getStatusColor(activeSession.status)}`}>
                    {getStatusIcon(activeSession.status)} {activeSession.status}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default AgentPanel;
