export type MessageType = 'command' | 'log' | 'status' | 'output';
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

// Re-export Kanban types and utilities
export * from './kanban.js';

// Re-export Agent types and utilities
export * from './agent.js';

export interface Message {
  raw: string;
  content: string;
  type: MessageType;
  level?: LogLevel;
}

export class Parser {
  parse(raw: string): Message {
    const content = stripAnsi(raw).trimEnd();
    const jsonMessage = parseJsonMessage(content.trim());
    if (jsonMessage) {
      return {
        raw,
        content: jsonMessage.content,
        type: jsonMessage.type,
        level: jsonMessage.level,
      };
    }

    const logMatch = content.match(/^\s*\[(INFO|WARN|ERROR|DEBUG)\]\s*(.*)$/);
    if (logMatch) {
      const level = mapLogLevel(logMatch[1]);
      return {
        raw,
        content: logMatch[2] || '',
        type: 'log',
        level,
      };
    }

    const statusMatch = content.match(/^\s*STATUS:\s*(.*)$/);
    if (statusMatch) {
      return {
        raw,
        content: statusMatch[1] || '',
        type: 'status',
      };
    }

    const commandMatch = content.match(/^\s*\$\s+(.*)$/);
    if (commandMatch) {
      return {
        raw,
        content: commandMatch[1] || '',
        type: 'command',
      };
    }

    return {
      raw,
      content,
      type: 'output',
    };
  }
}

const stripAnsi = (value: string) => {
  return value.replace(/\x1b\[[0-9;]*m/g, '');
};

const parseJsonMessage = (value: string): Message | null => {
  if (!value.startsWith('{') || !value.endsWith('}')) {
    return null;
  }

  try {
    const parsed = JSON.parse(value) as Partial<Message> & {
      type?: MessageType;
      content?: string;
      level?: LogLevel;
    };

    if (!parsed.type || typeof parsed.content !== 'string') {
      return null;
    }

    if (parsed.type === 'log' && parsed.level && !isLogLevel(parsed.level)) {
      return null;
    }

    if (!isMessageType(parsed.type)) {
      return null;
    }

    return {
      raw: value,
      content: parsed.content,
      type: parsed.type,
      level: parsed.level,
    };
  } catch {
    return null;
  }
};

const mapLogLevel = (level: string): LogLevel => {
  switch (level) {
    case 'DEBUG':
      return 'debug';
    case 'INFO':
      return 'info';
    case 'WARN':
      return 'warn';
    default:
      return 'error';
  }
};

const isLogLevel = (value: string): value is LogLevel => {
  return value === 'debug' || value === 'info' || value === 'warn' || value === 'error';
};

const isMessageType = (value: string): value is MessageType => {
  return value === 'command' || value === 'log' || value === 'status' || value === 'output';
};
