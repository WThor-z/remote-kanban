import { describe, it, expect } from 'vitest';
import { Parser } from '../src/index';

describe('Parser', () => {
  it('should parse a simple raw string into a Message object', () => {
    const raw = "\x1b[32mThinking...\x1b[0m";
    const parser = new Parser();
    const message = parser.parse(raw);

    expect(message).toBeDefined();
    expect(message.raw).toBe(raw);
    // For now, we expect content to be the stripped string or just handled simply.
    // The requirement says "can correctly parse a raw string... into a structured Message object"
    // We'll assume Message has at least 'raw' and 'content' fields.
    // Since complex ANSI parsing is not required yet, we might just store the raw string.
    // But let's check for a basic structure.
    expect(message.content).toBeDefined();
  });
});
