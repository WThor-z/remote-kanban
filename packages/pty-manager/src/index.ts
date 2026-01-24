import * as pty from 'node-pty';

export class PtyManager {
  spawn(file: string, args: string[] | string, options?: pty.IPtyForkOptions | pty.IWindowsPtyForkOptions): pty.IPty {
    const defaultOptions: pty.IPtyForkOptions = {
        name: 'xterm-color',
        cols: 80,
        rows: 30,
        cwd: process.cwd(),
        env: process.env as { [key: string]: string }
    };

    const finalOptions = { ...defaultOptions, ...options };

    // On Windows, use conpty: false to avoid AttachConsole errors in some environments
    if (process.platform === 'win32') {
      (finalOptions as pty.IWindowsPtyForkOptions).useConpty = false;
    }

    return pty.spawn(file, args, finalOptions);
  }

  write(terminal: pty.IPty, data: string): void {
    terminal.write(data);
  }

  onData(terminal: pty.IPty, handler: (data: string) => void) {
    return terminal.onData(handler);
  }

  resize(terminal: pty.IPty, cols: number, rows: number): void {
    terminal.resize(cols, rows);
  }
}
