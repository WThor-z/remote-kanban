import * as pty from 'node-pty';
export declare class PtyManager {
    spawn(file: string, args: string[] | string, options?: pty.IPtyForkOptions | pty.IWindowsPtyForkOptions): pty.IPty;
    write(terminal: pty.IPty, data: string): void;
    onData(terminal: pty.IPty, handler: (data: string) => void): pty.IDisposable;
    resize(terminal: pty.IPty, cols: number, rows: number): void;
}
