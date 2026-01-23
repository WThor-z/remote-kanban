import { useEffect, useRef } from 'react';
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { useOpencode } from '../hooks/useOpencode';
import 'xterm/css/xterm.css';

export const Terminal = () => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const { write: sendInput, onData } = useOpencode();

  useEffect(() => {
    if (!terminalRef.current) return;

    // Initialize xterm
    const term = new XTerm({
      cursorBlink: true,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      fontSize: 14,
      theme: {
        background: '#00000000',
        foreground: '#f8fafc',
        cursor: '#f8fafc',
        selectionBackground: 'rgba(255, 255, 255, 0.3)',
      },
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    term.open(terminalRef.current);
    
    // Slight delay to ensure DOM is ready for fit
    setTimeout(() => {
      fitAddon.fit();
    }, 0);

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    // Handle resizing
    const handleResize = () => {
      fitAddon.fit();
    };
    window.addEventListener('resize', handleResize);

    // Terminal Input -> Server
    term.onData((data) => {
      sendInput(data);
    });

    return () => {
      window.removeEventListener('resize', handleResize);
      term.dispose();
      fitAddon.dispose();
      xtermRef.current = null;
    };
  }, [sendInput]); // Init once

  // Server Output -> Terminal
  useEffect(() => {
    const cleanup = onData((data) => {
      if (xtermRef.current) {
        xtermRef.current.write(data);
      }
    });
    
    return cleanup;
  }, [onData]);

  return (
    <div 
      data-testid="terminal-container"
      className="w-full max-w-4xl h-[500px] bg-slate-900/80 backdrop-blur-xl rounded-xl border border-slate-700/50 shadow-2xl overflow-hidden flex flex-col my-8"
    >
      <div className="bg-slate-800/80 p-3 flex items-center justify-between border-b border-slate-700/50 backdrop-blur-md">
        <div className="flex items-center space-x-2">
          <div className="w-3 h-3 rounded-full bg-red-500/80 hover:bg-red-500 transition-colors"></div>
          <div className="w-3 h-3 rounded-full bg-yellow-500/80 hover:bg-yellow-500 transition-colors"></div>
          <div className="w-3 h-3 rounded-full bg-green-500/80 hover:bg-green-500 transition-colors"></div>
        </div>
        <div className="text-xs text-slate-400 font-mono flex items-center">
            <span className="mr-2">bash</span>
        </div>
      </div>
      <div className="flex-1 p-4 relative bg-slate-950/50">
        <div className="h-full w-full" ref={terminalRef} />
      </div>
    </div>
  );
};
