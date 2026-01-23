import * as pty from 'node-pty';
export declare class PtyManager {
    spawn(file: string, args: string[] | string, options?: pty.IPtyForkOptions | pty.IWindowsPtyForkOptions): pty.IPty;
}
