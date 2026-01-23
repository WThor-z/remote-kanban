# @opencode-vibe/pty-manager

A wrapper around `node-pty` to manage shell processes for OpenCode Vibe.

## Installation

```bash
npm install @opencode-vibe/pty-manager
```

## Usage

```typescript
import { PtyManager } from '@opencode-vibe/pty-manager';

const manager = new PtyManager();
const ptyProcess = manager.spawn('bash', [], {
  name: 'xterm-color',
  cols: 80,
  rows: 30,
  cwd: process.env.HOME,
  env: process.env
});

ptyProcess.onData((data) => {
  console.log(data);
});

ptyProcess.write('ls\r');
```

## Features

- Spawns shell processes using `node-pty`.
- Typescript support.
- Configurable options.
