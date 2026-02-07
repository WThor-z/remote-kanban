# Runs and Worktrees Permanent Deletion Design

Date: 2026-02-04

## Overview

Runs and worktrees remain persisted by default to preserve history and enable
post-run inspection. This design adds explicit, fine-grained REST endpoints to
permanently delete run data and to clean up worktrees on demand, so storage does
not grow without bound.

## Goals

- Provide API paths to permanently delete single runs and all runs for a task.
- Keep existing worktree persistence, while exposing a clear deletion endpoint.
- Prevent deletion of active runs to avoid races and partial data loss.
- Preserve current default behavior and backward compatibility.

## Non-Goals

- Automatic retention policies or scheduled cleanup (future enhancement).
- Changing run/worktree persistence defaults.
- Deleting tasks or task metadata as part of run cleanup.

## API Surface

- DELETE /api/tasks/{id}/runs/{run_id}
  - Deletes the run directory: run.json, events.jsonl, messages.jsonl.
  - 204 on success, 404 if task or run does not exist, 409 if run is active.

- DELETE /api/tasks/{id}/runs
  - Deletes all runs for a task (data_dir/runs/{task}).
  - 204 on success, 404 if task does not exist, 409 if any run is active.

- POST /api/tasks/{id}/cleanup (existing)
  - Triggers Gateway worktree removal for the task.

- DELETE /api/tasks/{id}/worktree (alias)
  - Calls the same handler as /cleanup for clearer semantics.

## Data Flow and Safety

1. Run deletion
   - Validate task exists.
   - Confirm run exists; load run metadata to inspect status.
   - If run status is non-terminal (Initializing/Running/Paused), return 409.
   - Call RunStore::delete_run and return 204.

2. Task run deletion
   - Validate task exists.
   - List runs and reject deletion if any run is active (409).
   - Call RunStore::delete_task_runs and return 204.

3. Worktree cleanup
   - Validate task exists and has worktree_path; otherwise 400.
   - Optionally reject if task has active runs (409).
   - Send GitOperationType::Cleanup to Gateway.
   - On success, task.git_status = none and worktree_path = null.

## Error Handling

- 404 for missing task or run.
- 409 for active runs to prevent deletion during execution.
- 400 for worktree cleanup when no worktree exists.
- 500 for Gateway or file system errors.

## Testing

- RunStore deletion tests using TempDir to verify directory removal and idempotency.
- API route tests for 204/404/409 branches on both run deletion endpoints.
- Worktree cleanup alias test to ensure behavior matches /cleanup.

## Compatibility

- No change to persistence defaults.
- Existing clients continue to work without changes.
- New endpoints are opt-in for manual cleanup.
