"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.startServer = startServer;
const express_1 = __importDefault(require("express"));
const http_1 = require("http");
const socket_io_1 = require("socket.io");
const cors_1 = __importDefault(require("cors"));
const pty_manager_1 = require("@opencode-vibe/pty-manager");
function startServer(port = 3000) {
    const app = (0, express_1.default)();
    app.use((0, cors_1.default)());
    const httpServer = (0, http_1.createServer)(app);
    const io = new socket_io_1.Server(httpServer, {
        cors: {
            origin: '*',
            methods: ['GET', 'POST']
        }
    });
    // Initialize PtyManager - intended for use in future socket events
    const ptyManager = new pty_manager_1.PtyManager();
    io.on('connection', (socket) => {
        console.log('Client connected:', socket.id);
        // Spawn a shell for this client
        // For MVP, we spawn a new shell for each connection, 
        // or we could use a session ID to reconnect to existing PTYs.
        // Let's keep it simple: One shell per socket.
        try {
            const shellCmd = process.platform === 'win32' ? 'powershell.exe' : 'bash';
            const shell = ptyManager.spawn(shellCmd, [], {
                cols: 80,
                rows: 24,
                cwd: process.cwd(),
                env: process.env
            });
            // Handle incoming data from client
            socket.on('input', (data) => {
                try {
                    shell.write(data);
                }
                catch (err) {
                    console.error('Error writing to shell:', err);
                }
            });
            // Handle outgoing data from shell
            const subscription = shell.onData((data) => {
                socket.emit('output', data);
            });
            socket.on('disconnect', () => {
                console.log('Client disconnected:', socket.id);
                try {
                    shell.kill();
                    subscription.dispose();
                }
                catch (err) {
                    console.error('Error cleanup shell:', err);
                }
            });
        }
        catch (err) {
            console.error('Failed to spawn shell:', err);
            socket.emit('output', '\r\n\x1b[31mError: Failed to spawn shell process.\x1b[0m\r\n');
        }
    });
    httpServer.listen(port);
    return {
        httpServer,
        io,
        stop: () => {
            io.close();
            httpServer.close();
        }
    };
}
