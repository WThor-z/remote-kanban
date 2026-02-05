# Crate: api-server

## Summary
Rust backend that serves REST APIs and Socket.IO events for tasks, runs, and gateway coordination.

## Entry Points
- UI: N/A
- API: REST server on port 8081; Socket.IO server on port 8080
- CLI: `cargo run -p api-server` (from `crates/`)

## Behavior and Boundaries
- Exposes REST routes for tasks, runs, executor controls, gateway management, and health checks.
- Hosts Socket.IO channels for kanban/task events used by the client.
- Delegates execution to `agent-runner` and remote hosts via `gateway` manager.

## Data and Storage Impact
- Uses `VK_DATA_DIR` (default `.vk-data`) for `tasks.json`, `kanban.json`, `runs/`, and `worktrees/`.
- Worktrees are created under `data_dir/worktrees` with `task/` branch prefixes.

## Permissions and Risks
- Writes to the configured data directory and creates git worktrees in the repo path.

## Observability
- Emits Socket.IO events (`kanban:*`, `task:*`) and logs via `tracing`.

## Test and Verification
- Run `cargo test -p api-server`.

## Related Changes
- REST routes in `crates/api-server/src/routes/*`.
