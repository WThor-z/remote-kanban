export type MessageType = 'command' | 'log' | 'status' | 'output';
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';
export interface Message {
    raw: string;
    content: string;
    type: MessageType;
    level?: LogLevel;
}
export declare class Parser {
    parse(raw: string): Message;
}
