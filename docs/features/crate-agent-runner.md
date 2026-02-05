# Crate: agent-runner

## Summary
Task execution engine that creates worktrees, runs agents, and persists run metadata and events.

## Entry Points
- UI: N/A
- API: Rust crate `agent-runner` used by `api-server`
- CLI: N/A

## Behavior and Boundaries
- Manages execution sessions and emits `ExecutionEvent` streams.
- Creates and removes git worktrees via `git-worktree`.
- Uses `AGENT_WORKER_URL` to call a worker service when executing tasks.

## Data and Storage Impact
- Persists run data under `data_dir/runs/<task_id>/<run_id>/` (run.json, events.jsonl, messages.jsonl).
- Uses `data_dir` configured in `ExecutorConfig` (default `.vk-data`).

## Permissions and Risks
- Requires access to the repository path for worktree operations.
- Writes run artifacts to the configured data directory.

## Observability
- Emits `ExecutionEvent` and logs with `tracing`.

## Test and Verification
- Run `cargo test -p agent-runner`.

## Related Changes
- Used by `crates/api-server` executor routes.
