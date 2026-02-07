# Rust 模块：vk-core

## 概述
`vk-core` 是 Rust 后端的核心领域层，提供任务、看板、项目、运行记录等模型，以及基于文件的存储实现。

## 入口
- UI：无
- API：被 `api-server` 与 `agent-runner` 引入的 Rust 模块 `vk-core`
- CLI：无

## 行为与边界
- 定义 Task / Kanban / Project / Run 的数据模型与辅助能力。
- 提供基于 JSON 文件的任务、看板、项目、运行记录存储。
- 不直接暴露 HTTP / WebSocket 接口，也不负责执行任务。

## 数据与存储影响
- 在调用方配置的数据目录中持久化 `tasks.json` 与 `kanban.json`（常见目录为 `.vk-data/`）。
- 在同一数据目录下持久化 `runs/<task_id>/<run_id>/`（`run.json`、`events.jsonl`、`messages.jsonl`）。
- 项目数据持久化到调用方指定的 `projects.json`。

## 权限与风险
- 需要对配置的数据目录有文件读写权限。

## 可观测性
- 运行记录存储相关路径使用 `tracing` 输出日志。

## 测试与验证
- 执行 `cargo test -p vk-core`。

## 相关变更
- 被 `crates/api-server` 与 `crates/agent-runner` 复用。
