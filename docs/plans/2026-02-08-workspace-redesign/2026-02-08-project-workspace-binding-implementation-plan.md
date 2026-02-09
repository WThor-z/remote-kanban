# Project Workspace Binding Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enforce workspace binding for all projects by introducing `workspaceId` as a required project field and keeping persistence/runtime initialization migration-safe.

**Architecture:** Extend `vk-core` project model/store with required workspace linkage, add startup default-workspace bootstrap in `AppState`, and propagate `workspaceId` through project API responses and existing test setup helpers.

**Tech Stack:** Rust (`axum`, `serde`, `uuid`, `tokio`, `chrono`) with existing file-store patterns in `vk-core` and route/state tests in `api-server`.

---

### Task 1: Add required workspace binding to core Project model/store

**Files:**
- Modify: `crates/core/src/project/model.rs`
- Modify: `crates/core/src/project/store.rs`
- Test: project model/store tests in same files

**Step 1: Write failing tests first**
- Add tests validating:
  - `Project::new(...)` requires and stores `workspace_id`.
  - `CreateProjectRequest` requires `workspace_id` for register.
  - Legacy loaded projects without `workspace_id` are migrated to provided default workspace id.

**Step 2: RED run**
- Run: `cargo test -p vk-core project::`
- Expected: FAIL due to missing workspace binding behavior/signature mismatches.

**Step 3: Minimal implementation**
- Add `workspace_id: Uuid` to `Project`, `CreateProjectRequest`, and `ProjectSummary`.
- Update `Project::new` signature to include workspace id.
- Update `ProjectStore::new` to accept `default_workspace_id` and migrate legacy entries lacking workspace id.
- Update `register`/`update` logic and tests for new field.

**Step 4: GREEN run**
- Run: `cargo test -p vk-core project::`
- Expected: PASS.

### Task 2: Bootstrap default workspace id in AppState and wire ProjectStore init

**Files:**
- Modify: `crates/api-server/src/state.rs`
- Test: state tests in same file

**Step 1: Write failing test first**
- Add test that `AppState::with_stores` ensures at least one workspace exists and project store can register with that workspace id.

**Step 2: RED run**
- Run: `cargo test -p api-server app_state_`
- Expected: FAIL for missing bootstrap behavior/signature mismatch.

**Step 3: Minimal implementation**
- In `with_stores`, initialize `WorkspaceStore` first.
- Ensure default workspace exists (create when empty).
- Initialize `ProjectStore` with discovered default workspace id.

**Step 4: GREEN run**
- Run: `cargo test -p api-server app_state_`
- Expected: PASS.

### Task 3: Propagate workspaceId through project APIs and update test fixtures

**Files:**
- Modify: `crates/api-server/src/routes/project.rs`
- Modify: `crates/api-server/src/routes/task.rs` (test fixtures)
- Modify: `crates/api-server/src/routes/executor.rs` (test fixtures)

**Step 1: Write/adjust failing tests first**
- Add route tests (or update existing fixture assertions) for `workspaceId` in project detail/list payloads.
- Update task/executor tests that construct `CreateProjectRequest` so they fail until `workspace_id` is supplied.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::project`
- Run: `cargo test -p api-server routes::task::tests::create_task_with_valid_project_sets_project_id`
- Run: `cargo test -p api-server routes::executor::tests::start_execution_dispatches_project_cwd_to_bound_host`
- Expected: FAIL before implementation alignment.

**Step 3: Minimal implementation**
- Include `workspaceId` in project route response DTOs.
- Ensure all `CreateProjectRequest` constructions include a workspace id (using created/default workspace in tests).

**Step 4: GREEN run**
- Re-run above focused tests; expected PASS.

### Task 4: Verification sweep for M2 slice

**Files:**
- Update evidence only: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Run focused suites**
- `cargo test -p vk-core project::`
- `cargo test -p api-server routes::project`

**Step 2: Run full safety suites**
- `cargo test -p vk-core -p api-server`
- `pnpm run test:scripts`

**Step 3: Record outputs**
- Capture RED/GREEN evidence and final PASS status in progress file.
