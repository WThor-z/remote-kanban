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

    return pty.spawn(file, args, finalOptions);
  }
}
