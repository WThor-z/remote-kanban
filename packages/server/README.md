# @opencode-vibe/server

The WebSocket Gateway module for Opencode Vibe. It acts as the bridge between the Frontend and the PTY Manager.

## Features

- **WebSocket Server**: Manages real-time communication.
- **PTY Integration**: Spawns and controls pseudo-terminals via `@opencode-vibe/pty-manager`.
- **Protocol Enforced**: Uses `@opencode-vibe/protocol` for message definitions.

## Installation

This package is part of the workspace.

```bash
npm install
```

## Usage

### Development

```bash
npm start
# or
npm run dev
```

### Testing

```bash
npm test
```

## API

### WebSocket Events

- `connect`: Client connected.
- `disconnect`: Client disconnected.
- (More to be added as features are implemented)

## Architecture

1. **Frontend** connects to **Server** via WebSocket.
2. **Server** spawns a shell using **PtyManager**.
3. **Server** forwards shell output to Frontend.
4. **Server** forwards Frontend input to shell.
