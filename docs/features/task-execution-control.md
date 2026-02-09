# AI 任务执行控制

## 概述
该功能支持从任务卡片直接发起 AI 执行，并提供执行状态查询、运行中输入、停止执行与 worktree 清理能力，形成完整的任务执行闭环。

## 入口
- UI：`packages/client/src/components/task/CreateTaskModal.tsx`、`packages/client/src/components/task/TaskDetailPanel.tsx`
- API：`POST /api/tasks/{id}/execute`、`GET /api/tasks/{id}/status`、`POST /api/tasks/{id}/stop`、`POST /api/tasks/{id}/input`、`DELETE /api/tasks/{id}/worktree`
- CLI：无

## 行为与边界
- 支持创建任务后立即执行，执行参数包括 `agentType`、`baseBranch`、`model`。
- 执行时由后端按任务绑定项目自动确定 `cwd` 和目标网关主机，不接受客户端直接指定执行目录。
- 任务必须绑定项目；未绑定返回 `422`，项目不存在返回 `404`，项目主机离线返回 `409`。
- 对话面板可在运行中发送输入，中止后可执行 worktree 清理。

## 数据与存储影响
- 每次执行会生成 run 记录，并保存状态、事件计数与上下文元数据（project/workspace）。
- 任务执行可能在仓库下创建隔离 worktree，并写入执行产物。

## 权限与风险
- 网关会在目标主机执行命令并修改代码，属于高权限操作。
- 需要 `agent-gateway` 在线且认证通过。

## 可观测性
- 前端通过 `task:status`、`task:message`、`task:execution_event` 展示执行进度与输出。
- 后端记录网关分发、运行状态变更与失败原因。

## 测试与验证
- 执行 `pnpm --filter client test` 验证任务详情面板执行控制链路。
- 执行 `cargo test -p api-server` 验证执行路由的绑定校验与错误码。

## 相关变更
- `packages/client/src/hooks/useTaskExecutor.ts`
- `packages/client/src/components/task/CreateTaskModal.tsx`
- `packages/client/src/components/task/TaskDetailPanel.tsx`
- `crates/api-server/src/routes/executor.rs`
- `services/agent-gateway/src/executor.ts`
