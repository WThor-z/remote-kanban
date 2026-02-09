# Run Context Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Persist workspace/project context on run records and propagate that context through execution dispatch without breaking existing run data.

**Architecture:** Extend run metadata with optional `project_id` and `workspace_id` fields, thread these values from task/project lookup into gateway dispatch and run persistence, and preserve backward compatibility by defaulting absent fields when loading legacy `run.json` files.

**Tech Stack:** Rust (`serde`, `uuid`, `axum`, `tokio`) in `agent-runner` and `api-server`.

---

### Task 1: Add run metadata context fields

**Files:**
- Modify: `crates/agent-runner/src/run.rs`
- Test: `crates/agent-runner/src/run.rs`

**Step 1: Write failing tests first**
- Add tests asserting `RunMetadata` supports `project_id` and `workspace_id` and defaults to `None` on new runs.

**Step 2: RED run**
- Run: `cargo test -p agent-runner run::tests::test_run_metadata`
- Expected: FAIL due missing metadata fields.

**Step 3: Minimal implementation**
- Add optional fields to `RunMetadata`:
  - `project_id: Option<Uuid>`
  - `workspace_id: Option<Uuid>`
- Mark both with `#[serde(default)]`.

**Step 4: GREEN run**
- Run: `cargo test -p agent-runner run::tests::test_run_`
- Expected: PASS.

### Task 2: Propagate context through execution route and persisted runs

**Files:**
- Modify: `crates/api-server/src/routes/executor.rs`
- Test: `crates/api-server/src/routes/executor.rs`

**Step 1: Write failing test first**
- Extend route test coverage so execution dispatch must include `projectId` and `workspaceId` in gateway metadata and persisted run metadata.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::executor::tests::start_execution_dispatches_project_cwd_to_bound_host`
- Expected: FAIL because metadata is currently `Null` and run context is missing.

**Step 3: Minimal implementation**
- Thread project/workspace IDs into `dispatch_to_gateway`.
- Set gateway task metadata JSON:
  - `projectId`
  - `workspaceId`
- Set run metadata context fields on initial run save and terminal run updates.

**Step 4: GREEN run**
- Re-run: `cargo test -p api-server routes::executor::tests::start_execution_dispatches_project_cwd_to_bound_host`
- Expected: PASS.

### Task 3: Add backward-compatibility coverage for legacy run metadata

**Files:**
- Modify: `crates/agent-runner/src/persistence.rs`
- Test: `crates/agent-runner/src/persistence.rs`

**Step 1: Write persistence regression test**
- Add test that writes legacy `run.json` without context fields and verifies `load_run(...)` still succeeds with both values as `None`.

**Step 2: Verify behavior**
- Run: `cargo test -p agent-runner persistence::tests::`
- Expected: PASS with legacy compatibility test included.

### Task 4: Verification sweep for M4

**Files:**
- Update evidence: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Focused checks**
- `cargo test -p agent-runner run::tests::test_run_metadata`
- `cargo test -p agent-runner persistence::tests::`
- `cargo test -p api-server routes::executor::tests::start_execution_`

**Step 2: Full safety checks**
- `cargo test -p vk-core -p agent-runner -p api-server`
- `pnpm run test:scripts`

**Step 3: Record outcomes**
- Document RED/GREEN evidence and final pass status in `progress.md`.
