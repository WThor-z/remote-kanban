# Rust 模块：api-server

## 概述
`api-server` 是 Rust 后端入口，提供任务、运行记录与网关协同所需的 REST API 和 Socket.IO 事件通道。

## 入口
- UI：无
- API：REST 服务端口 `8081`；Socket.IO 服务端口 `8080`
- CLI：在 `crates/` 下执行 `cargo run -p api-server`

## 行为与边界
- 提供任务、运行记录、执行控制、网关管理、健康检查等 REST 路由。
- 承载前端使用的看板/任务 Socket.IO 事件通道。
- 执行能力委托给 `agent-runner`，远程主机协同由 `gateway` 管理器处理。

## 数据与存储影响
- 使用 `VK_DATA_DIR`（默认 `.vk-data`）存放 `tasks.json`、`kanban.json`、`runs/`、`worktrees/`。
- worktree 创建在 `data_dir/worktrees` 下，分支前缀默认 `task/`。

## 权限与风险
- 需要对数据目录写入权限，并会在仓库路径执行 Git worktree 操作。

## 可观测性
- 通过 Socket.IO 发出 `kanban:*`、`task:*` 事件，并使用 `tracing` 记录日志。

## 测试与验证
- 执行 `cargo test -p api-server`。

## 相关变更
- 路由位于 `crates/api-server/src/routes/*`。
