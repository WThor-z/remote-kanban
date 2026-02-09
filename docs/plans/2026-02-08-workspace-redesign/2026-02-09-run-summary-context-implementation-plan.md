# Run Summary Context Exposure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose workspace/project context from persisted runs to run summary APIs so downstream clients can reason about run lineage without reloading full run metadata.

**Architecture:** Extend `agent-runner` run summary model with optional context fields (`project_id`, `workspace_id`) sourced from `Run.metadata`, then propagate these fields through `api-server` run list responses. Keep everything backward-compatible by preserving optional semantics and existing run loading defaults.

**Tech Stack:** Rust (`agent-runner`, `api-server`, `serde`, `uuid`, `axum`) and TypeScript client hook typings.

---

### Task 1: Extend `RunSummary` with workspace/project context

**Files:**
- Modify: `crates/agent-runner/src/run.rs`
- Test: `crates/agent-runner/src/run.rs`

**Step 1: Write failing unit test first**
- Add a test asserting `RunSummary::from(&run)` carries `project_id` and `workspace_id` from `run.metadata`.

**Step 2: RED run**
- Run: `cargo test -p agent-runner run::tests::test_run_summary`
- Expected: FAIL due missing `RunSummary` context fields.

**Step 3: Minimal implementation**
- Add optional fields to `RunSummary`:
  - `project_id: Option<Uuid>`
  - `workspace_id: Option<Uuid>`
- Populate both in `impl From<&Run> for RunSummary`.

**Step 4: GREEN run**
- Run: `cargo test -p agent-runner run::tests::test_run_summary`
- Expected: PASS.

### Task 2: Expose run context in task run list API

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`
- Test: `crates/api-server/src/routes/task.rs`

**Step 1: Write failing route test first**
- Add test for `GET /api/tasks/{id}/runs` that saves a run with metadata context and verifies response includes `projectId`/`workspaceId`.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::task::tests::list_runs_includes_project_and_workspace_context`
- Expected: FAIL because response shape lacks context fields.

**Step 3: Minimal implementation**
- Add `project_id` and `workspace_id` fields to `RunSummaryResponse`.
- Map these fields in `impl From<RunSummary> for RunSummaryResponse`.

**Step 4: GREEN run**
- Re-run the focused route test above.
- Expected: PASS.

### Task 3: Align client run summary typings

**Files:**
- Modify: `packages/client/src/hooks/useTaskRuns.ts`

**Step 1: Add optional fields to type**
- Extend `RunSummary` type with optional `projectId` and `workspaceId`.

**Step 2: Typecheck verification**
- Run: `pnpm --filter @vk/client test -- --runInBand` (or existing client verification command if available).
- Expected: PASS without runtime behavior changes.

### Task 4: Verification + progress logging

**Files:**
- Modify: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Focused checks**
- `cargo test -p agent-runner run::tests::test_run_summary`
- `cargo test -p api-server routes::task::tests::list_runs_includes_project_and_workspace_context`

**Step 2: Full safety checks**
- `cargo test -p vk-core -p agent-runner -p api-server`
- `pnpm run test:scripts`

**Step 3: Record evidence**
- Append RED/GREEN results and final verification outputs to `progress.md` under M5 sections.
