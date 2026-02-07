# Task Runs Delete Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a DELETE endpoint that removes all runs for a task, returning 409 if any run is active.

**Architecture:** Extend the task routes with a new handler that validates task existence, checks run status, and deletes via RunStore, plus route wiring. Add two tests to cover success and active-run rejection.

**Tech Stack:** Rust, axum, tokio, api-server crate tests

---

### Task 1: Add failing test for removing all runs

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`
- Test: `crates/api-server/src/routes/task.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn delete_task_runs_removes_all() {
    let (state, _temp_dir) = build_state().await;
    let task = state
        .task_store()
        .create(Task::new("Delete task runs".to_string()))
        .await
        .unwrap();

    let mut run_one = Run::new(
        task.id,
        AgentType::OpenCode,
        "Test prompt one".to_string(),
        "main".to_string(),
    );
    run_one.update_status(ExecutionStatus::Completed);
    state.executor().run_store().save_run(&run_one).unwrap();

    let mut run_two = Run::new(
        task.id,
        AgentType::OpenCode,
        "Test prompt two".to_string(),
        "main".to_string(),
    );
    run_two.update_status(ExecutionStatus::Completed);
    state.executor().run_store().save_run(&run_two).unwrap();

    let app = router().with_state(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tasks/{}/runs", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(state.executor().list_runs(task.id).unwrap().is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`
Expected: FAIL because route/handler does not exist.

**Step 3: Write minimal implementation**

Skip until Task 3.

**Step 4: Run test to verify it passes**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`
Expected: PASS after handler/route implemented.

**Step 5: Commit**

Skip (user requested no commits). If needed later:

```bash
git add crates/api-server/src/routes/task.rs
git commit -m "feat: delete all task runs"
```

### Task 2: Add failing test for active run rejection

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`
- Test: `crates/api-server/src/routes/task.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn delete_task_runs_rejects_active_runs() {
    let (state, _temp_dir) = build_state().await;
    let task = state
        .task_store()
        .create(Task::new("Delete task runs active".to_string()))
        .await
        .unwrap();

    let mut run = Run::new(
        task.id,
        AgentType::OpenCode,
        "Test prompt".to_string(),
        "main".to_string(),
    );
    run.update_status(ExecutionStatus::Running);
    state.executor().run_store().save_run(&run).unwrap();

    let app = router().with_state(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tasks/{}/runs", task.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`
Expected: FAIL because handler/route does not exist.

**Step 3: Write minimal implementation**

Skip until Task 3.

**Step 4: Run test to verify it passes**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`
Expected: PASS after handler/route implemented.

**Step 5: Commit**

Skip (user requested no commits). If needed later:

```bash
git add crates/api-server/src/routes/task.rs
git commit -m "feat: block deleting task runs when active"
```

### Task 3: Implement delete_task_runs handler and route

**Files:**
- Modify: `crates/api-server/src/routes/task.rs`
- Test: `crates/api-server/src/routes/task.rs`

**Step 1: Write the failing test**

Already covered in Tasks 1-2.

**Step 2: Run test to verify it fails**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`
Expected: FAIL (handler/route missing).

**Step 3: Write minimal implementation**

```rust
/// DELETE /api/tasks/:id/runs - Delete all runs for a task
async fn delete_task_runs(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let task = state.task_store().get(task_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )
    })?;

    if task.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", task_id),
            }),
        ));
    }

    let runs = state.executor().list_runs(task_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )
    })?;

    if runs.iter().any(|run| run.status.is_active()) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Run is active".to_string(),
            }),
        ));
    }

    state
        .executor()
        .run_store()
        .delete_task_runs(task_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: e.to_string() }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}
```

Add to router:

```rust
.route("/api/tasks/{id}/runs", get(list_task_runs).delete(delete_task_runs))
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p api-server delete_task_runs_ -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Skip (user requested no commits). If needed later:

```bash
git add crates/api-server/src/routes/task.rs
git commit -m "feat: add delete task runs endpoint"
```
