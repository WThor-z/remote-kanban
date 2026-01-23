export interface Message {
    raw: string;
    content: string;
}
export declare class Parser {
    parse(raw: string): Message;
}
