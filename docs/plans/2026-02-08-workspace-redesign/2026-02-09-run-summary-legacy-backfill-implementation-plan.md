# Run Summary Legacy Context Backfill Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure legacy runs missing context metadata still return `projectId` and `workspaceId` in task run list responses by backfilling from the bound task.

**Architecture:** Keep persisted run data unchanged; perform response-time backfill in `list_task_runs` by merging `RunSummary` context with `Task` bindings (`run context` first, `task binding` fallback). This preserves backward compatibility without mutating historical run files.

**Tech Stack:** Rust (`axum`, `uuid`, `serde`) in `api-server` route layer.

---

### Task 1: Add failing regression test for legacy run summaries

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`
- Test: `crates/api-server/src/routes/task.rs`

**Step 1: Write failing test first**
- Add test that:
  - creates a task with `project_id` + `workspace_id` bindings,
  - saves a legacy-style run summary (no run metadata context),
  - calls `GET /api/tasks/{id}/runs`,
  - expects response `projectId` + `workspaceId` to be backfilled from task.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::task::tests::list_runs_backfills_legacy_context_from_task_binding`
- Expected: FAIL because response currently returns `null` for missing run context.

### Task 2: Implement response-time backfill logic

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`

**Step 1: Minimal implementation**
- In `list_task_runs`, after loading task + runs, map each run summary into response with fallback:
  - `project_id = run.project_id.or(task.project_id)`
  - `workspace_id = run.workspace_id.or(task.workspace_id)`

**Step 2: GREEN run**
- Re-run: `cargo test -p api-server routes::task::tests::list_runs_backfills_legacy_context_from_task_binding`
- Expected: PASS.

### Task 3: Verification sweep + progress logging

**Files:**
- Modify: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Focused checks**
- `cargo test -p api-server routes::task::tests::list_runs_backfills_legacy_context_from_task_binding`
- `cargo test -p api-server routes::task::tests::list_runs_includes_project_and_workspace_context`

**Step 2: Safety checks**
- `cargo test -p vk-core -p agent-runner -p api-server`
- `pnpm run test:scripts`

**Step 3: Record evidence**
- Append RED/GREEN results and final verification outcomes to `progress.md` under M6 section.
