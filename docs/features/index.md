# 功能目录

## 用户功能

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| 任务命令 | 通过斜杠命令快速创建与管理任务 | 稳定 | 待定 | 2026-02-06 | [文档](task-commands.md) |
| 工作区与项目作用域 | 通过 workspace-first 入口与 workspace/project 绑定约束任务可见范围与执行上下文 | 活跃 | 待定 | 2026-02-10 | [文档](workspace-project-scope.md) |
| 看板拖拽与实时同步 | 支持三列看板拖拽编排，并通过 Socket.IO 向所有客户端实时广播状态 | 活跃 | 待定 | 2026-02-09 | [文档](kanban-realtime-sync.md) |
| AI 任务执行控制 | 提供任务启动、停止、输入、状态查询与 worktree 清理能力 | 活跃 | 待定 | 2026-02-09 | [文档](task-execution-control.md) |
| 任务运行历史与事件回放 | 支持 runs 列表、事件筛选分页和历史消息回放 | 活跃 | 待定 | 2026-02-09 | [文档](task-run-history.md) |

## 模块目录

### Rust 模块（Crates）

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| crate/vk-core | Rust 后端核心模型与存储（含 workspace/project/task 绑定） | 活跃 | 待定 | 2026-02-09 | [文档](crate-vk-core.md) |
| crate/api-server | 提供 workspace/task/run/executor 的 Rust REST + Socket.IO 后端 | 活跃 | 待定 | 2026-02-10 | [文档](crate-api-server.md) |
| crate/agent-runner | 负责任务执行、worktree 隔离与带上下文运行记录持久化 | 活跃 | 待定 | 2026-02-09 | [文档](crate-agent-runner.md) |
| crate/git-worktree | 提供任务分支与隔离工作区的 Git worktree 管理能力 | 活跃 | 待定 | 2026-02-06 | [文档](crate-git-worktree.md) |

### Node/TS 包（Packages）

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| package/protocol | 共享协议类型与解析工具（看板事件与 Agent 输出） | 稳定 | 待定 | 2026-02-06 | [文档](package-protocol.md) |
| package/server | Node Socket.IO 服务端，负责看板与任务会话管理 | 活跃 | 待定 | 2026-02-06 | [文档](package-server.md) |
| package/client | React 前端界面，提供 workspace-first 入口与作用域任务看板执行控制 | 活跃 | 待定 | 2026-02-10 | [文档](package-client.md) |
| package/pty-manager | PTY 进程管理封装（已弃用，保留兼容） | 弃用 | 待定 | 2026-02-06 | [文档](package-pty-manager.md) |

### 服务（Services）

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| service/agent-gateway | 远程执行网关，负责接收任务并回传执行事件 | 活跃 | 待定 | 2026-02-06 | [文档](service-agent-gateway.md) |
| service/agent-memory | Agent Memory 注入、抽取、双存储与管理 API | 活跃 | 待定 | 2026-02-09 | [文档](service-agent-memory.md) |

## 开发工具与治理

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| dev/ci-quality-gates | 合并请求/推送自动执行安装、构建、测试与文档校验 | 活跃 | 待定 | 2026-02-06 | [文档](dev-ci-quality-gates.md) |
| dev/runtime-cleanup | 一键清理运行时数据目录（支持 dry-run / apply） | 活跃 | 待定 | 2026-02-06 | [文档](dev-runtime-cleanup.md) |
