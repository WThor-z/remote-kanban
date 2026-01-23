import { useOpencode } from './hooks/useOpencode';
import { Terminal } from 'lucide-react';

function App() {
  const { isConnected } = useOpencode();

  return (
    <div className="min-h-screen bg-slate-900 text-white flex flex-col items-center justify-center p-4">
      <div className="bg-slate-800 p-8 rounded-xl shadow-2xl border border-slate-700 max-w-md w-full text-center space-y-6 transform transition-all hover:scale-105">
        <div className="flex justify-center">
          <div className="bg-indigo-600 p-4 rounded-full shadow-lg shadow-indigo-500/20">
            <Terminal size={48} className="text-white" />
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
    </div>
  )
}

export default App
