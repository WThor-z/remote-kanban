# @opencode-vibe/client

The frontend client for Opencode Vibe Kanban. Built with React, Vite, and Tailwind CSS.

## Features

- **Real-time**: Connects to the server via WebSocket (Socket.IO).
- **Terminal Integration**: Uses `xterm.js` for terminal rendering (upcoming).
- **Modern UI**: Styled with Tailwind CSS for a premium look.

## Inputs

- **WebSocket Events**: Listens for terminal data and system status from the server.
- **User Interactions**: Sends commands and UI actions to the server.

## Outputs

- **Web Interface**: A responsive web application.

## Usage

### Development

```bash
npm run dev
```

### Build

```bash
npm run build
```

### Test

```bash
npm test
# or
npx vitest run
```
