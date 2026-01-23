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
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const vitest_1 = require("vitest");
const pty = __importStar(require("node-pty"));
const index_1 = require("../src/index");
// Mock node-pty
vitest_1.vi.mock('node-pty', () => {
    return {
        spawn: vitest_1.vi.fn(),
    };
});
(0, vitest_1.describe)('PtyManager', () => {
    (0, vitest_1.beforeEach)(() => {
        vitest_1.vi.clearAllMocks();
    });
    (0, vitest_1.it)('should spawn a process using node-pty', () => {
        const manager = new index_1.PtyManager();
        const mockTerminal = {
            onData: vitest_1.vi.fn(),
            write: vitest_1.vi.fn(),
            resize: vitest_1.vi.fn(),
            kill: vitest_1.vi.fn(),
            onExit: vitest_1.vi.fn(),
            pid: 123
        };
        // Setup the mock return value
        vitest_1.vi.mocked(pty.spawn).mockReturnValue(mockTerminal);
        const shell = 'bash';
        const args = ['-c', 'echo hello'];
        // Call the method
        const term = manager.spawn(shell, args);
        // Assertions
        (0, vitest_1.expect)(pty.spawn).toHaveBeenCalledTimes(1);
        (0, vitest_1.expect)(pty.spawn).toHaveBeenCalledWith(shell, args, vitest_1.expect.any(Object));
        (0, vitest_1.expect)(term).toBe(mockTerminal);
    });
    (0, vitest_1.it)('should write data to the terminal', () => {
        const manager = new index_1.PtyManager();
        const mockTerminal = {
            write: vitest_1.vi.fn(),
        };
        manager.write(mockTerminal, 'ls');
        (0, vitest_1.expect)(mockTerminal.write).toHaveBeenCalledWith('ls');
    });
    (0, vitest_1.it)('should subscribe to terminal output', () => {
        const manager = new index_1.PtyManager();
        const subscription = { dispose: vitest_1.vi.fn() };
        const mockTerminal = {
            onData: vitest_1.vi.fn().mockReturnValue(subscription),
        };
        const handler = vitest_1.vi.fn();
        const result = manager.onData(mockTerminal, handler);
        (0, vitest_1.expect)(mockTerminal.onData).toHaveBeenCalledWith(handler);
        (0, vitest_1.expect)(result).toBe(subscription);
    });
    (0, vitest_1.it)('should resize the terminal', () => {
        const manager = new index_1.PtyManager();
        const mockTerminal = {
            resize: vitest_1.vi.fn(),
        };
        manager.resize(mockTerminal, 120, 40);
        (0, vitest_1.expect)(mockTerminal.resize).toHaveBeenCalledWith(120, 40);
    });
});
