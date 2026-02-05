# Feature Catalog

## User Features

| Feature | Summary | Status | Owner | Updated | Link |
|--------|---------|--------|-------|---------|------|
| Task Commands | Command-driven task creation and management via slash commands | Stable | TBD | 2026-02-05 | [Doc](task-commands.md) |

## Module Catalog

### Crates (Rust)

| Feature | Summary | Status | Owner | Updated | Link |
|--------|---------|--------|-------|---------|------|
| crate/vk-core | Core models and file-backed stores for tasks, kanban, projects, and runs | Active | TBD | 2026-02-05 | [Doc](crate-vk-core.md) |
| crate/api-server | Rust REST + Socket.IO backend for tasks, runs, and gateway coordination | Active | TBD | 2026-02-05 | [Doc](crate-api-server.md) |
| crate/agent-runner | Task execution engine with worktree isolation and run persistence | Active | TBD | 2026-02-05 | [Doc](crate-agent-runner.md) |
| crate/git-worktree | Git worktree management helpers for isolated task branches | Active | TBD | 2026-02-05 | [Doc](crate-git-worktree.md) |

### Packages (Node/TS)

| Feature | Summary | Status | Owner | Updated | Link |
|--------|---------|--------|-------|---------|------|
| package/protocol | Shared protocol types and parsers for kanban and agent messages | Stable | TBD | 2026-02-05 | [Doc](package-protocol.md) |
| package/server | Node Socket.IO server for kanban and agent session management | Active | TBD | 2026-02-05 | [Doc](package-server.md) |
| package/client | React UI for kanban tasks and execution control | Active | TBD | 2026-02-05 | [Doc](package-client.md) |
| package/pty-manager | Node PTY wrapper for process I/O (deprecated; retained for compatibility) | Deprecated | TBD | 2026-02-05 | [Doc](package-pty-manager.md) |

### Services

| Feature | Summary | Status | Owner | Updated | Link |
|--------|---------|--------|-------|---------|------|
| service/agent-gateway | Remote gateway that executes tasks and streams events | Active | TBD | 2026-02-05 | [Doc](service-agent-gateway.md) |
