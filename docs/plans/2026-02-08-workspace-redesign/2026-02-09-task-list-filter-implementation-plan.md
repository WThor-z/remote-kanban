# Task List Workspace/Project Filter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add workspace/project filtering to task listing so clients can request scoped task views in multi-workspace workflows.

**Architecture:** Extend `GET /api/tasks` with optional query parameters (`projectId`, `workspaceId`) and apply AND filtering in the route layer after loading tasks. Keep the endpoint backward-compatible by returning all tasks when no filter params are provided.

**Tech Stack:** Rust (`axum`, `serde`, `uuid`) in `api-server`; TypeScript type updates in `packages/client`.

---

### Task 1: Add failing route test for task filters

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`

**Step 1: Write failing test first**
- Add test asserting:
  - `GET /api/tasks?projectId=...` only returns tasks bound to that project.
  - `GET /api/tasks?workspaceId=...` only returns tasks bound to that workspace.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::task::tests::list_tasks_supports_project_and_workspace_filters`
- Expected: FAIL because query filtering is not implemented.

### Task 2: Implement list query + filter logic

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`

**Step 1: Minimal implementation**
- Add `ListTasksQuery` with optional `project_id` and `workspace_id` (`camelCase`).
- Update `list_tasks` handler to accept `Query<ListTasksQuery>`.
- Apply filter predicate:
  - `project_id` match if provided
  - `workspace_id` match if provided
  - both provided => AND behavior.

**Step 2: GREEN run**
- Re-run focused test command above.
- Expected: PASS.

### Task 3: Client API alignment

**Files:**
- Modify: `packages/client/src/hooks/useTaskApi.ts`

**Step 1: Type/API adjustments**
- Add `workspaceId` to `Task` type.
- Extend `fetchTasks` with optional filter args and query serialization.

**Step 2: Verification**
- Run: `pnpm --filter client test`
- Expected: PASS.

### Task 4: Verification + progress logging

**Files:**
- Modify: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Focused checks**
- `cargo test -p api-server routes::task::tests::list_tasks_supports_project_and_workspace_filters`
- `cargo test -p api-server routes::task::tests::list_runs_backfills_legacy_context_from_task_binding`

**Step 2: Safety checks**
- `cargo test -p vk-core -p agent-runner -p api-server`
- `pnpm run test:scripts`
- `pnpm --filter client test`

**Step 3: Record evidence**
- Append M7 RED/GREEN and verification outcomes to `progress.md`.
