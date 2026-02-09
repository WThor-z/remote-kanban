# Rust 模块：api-server

## 概述
`api-server` 是 Rust 后端入口，提供任务、工作区、项目、运行记录与网关协同所需的 REST API 和 Socket.IO 事件通道。

## 入口
- UI：无
- API：REST 服务端口 `8081`；Socket.IO 服务端口 `8080`
- CLI：在 `crates/` 下执行 `cargo run -p api-server`

## 行为与边界
- 提供 workspace/project/task/run/executor 路由与健康检查能力。
- 工作区相关能力包括：
  - `GET/POST/PATCH /api/workspaces`
  - 默认工作区启动自举（含 archived 工作区处理与 slug 冲突规避）
- 执行路径具备 workspace 一致性约束：
  - 任务与项目 workspace 不一致时返回 `409`
  - 缺失 task workspace 绑定时会自动回填并持久化
- 任务与项目列表支持按 `workspaceId`（任务还支持 `projectId`）过滤。
- 执行能力委托给 `agent-runner`，远程主机协同由 `gateway` 管理器处理。

## 数据与存储影响
- 使用 `VK_DATA_DIR`（默认 `.vk-data`）存放 `tasks.json`、`kanban.json`、`projects.json`、`workspaces.json`、`runs/`、`worktrees/`。
- worktree 创建在 `data_dir/worktrees` 下，分支前缀默认 `task/`。
- 运行摘要与任务运行列表 API 返回 `projectId`/`workspaceId` 上下文（含 legacy run 回填）。

## 权限与风险
- 需要对数据目录写入权限，并会在仓库路径执行 Git worktree 操作。
- workspace 作用域过滤与执行一致性校验会改变错误码分布（例如 `404/409/422`）。

## 可观测性
- 通过 Socket.IO 发出 `kanban:*`、`task:*` 事件，并使用 `tracing` 记录执行分发与持久化日志。

## 测试与验证
- 执行 `cargo test -p api-server`。

## 相关变更
- 路由位于 `crates/api-server/src/routes/*`，核心状态初始化位于 `crates/api-server/src/state.rs`。
