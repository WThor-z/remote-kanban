# Workspace Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Introduce first-class Workspace domain support (model, persistence, and API) so workspace entities exist as real backend data instead of implicit folder conventions.

**Architecture:** Add a new `workspace` module in `vk-core` with file-backed `WorkspaceStore`, then wire it into `api-server` state and expose REST routes for list/create/get/update. Keep this slice additive and backward-compatible with current project/task flows.

**Tech Stack:** Rust (`axum`, `serde`, `tokio`, `uuid`, `chrono`), existing file-store patterns in `vk-core` and route testing style in `api-server`.

---

### Task 1: Add Workspace domain model and store in vk-core

**Files:**
- Create: `crates/core/src/workspace/mod.rs`
- Create: `crates/core/src/workspace/model.rs`
- Create: `crates/core/src/workspace/store.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/src/workspace/model.rs`, `crates/core/src/workspace/store.rs`

**Step 1: Write the failing tests (model + store)**
- Add tests for:
  - `Workspace::new()` default values.
  - Builder/update behavior (`with_slug`, archive semantics if included).
  - Store create/get/list/update persistence across instances.

**Step 2: Run tests to verify they fail**
- Run: `cargo test -p vk-core workspace::`
- Expected: FAIL because workspace module/types do not exist.

**Step 3: Write minimal implementation**
- Implement `Workspace` model with fields:
  - `id`, `name`, `slug`, `root_path`, `default_project_id`, `created_at`, `updated_at`, `archived_at`.
- Implement `CreateWorkspaceRequest` and `WorkspaceSummary`.
- Implement file-backed `WorkspaceStore` mirroring `ProjectStore` style.
- Export module from `crates/core/src/lib.rs`.

**Step 4: Run tests to verify pass**
- Run: `cargo test -p vk-core workspace::`
- Expected: PASS for new workspace tests.

### Task 2: Wire WorkspaceStore into api-server shared state

**Files:**
- Modify: `crates/api-server/src/state.rs`
- Test: existing API server tests compile and pass

**Step 1: Write failing state-level expectation**
- Add/adjust a state-focused test that requires `AppState` to expose `workspace_store()`.

**Step 2: Run test to verify failure**
- Run: `cargo test -p api-server workspace_store`
- Expected: FAIL because `workspace_store` not present.

**Step 3: Minimal implementation**
- Add `workspace_store: Arc<WorkspaceStore>` to `AppStateInner`.
- Initialize from `data_dir.join("workspaces.json")` in `AppState::with_stores`.
- Add getter methods `workspace_store()` / `workspace_store_arc()`.

**Step 4: Verify**
- Run: `cargo test -p api-server`
- Expected: Existing tests still pass.

### Task 3: Add Workspace REST routes

**Files:**
- Create: `crates/api-server/src/routes/workspace.rs`
- Modify: `crates/api-server/src/routes/mod.rs`
- Modify: `crates/api-server/src/main.rs`
- Test: `crates/api-server/src/routes/workspace.rs`

**Step 1: Write failing route tests first**
- Tests for:
  - `GET /api/workspaces` returns empty list initially.
  - `POST /api/workspaces` creates workspace and returns `201`.
  - `GET /api/workspaces/{id}` returns workspace.
  - `PATCH /api/workspaces/{id}` updates mutable fields.
  - Invalid ID -> `400`, missing workspace -> `404`.

**Step 2: Run tests to verify RED**
- Run: `cargo test -p api-server workspace::tests`
- Expected: FAIL because route/module not wired.

**Step 3: Minimal implementation**
- Implement route handlers + request/response DTOs following `project.rs` style.
- Register workspace router in `routes/mod.rs` and `main.rs`.

**Step 4: Verify GREEN**
- Run: `cargo test -p api-server workspace::tests`
- Expected: PASS.

### Task 4: Full verification for this slice

**Files:**
- Modify if needed: any touched by Tasks 1-3

**Step 1: Run focused suites**
- Run: `cargo test -p vk-core workspace::`
- Run: `cargo test -p api-server workspace::tests`

**Step 2: Run baseline safety suites**
- Run: `cargo test -p vk-core -p api-server`
- Run: `pnpm run test:scripts`

**Step 3: Record outcomes**
- Update `docs/plans/2026-02-08-workspace-redesign/progress.md` with pass/fail and notes.
