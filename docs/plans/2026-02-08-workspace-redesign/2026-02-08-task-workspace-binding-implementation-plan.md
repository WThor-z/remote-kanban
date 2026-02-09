# Task Workspace Binding Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure task records carry workspace binding and remain consistent with project binding.

**Architecture:** Extend core task model with workspace linkage, set workspace binding automatically at task creation from selected project, and enforce workspace/project consistency during execution.

**Tech Stack:** Rust (`axum`, `serde`, `uuid`, `tokio`) in `vk-core` and `api-server` route layers.

---

### Task 1: Add workspace binding to core Task model

**Files:**
- Modify: `crates/core/src/task/model.rs`

**Step 1: Write failing tests first**
- Add tests for:
  - `with_workspace_id` sets workspace id.
  - `with_project_binding(project_id, workspace_id)` sets both ids consistently.

**Step 2: RED run**
- Run: `cargo test -p vk-core task::model::`
- Expected: FAIL due missing methods/fields.

**Step 3: Minimal implementation**
- Add `workspace_id: Option<Uuid>` to `Task`.
- Add builders:
  - `with_workspace_id(workspace_id)`
  - `with_project_binding(project_id, workspace_id)`

**Step 4: GREEN run**
- Run: `cargo test -p vk-core task::model::`
- Expected: PASS.

### Task 2: Set and expose task workspace binding in task routes

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`

**Step 1: Write failing route tests first**
- Add/update tests to verify:
  - Creating task with valid project returns `workspaceId` equal to project’s workspace.
  - Task response includes `workspaceId`.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::task::tests::create_task_with_valid_project_sets_project_id`
- Expected: FAIL due missing response field/workspace assignment.

**Step 3: Minimal implementation**
- Add `workspace_id` to `TaskResponse` with camelCase serialization.
- In create handler, after project lookup, create task via `with_project_binding(project_id, project.workspace_id)`.
- Map `Task.workspace_id` into `TaskResponse`.

**Step 4: GREEN run**
- Re-run focused task route test(s), expected PASS.

### Task 3: Enforce project/task workspace consistency in executor route

**Files:**
- Modify: `crates/api-server/src/routes/executor.rs`

**Step 1: Write failing tests first**
- Add tests:
  - execution rejects task when `task.workspace_id` mismatches `project.workspace_id` (`409`).
  - execution accepts when binding matches.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::executor::tests::start_execution_`
- Expected: FAIL before consistency check is implemented.

**Step 3: Minimal implementation**
- In execution start path, after project lookup:
  - if `task.workspace_id` is `Some` and differs from project workspace -> return `409`.
  - if `None`, backfill to project workspace and persist before dispatch.

**Step 4: GREEN run**
- Re-run focused executor tests, expected PASS.

### Task 4: Verification sweep for M3 slice

**Files:**
- Update evidence only: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Focused checks**
- `cargo test -p vk-core task::model::`
- `cargo test -p api-server routes::task::tests::create_task_with_valid_project_sets_project_id`
- `cargo test -p api-server routes::executor::tests::start_execution_`

**Step 2: Full safety checks**
- `cargo test -p vk-core -p api-server`
- `pnpm run test:scripts`

**Step 3: Record RED/GREEN and final results**
- Update progress evidence for each task.
