import { useState } from 'react';
import { useOpencode } from './hooks/useOpencode';
import { useKanban } from './hooks/useKanban';
import { Terminal as TerminalIcon } from 'lucide-react';
import { Terminal } from './components/Terminal';
import { ChatView } from './components/ChatView';
import { InputBar } from './components/InputBar';
import { KanbanBoard } from './components/kanban/KanbanBoard';

function App() {
  const { isConnected } = useOpencode();
  const { board, moveTask, deleteTask } = useKanban();
  const [isTerminalOpen, setIsTerminalOpen] = useState(false);

  return (
    <div className="min-h-screen bg-slate-900 text-white flex flex-col items-center p-6 gap-8">
      <div className="w-full max-w-6xl bg-slate-800 p-8 rounded-xl shadow-2xl border border-slate-700 text-center space-y-6">
        <div className="flex justify-center">
          <div className="bg-indigo-600 p-4 rounded-full shadow-lg shadow-indigo-500/20">
            <TerminalIcon size={48} className="text-white" />
          </div>
        </div>

        <h1 className="text-4xl font-bold bg-gradient-to-r from-indigo-400 to-cyan-400 bg-clip-text text-transparent">
          Hello Opencode
        </h1>

        <p className="text-slate-400 text-lg">
          Welcome to Opencode Vibe Kanban client.
        </p>

        <div className={`inline-flex items-center px-4 py-2 rounded-full text-sm font-medium ${
          isConnected 
            ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' 
            : 'bg-rose-500/10 text-rose-400 border border-rose-500/20'
        }`}>
          <span className={`w-2 h-2 rounded-full mr-2 ${
            isConnected ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'
          }`}></span>
          {isConnected ? 'Connected to Server' : 'Disconnected'}
        </div>
      </div>

      <div className="w-full max-w-6xl grid grid-cols-1 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] gap-6">
        <div className="w-full bg-slate-900/60 border border-slate-700/50 rounded-xl overflow-hidden shadow-2xl">
          <div className="flex items-center justify-between px-4 py-3 border-b border-slate-700/60 bg-slate-800/80">
            <div className="text-sm font-semibold text-slate-200">Terminal</div>
            <button
              type="button"
              onClick={() => setIsTerminalOpen((prev) => !prev)}
              className="text-xs font-semibold px-3 py-1 rounded-full bg-slate-700/70 text-slate-200 hover:bg-slate-600"
            >
              {isTerminalOpen ? 'Collapse' : 'Expand'}
            </button>
          </div>
          {isTerminalOpen ? (
            <div className="p-4">
              <Terminal />
            </div>
          ) : (
            <div className="p-6 text-sm text-slate-400">Terminal is hidden. Expand to inspect raw output.</div>
          )}
        </div>
        <ChatView />
      </div>

      {/* Kanban Board */}
      <div className="w-full max-w-6xl">
        <KanbanBoard board={board} onMoveTask={moveTask} onDeleteTask={deleteTask} />
      </div>

      <InputBar />
    </div>
  )
}

export default App
