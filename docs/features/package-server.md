# Node 包：server

## 概述
Node.js Socket.IO 服务端，负责看板状态同步与任务会话管理。

## 入口
- UI：无
- API：由 `packages/server/src/index.ts` 启动的 Socket.IO 服务
- CLI：`pnpm --filter @opencode-vibe/server dev`

## 行为与边界
- 处理 `kanban:*` 事件（创建、移动、删除任务）。
- 通过 `TaskSessionManager` 管理 Agent 会话与任务执行。
- 主要用于本地开发场景下的 Socket.IO 客户端协同。

## 数据与存储影响
- 将看板状态持久化到 `.opencode/kanban.json`，并将任务会话历史保存到 `.opencode/tasks/`（按任务 JSON 文件存储，相对服务工作目录）。

## 权限与风险
- 需要对当前工作目录下 `.opencode/` 有写权限。

## 可观测性
- 发出 Socket.IO 事件（`kanban:*`、`task:*`、`agent:*`）。

## 测试与验证
- 执行 `pnpm --filter @opencode-vibe/server test`。

## 相关变更
- 依赖 `@opencode-vibe/protocol`。
