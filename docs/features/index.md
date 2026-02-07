# 功能目录

## 用户功能

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| 任务命令 | 通过斜杠命令快速创建与管理任务 | 稳定 | 待定 | 2026-02-06 | [文档](task-commands.md) |

## 模块目录

### Rust 模块（Crates）

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| crate/vk-core | Rust 后端使用的核心模型与文件存储（任务、看板、项目、运行记录） | 活跃 | 待定 | 2026-02-06 | [文档](crate-vk-core.md) |
| crate/api-server | 提供任务、运行、网关协同的 Rust REST + Socket.IO 后端 | 活跃 | 待定 | 2026-02-06 | [文档](crate-api-server.md) |
| crate/agent-runner | 负责任务执行、worktree 隔离与运行记录持久化 | 活跃 | 待定 | 2026-02-06 | [文档](crate-agent-runner.md) |
| crate/git-worktree | 提供任务分支与隔离工作区的 Git worktree 管理能力 | 活跃 | 待定 | 2026-02-06 | [文档](crate-git-worktree.md) |

### Node/TS 包（Packages）

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| package/protocol | 共享协议类型与解析工具（看板事件与 Agent 输出） | 稳定 | 待定 | 2026-02-06 | [文档](package-protocol.md) |
| package/server | Node Socket.IO 服务端，负责看板与任务会话管理 | 活跃 | 待定 | 2026-02-06 | [文档](package-server.md) |
| package/client | React 前端界面，提供任务看板与执行控制 | 活跃 | 待定 | 2026-02-06 | [文档](package-client.md) |
| package/pty-manager | PTY 进程管理封装（已弃用，保留兼容） | 弃用 | 待定 | 2026-02-06 | [文档](package-pty-manager.md) |

### 服务（Services）

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| service/agent-gateway | 远程执行网关，负责接收任务并回传执行事件 | 活跃 | 待定 | 2026-02-06 | [文档](service-agent-gateway.md) |

## 开发工具与治理

| 功能 | 摘要 | 状态 | 负责人 | 更新日期 | 文档 |
|------|------|------|--------|----------|------|
| dev/ci-quality-gates | 合并请求/推送自动执行安装、构建、测试与文档校验 | 活跃 | 待定 | 2026-02-06 | [文档](dev-ci-quality-gates.md) |
| dev/runtime-cleanup | 一键清理运行时数据目录（支持 dry-run / apply） | 活跃 | 待定 | 2026-02-06 | [文档](dev-runtime-cleanup.md) |
