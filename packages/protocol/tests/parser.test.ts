import { describe, it, expect } from 'vitest';
import { Parser } from '../src/index';

describe('Parser', () => {
  it('strips ANSI codes and classifies output', () => {
    const raw = "\x1b[32mThinking...\x1b[0m";
    const parser = new Parser();
    const message = parser.parse(raw);

    expect(message.raw).toBe(raw);
    expect(message.content).toBe('Thinking...');
    expect(message.type).toBe('output');
  });

  it('parses log messages with levels', () => {
    const parser = new Parser();
    const message = parser.parse('[ERROR] Something went wrong');

    expect(message.type).toBe('log');
    expect(message.level).toBe('error');
    expect(message.content).toBe('Something went wrong');
  });

  it('parses status messages', () => {
    const parser = new Parser();
    const message = parser.parse('STATUS: Fetching dependencies');

    expect(message.type).toBe('status');
    expect(message.content).toBe('Fetching dependencies');
  });

  it('parses command messages', () => {
    const parser = new Parser();
    const message = parser.parse('$ npm run test');

    expect(message.type).toBe('command');
    expect(message.content).toBe('npm run test');
  });

  it('parses JSON payloads when available', () => {
    const parser = new Parser();
    const message = parser.parse('{"type":"status","content":"Ready"}');

    expect(message.type).toBe('status');
    expect(message.content).toBe('Ready');
  });
});
