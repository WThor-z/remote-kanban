# Rust 模块：vk-core

## 概述
`vk-core` 是 Rust 后端的核心领域层，承载任务、看板、项目、工作区（workspace）与运行记录模型，并提供基于文件的存储实现。

## 入口
- UI：无
- API：被 `api-server` 与 `agent-runner` 引入的 Rust 模块 `vk-core`
- CLI：无

## 行为与边界
- 定义 Task / Kanban / Project / Workspace / Run 的领域模型与转换逻辑。
- 维护工作区与项目、任务绑定关系：
  - `Project` 强制携带 `workspace_id`
  - `Task` 支持 `project_id` + `workspace_id` 绑定与一致性辅助方法
- 提供文件存储与迁移能力（包含 legacy project 数据补齐 workspace 绑定）。
- 不直接暴露 HTTP / WebSocket 接口，也不负责执行任务。

## 数据与存储影响
- 在数据目录中持久化 `tasks.json`、`kanban.json`、`projects.json`、`workspaces.json`（常见目录为 `.vk-data/`）。
- 运行记录目录持久化在 `runs/<task_id>/<run_id>/`（`run.json`、`events.jsonl`、`messages.jsonl`）。

## 权限与风险
- 需要对配置的数据目录有文件读写权限。
- 工作区 slug 归一化与唯一性约束会影响创建/更新行为（重复 slug 会被拒绝）。

## 可观测性
- 关键存储路径与持久化失败场景使用 `tracing` 输出日志。

## 测试与验证
- 执行 `cargo test -p vk-core`。

## 相关变更
- 被 `crates/api-server` 的 workspace/project/task 路由与 `crates/agent-runner` 运行记录层复用。
