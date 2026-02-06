# Rust 模块：agent-runner

## 概述
`agent-runner` 是任务执行引擎，负责创建隔离 worktree、驱动 Agent 执行，并持久化运行元数据与事件。

## 入口
- UI：无
- API：被 `api-server` 调用的 Rust 模块 `agent-runner`
- CLI：无

## 行为与边界
- 管理执行会话并产出 `ExecutionEvent` 事件流。
- 通过 `git-worktree` 创建与回收 Git worktree。
- 执行任务时通过 `AGENT_WORKER_URL` 调用 worker 服务。

## 数据与存储影响
- 将运行数据持久化到 `data_dir/runs/<task_id>/<run_id>/`（`run.json`、`events.jsonl`、`messages.jsonl`）。
- 使用 `ExecutorConfig` 配置的 `data_dir`（默认 `.vk-data`）。

## 权限与风险
- 需要访问仓库路径执行 worktree 操作。
- 需要对数据目录有写权限以保存运行产物。

## 可观测性
- 输出 `ExecutionEvent`，并通过 `tracing` 记录日志。

## 测试与验证
- 执行 `cargo test -p agent-runner`。

## 相关变更
- 由 `crates/api-server` 的执行相关路由调用。
