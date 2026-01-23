export interface Message {
  raw: string;
  content: string;
}

export class Parser {
  parse(raw: string): Message {
    return {
      raw,
      content: raw, // For MVP, content is just the raw string
    };
  }
}
