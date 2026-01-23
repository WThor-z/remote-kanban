"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const vitest_1 = require("vitest");
const socket_io_client_1 = require("socket.io-client");
const server_1 = require("../src/server");
// Mock PtyManager
vitest_1.vi.mock('@opencode-vibe/pty-manager', () => {
    return {
        PtyManager: vitest_1.vi.fn().mockImplementation(() => ({
            spawn: vitest_1.vi.fn().mockReturnValue({
                on: vitest_1.vi.fn(),
                write: vitest_1.vi.fn(),
                kill: vitest_1.vi.fn(),
            }),
        })),
    };
});
(0, vitest_1.describe)('Gateway Server', () => {
    let clientSocket;
    let httpServer;
    let port;
    let stopServer;
    (0, vitest_1.beforeAll)(async () => {
        // Start the server on port 0 for random available port
        const app = (0, server_1.startServer)(0);
        httpServer = app.httpServer;
        stopServer = app.stop;
        await new Promise((resolve) => {
            httpServer.on('listening', () => {
                port = httpServer.address().port;
                resolve();
            });
        });
    });
    (0, vitest_1.afterAll)(() => {
        if (stopServer)
            stopServer();
        if (clientSocket)
            clientSocket.disconnect();
    });
    (0, vitest_1.it)('should allow a client to connect', () => new Promise((done) => {
        clientSocket = (0, socket_io_client_1.io)(`http://localhost:${port}`);
        clientSocket.on('connect', () => {
            (0, vitest_1.expect)(clientSocket.connected).toBe(true);
            done();
        });
    }));
});
