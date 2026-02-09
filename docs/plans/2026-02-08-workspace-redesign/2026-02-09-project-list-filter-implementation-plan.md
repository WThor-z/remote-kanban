# Project List Workspace Filter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add workspace-scoped project listing so clients can load only projects in the active workspace while keeping default behavior unchanged.

**Architecture:** Extend `GET /api/projects` with an optional `workspaceId` query parameter and filter existing project summaries in-memory. Keep endpoint backward-compatible by returning all projects when the query is absent. Mirror this in client `useProjects` by supporting optional workspace-scoped fetches and aligning the `Project` type with backend payload.

**Tech Stack:** Rust (`axum`, `serde`, `uuid`) for `api-server` routes; TypeScript for client hook typing/fetch.

---

### Task 1: Add failing API test for workspace filtering

**Files:**
- Modify: `crates/api-server/src/routes/project.rs`

**Step 1: Write failing test first**
- Add test `list_projects_supports_workspace_filter` that creates two workspaces + two projects and verifies `GET /api/projects?workspaceId=<id>` only returns matching project.

**Step 2: RED run**
- Run: `cargo test -p api-server routes::project::tests::list_projects_supports_workspace_filter`
- Expected: FAIL because list endpoint currently returns all projects.

### Task 2: Implement `workspaceId` query filtering

**Files:**
- Modify: `crates/api-server/src/routes/project.rs`

**Step 1: Minimal implementation**
- Add `ListProjectsQuery` with optional `workspace_id` (camelCase).
- Update `list_projects` handler to accept query and filter summaries when `workspace_id` is provided.

**Step 2: GREEN run**
- Re-run focused RED command.
- Expected: PASS.

### Task 3: Client `useProjects` alignment

**Files:**
- Modify: `packages/client/src/hooks/useProjects.ts`

**Step 1: Type + API adjustments**
- Add `workspaceId` to `Project` interface.
- Add optional hook options (`workspaceId`) and include query serialization in fetch URL.

**Step 2: Client verification**
- Run: `pnpm --filter client test`
- Expected: PASS.

### Task 4: Verification + progress logging

**Files:**
- Modify: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Focused checks**
- `cargo test -p api-server routes::project::tests::list_projects_supports_workspace_filter`
- `cargo test -p api-server routes::project::tests::list_projects_includes_workspace_id`

**Step 2: Safety checks**
- `cargo test -p vk-core -p agent-runner -p api-server`
- `pnpm run test:scripts`
- `pnpm --filter client test`

**Step 3: Record evidence**
- Append M8 RED/GREEN evidence and verification outputs in `progress.md`.
