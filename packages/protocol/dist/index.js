"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Parser = void 0;
// Re-export Kanban types and utilities
__exportStar(require("./kanban"), exports);
class Parser {
    parse(raw) {
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
exports.Parser = Parser;
const stripAnsi = (value) => {
    return value.replace(/\x1b\[[0-9;]*m/g, '');
};
const parseJsonMessage = (value) => {
    if (!value.startsWith('{') || !value.endsWith('}')) {
        return null;
    }
    try {
        const parsed = JSON.parse(value);
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
    }
    catch {
        return null;
    }
};
const mapLogLevel = (level) => {
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
const isLogLevel = (value) => {
    return value === 'debug' || value === 'info' || value === 'warn' || value === 'error';
};
const isMessageType = (value) => {
    return value === 'command' || value === 'log' || value === 'status' || value === 'output';
};
