# Crate: git-worktree

## Summary
Git worktree management utilities for isolating task branches and working directories.

## Entry Points
- UI: N/A
- API: Rust crate `git-worktree` used by `agent-runner` and `api-server`
- CLI: N/A

## Behavior and Boundaries
- Creates, lists, and removes git worktrees and task branches.
- Requires the repository path to be a valid git repository.

## Data and Storage Impact
- Creates worktrees under the configured directory (default `.worktrees` relative to repo root).
- Uses branch prefixes (default `task/`).

## Permissions and Risks
- Executes git commands in the repository path.
- Can create or remove worktrees/branches as part of task execution.

## Observability
- Logs worktree operations via `tracing`.

## Test and Verification
- Run `cargo test -p git-worktree`.

## Related Changes
- Used by `crates/agent-runner`.
