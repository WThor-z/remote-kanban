# Project-Bound Task Execution Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every task execution resolve its working directory from a selected project (`projectId`) instead of host-level default cwd.

**Architecture:** Extend task data with `projectId`, wire project storage into the running API server state, and enforce project->host->cwd resolution at execution time. Frontend task creation requires project selection; backend dispatch only sends tasks to the project-bound host and fills `GatewayTaskRequest.cwd` from project metadata.

**Tech Stack:** Rust (`axum`, `vk-core`, `agent-runner`), TypeScript/React (`packages/client`), Node gateway service (`services/agent-gateway`).

---

### Task 1: Wire project module into runtime state (Rust)

**Files:**
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/api-server/src/state.rs`
- Modify: `crates/api-server/src/main.rs`
- Modify: `crates/api-server/src/routes/mod.rs`

**Step 1: Write failing test/build signal**

The existing `project` API routes are not part of the compiled router state.

**Step 2: Verify failure state**

Run: `cargo test --manifest-path crates/Cargo.toml -p api-server routes::project`

Expected: No compiled project route tests / project route module not wired in active router.

**Step 3: Implement minimal wiring**

- Export `project` module in `vk-core`:

```rust
// crates/core/src/lib.rs
pub mod project;
```

- Add `project_store: Arc<ProjectStore>` to `AppStateInner`.
- In `AppState::with_stores`, initialize project store from `data_dir.join("projects.json")`.
- Add accessors:
  - `pub fn project_store(&self) -> &ProjectStore`
  - `pub fn project_store_arc(&self) -> Arc<ProjectStore>`
- Register project routes:
  - `pub mod project;` in `routes/mod.rs`
  - `.merge(routes::project::router())` in `main.rs`

**Step 4: Verify wiring**

Run: `cargo test --manifest-path crates/Cargo.toml -p api-server`

Expected: `api-server` tests still pass with project routes compiled.

**Step 5: Commit**

```bash
git add crates/core/src/lib.rs crates/api-server/src/state.rs crates/api-server/src/main.rs crates/api-server/src/routes/mod.rs
git commit -m "feat(api): wire project store into runtime state"
```

---

### Task 2: Add `projectId` to task model and create API

**Files:**
- Modify: `crates/core/src/task/model.rs`
- Modify: `crates/api-server/src/routes/task.rs`
- Modify: `packages/client/src/hooks/useTaskApi.ts`

**Step 1: Add failing expectations**

Add/extend task route tests to expect:
- create task without `projectId` => `422` (for new tasks)
- create task with invalid `projectId` => `404`

**Step 2: Verify tests fail**

Run: `cargo test --manifest-path crates/Cargo.toml -p api-server create_task`

Expected: Failure because `projectId` is not validated or persisted.

**Step 3: Implement model + API changes**

- Add field to `Task`:

```rust
pub project_id: Option<Uuid>,
```

- Add builder:

```rust
pub fn with_project_id(mut self, project_id: Uuid) -> Self {
    self.project_id = Some(project_id);
    self
}
```

- Update `CreateTaskRequest` and `TaskResponse` in `routes/task.rs` with `project_id` (`camelCase` as `projectId`).
- In `create_task` handler:
  - Require `project_id` for new tasks (`422 ProjectRequired` style message).
  - Validate project exists in `state.project_store()` (`404 ProjectNotFound`).
  - Persist into task.
- Update client request/response types in `useTaskApi.ts`:
  - `Task.projectId: string | null`
  - `CreateTaskRequest.projectId?: string`

**Step 4: Verify pass**

Run:
- `cargo test --manifest-path crates/Cargo.toml -p api-server`
- `pnpm --filter client run test`

Expected: Tests pass with new field compatibility.

**Step 5: Commit**

```bash
git add crates/core/src/task/model.rs crates/api-server/src/routes/task.rs packages/client/src/hooks/useTaskApi.ts
git commit -m "feat(task): require project binding for new task creation"
```

---

### Task 3: Enforce project-bound execution resolution

**Files:**
- Modify: `crates/api-server/src/routes/executor.rs`
- Modify: `crates/api-server/src/gateway/manager.rs`
- Modify: `crates/api-server/src/gateway/protocol.rs` (if error details/types need extension)

**Step 1: Add failing backend tests**

Add tests for execute route:
- task with no `projectId` => `422`
- task with unknown project => `404`
- project host offline => `409`
- gateway dispatch receives `cwd == project.local_path`

**Step 2: Verify tests fail**

Run: `cargo test --manifest-path crates/Cargo.toml -p api-server start_execution`

Expected: Failures because current path uses host default cwd and weak host selection.

**Step 3: Implement execution constraints**

- In `start_execution`:
  - Load task -> require `task.project_id`.
  - Load project via `state.project_store().get(project_id)`.
  - Resolve base branch via `task.base_branch.unwrap_or(project.default_branch)`.
- Replace generic dispatch with host-targeted dispatch:

```rust
gateway_manager.dispatch_task_to_host(&project.gateway_id.to_string(), gateway_task)
```

- Fill gateway task cwd from project:

```rust
cwd: project.local_path.clone()
```

- Keep model passthrough, but ignore client-provided targetHost for project-bound tasks.

- In `GatewayManager`, add:

```rust
pub async fn dispatch_task_to_host(&self, host_id: &str, task: GatewayTaskRequest) -> Result<String, String>
```

This function should:
- find exact host id
- validate availability for agent type
- enqueue task
- return meaningful errors (`host not found`, `host offline/busy`).

**Step 4: Verify pass**

Run: `cargo test --manifest-path crates/Cargo.toml -p api-server`

Expected: Execution path and host routing tests pass.

**Step 5: Commit**

```bash
git add crates/api-server/src/routes/executor.rs crates/api-server/src/gateway/manager.rs crates/api-server/src/gateway/protocol.rs
git commit -m "feat(executor): resolve cwd from project and enforce host binding"
```

---

### Task 4: Frontend require project selection in task creation

**Files:**
- Modify: `packages/client/src/components/task/CreateTaskModal.tsx`
- Modify: `packages/client/src/App.tsx`
- Optional Modify: `packages/client/src/hooks/useTaskExecutor.ts`

**Step 1: Add/adjust UI tests**

Add or update modal tests:
- create disabled when no project selected
- payload contains `projectId`
- create-and-start path sends `projectId`

**Step 2: Verify tests fail**

Run: `pnpm --filter client run test`

Expected: Failures before modal gains project selector/validation.

**Step 3: Implement UI flow**

- In `CreateTaskModal`:
  - load projects via `useProjects()`
  - add required project selector
  - include `projectId` in `CreateTaskRequest`
  - prevent submit if project not selected
- In `App.tsx`:
  - keep execute request slim (agent/model/baseBranch)
  - rely on backend project binding for target host + cwd

**Step 4: Verify pass**

Run:
- `pnpm --filter client run test`
- `pnpm -r run build`

Expected: UI tests and build pass.

**Step 5: Commit**

```bash
git add packages/client/src/components/task/CreateTaskModal.tsx packages/client/src/App.tsx packages/client/src/hooks/useTaskExecutor.ts
git commit -m "feat(client): require project selection when creating tasks"
```

---

### Task 5: Optional gateway path allowlist hardening

**Files:**
- Modify: `services/agent-gateway/src/index.ts`
- Modify: `services/agent-gateway/src/executor.ts`
- Add/Modify tests under `services/agent-gateway/src/*.test.ts`

**Step 1: Add failing tests**

Add tests to reject task execution when `task.cwd` is outside allowlist.

**Step 2: Verify tests fail**

Run: `pnpm --filter @vk/agent-gateway run test`

Expected: Failures before allowlist guard.

**Step 3: Implement guard**

- Add env-driven allowlist (e.g. `GATEWAY_ALLOWED_PROJECT_ROOTS` comma-separated).
- Reject `task:execute` if cwd is not under allowed roots.
- Emit `task:failed` with explicit error code/message.

**Step 4: Verify pass**

Run: `pnpm --filter @vk/agent-gateway run test`

Expected: Guard tests pass.

**Step 5: Commit**

```bash
git add services/agent-gateway/src/index.ts services/agent-gateway/src/executor.ts services/agent-gateway/src/*.test.ts
git commit -m "feat(gateway): enforce project cwd allowlist"
```

---

### Task 6: Update feature docs and README snippets

**Files:**
- Modify: `docs/features/task-commands.md`
- Modify: `docs/features/service-agent-gateway.md`
- Modify: `docs/features/index.md` (if status/date changes)
- Optional: `README.md`

**Step 1: Document new behavior**

- Task creation now requires `projectId`.
- Execution cwd resolved server-side from project binding.
- Host routing enforced by project-gateway binding.

**Step 2: Verify docs index integrity**

Run: `pnpm run check:docs:features`

Expected: pass.

**Step 3: Commit**

```bash
git add docs/features/*.md README.md
git commit -m "docs: describe project-bound task execution flow"
```

---

## Final Verification Checklist

Run in worktree root:

```bash
pnpm install
pnpm run check:docs:features
pnpm run test:scripts
pnpm -r run build
pnpm -r run test
cargo test --manifest-path crates/Cargo.toml
```

Expected: all commands succeed.
