# Rust 模块：git-worktree

## 概述
`git-worktree` 提供任务分支与隔离工作目录管理能力，用于支撑任务级并行开发。

## 入口
- UI：无
- API：被 `agent-runner` 与 `api-server` 调用的 Rust 模块 `git-worktree`
- CLI：无

## 行为与边界
- 提供 Git worktree 的创建、列表、删除，以及任务分支相关操作。
- 要求目标仓库路径是有效的 Git 仓库。

## 数据与存储影响
- 在配置目录创建 worktree（默认仓库根目录下 `.worktrees`）。
- 使用分支前缀（默认 `task/`）。

## 权限与风险
- 会在仓库路径执行 Git 命令。
- 任务执行过程中可能创建或删除 worktree / 分支。

## 可观测性
- worktree 相关操作通过 `tracing` 输出日志。

## 测试与验证
- 执行 `cargo test -p git-worktree`。

## 相关变更
- 被 `crates/agent-runner` 复用。
