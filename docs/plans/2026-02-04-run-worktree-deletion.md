# Run and Worktree Deletion API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add REST endpoints to permanently delete runs (single and all-for-task) and document the existing worktree cleanup path so storage can be reclaimed on demand.

**Architecture:** Extend `crates/api-server/src/routes/task.rs` with two DELETE handlers that call `agent_runner::RunStore` via `state.executor()`. Use `ExecutionStatus::is_active()` from `agent_runner` to reject deletion of active runs (409). Keep worktree cleanup via existing `POST /api/tasks/{id}/cleanup` (no new path because `DELETE /api/tasks/{id}/worktree` already exists in executor routes).

**Tech Stack:** Rust, axum, agent-runner (RunStore/Run/ExecutionStatus), vk-core Task store, tokio tests, tower::ServiceExt.

---

### Task 1: Add single-run delete endpoint (test-first)

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`

**Step 1: Write the failing test**

Add a `#[cfg(test)]` module near the end of `crates/api-server/src/routes/task.rs` with helpers and the first test.

```rust
#[cfg(test)]
mod run_delete_tests {
    use super::*;
    use axum::body::Body;
    use http::Request;
    use tempfile::TempDir;
    use tower::ServiceExt;
    use std::sync::Arc;

    use agent_runner::{Run, ExecutionStatus};
    use agent_runner::process::AgentType;
    use vk_core::kanban::KanbanStore;
    use vk_core::task::FileTaskStore;
    use crate::gateway::GatewayManager;

    async fn setup_state() -> (AppState, TempDir) {
        let temp = TempDir::new().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let task_store = Arc::new(FileTaskStore::new(data_dir.join("tasks.json")).await.unwrap());
        let kanban_store = Arc::new(KanbanStore::with_task_store(data_dir.join("kanban.json"), Arc::clone(&task_store)).await.unwrap());
        let gateway_manager = Arc::new(GatewayManager::new());
        let state = AppState::with_stores(data_dir, task_store, kanban_store, gateway_manager).await.unwrap();
        (state, temp)
    }

    fn build_run(task_id: Uuid, status: ExecutionStatus) -> Run {
        let mut run = Run::new(task_id, AgentType::OpenCode, "test prompt".to_string(), "main".to_string());
        run.status = status;
        run
    }

    #[tokio::test]
    async fn delete_run_returns_no_content() {
        let (state, _temp) = setup_state().await;
        let task = Task::new("delete run");
        state.task_store().create(task.clone()).await.unwrap();

        let run = build_run(task.id, ExecutionStatus::Completed);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app.oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tasks/{}/runs/{}", task.id, run.id))
                .body(Body::empty())
                .unwrap(),
        ).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.executor().list_runs(task.id).unwrap().is_empty());
    }
}
```

**Step 2: Run test to verify it fails**

Run (from `crates/`): `cargo test -p api-server delete_run_returns_no_content -- --nocapture`

Expected: FAIL because the DELETE handler and route are not implemented.

**Step 3: Write minimal implementation**

Add the handler and route in `crates/api-server/src/routes/task.rs`.

```rust
async fn delete_run(
    State(state): State<AppState>,
    Path((task_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let task = state.task_store().get(task_id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    if task.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Task {} not found", task_id) })));
    }

    let runs = state.executor().list_runs(task_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    let run = runs.into_iter().find(|r| r.id == run_id).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Run {} not found", run_id) }))
    })?;

    if run.status.is_active() {
        return Err((StatusCode::CONFLICT, Json(ErrorResponse { error: "Run is active".to_string() })));
    }

    state.executor().run_store().delete_run(task_id, run_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    Ok(StatusCode::NO_CONTENT)
}
```

Add the route:

```rust
.route("/api/tasks/{id}/runs/{run_id}", delete(delete_run))
```

And update imports to include `axum::routing::delete`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p api-server delete_run_returns_no_content -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/api-server/src/routes/task.rs
git commit -m "feat(api): add delete-run endpoint"
```

---

### Task 2: Add delete-all-runs endpoint (test-first)

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`

**Step 1: Write failing tests**

Add two tests into the same `run_delete_tests` module:

```rust
    #[tokio::test]
    async fn delete_task_runs_removes_all() {
        let (state, _temp) = setup_state().await;
        let task = Task::new("delete task runs");
        state.task_store().create(task.clone()).await.unwrap();

        let run_a = build_run(task.id, ExecutionStatus::Completed);
        let run_b = build_run(task.id, ExecutionStatus::Completed);
        state.executor().run_store().save_run(&run_a).unwrap();
        state.executor().run_store().save_run(&run_b).unwrap();

        let app = router().with_state(state.clone());
        let response = app.oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tasks/{}/runs", task.id))
                .body(Body::empty())
                .unwrap(),
        ).await.unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.executor().list_runs(task.id).unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_task_runs_rejects_active_runs() {
        let (state, _temp) = setup_state().await;
        let task = Task::new("active run");
        state.task_store().create(task.clone()).await.unwrap();

        let run = build_run(task.id, ExecutionStatus::Running);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app.oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tasks/{}/runs", task.id))
                .body(Body::empty())
                .unwrap(),
        ).await.unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`

Expected: FAIL because delete-all endpoint does not exist.

**Step 3: Write minimal implementation**

Add the handler in `crates/api-server/src/routes/task.rs`:

```rust
async fn delete_task_runs(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let task = state.task_store().get(task_id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    if task.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Task {} not found", task_id) })));
    }

    let runs = state.executor().list_runs(task_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    if runs.iter().any(|run| run.status.is_active()) {
        return Err((StatusCode::CONFLICT, Json(ErrorResponse { error: "Run is active".to_string() })));
    }

    state.executor().run_store().delete_task_runs(task_id).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))
    })?;

    Ok(StatusCode::NO_CONTENT)
}
```

Attach to existing route:

```rust
.route("/api/tasks/{id}/runs", get(list_task_runs).delete(delete_task_runs))
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/api-server/src/routes/task.rs
git commit -m "feat(api): add delete-all-runs endpoint"
```

---

### Task 3: Document cleanup paths

**Files:**
- Modify: `README.md`

**Step 1: Update documentation**

Add a short REST API note under API Reference (or a new subsection) mentioning:

- `DELETE /api/tasks/{id}/runs/{run_id}`
- `DELETE /api/tasks/{id}/runs`
- `POST /api/tasks/{id}/cleanup` (worktree cleanup via Gateway)

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: document run deletion endpoints"
```

---

## Notes / Constraints

- Frontend dependencies could not be installed in this environment because `@opencode-vibe/protocol` is not resolvable from the public npm registry. JS tests are skipped until the registry/auth issue is resolved.
- Rust tests run via `cargo test` should remain the verification baseline for this change.
