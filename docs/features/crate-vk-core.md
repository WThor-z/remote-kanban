# Crate: vk-core

## Summary
Core domain models and file-backed stores for tasks, kanban boards, projects, and runs used by the Rust backend.

## Entry Points
- UI: N/A
- API: Rust crate `vk-core` imported by `api-server` and `agent-runner`
- CLI: N/A

## Behavior and Boundaries
- Defines Task/Kanban/Project/Run data models and helpers.
- Provides JSON-backed stores for tasks, kanban, projects, and runs.
- Does not expose HTTP/WS endpoints or execute tasks directly.

## Data and Storage Impact
- Persists tasks to `tasks.json` and kanban state to `kanban.json` in a caller-provided data directory (commonly `.vk-data/`).
- Persists run data under `runs/<task_id>/<run_id>/` (run.json, events.jsonl, messages.jsonl) in the same data directory.
- Persists projects to a caller-provided `projects.json` path.

## Permissions and Risks
- File system writes to the configured data directory.

## Observability
- Uses `tracing` for persistence logs in run storage.

## Test and Verification
- Run `cargo test -p vk-core`.

## Related Changes
- Used by `crates/api-server` and `crates/agent-runner`.
