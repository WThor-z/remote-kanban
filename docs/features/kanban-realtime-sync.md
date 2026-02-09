# 看板拖拽与实时同步

## 概述
该功能提供三列看板的可视化任务管理，并通过 Socket.IO 将任务创建、移动、删除等变更实时同步到所有在线客户端。

## 入口
- UI：`packages/client/src/components/kanban/KanbanBoard.tsx`、`packages/client/src/components/kanban/KanbanColumn.tsx`
- API：Socket 事件 `kanban:request-sync`、`kanban:create`、`kanban:move`、`kanban:delete`、`kanban:sync`
- CLI：无

## 行为与边界
- 看板固定为 `To Do / Doing / Done` 三列，支持任务卡片跨列拖拽与列内排序。
- 客户端连接后会主动请求一次 `kanban:request-sync`，服务端返回当前完整看板状态。
- 任一客户端触发创建/移动/删除后，服务端广播 `kanban:sync`，所有客户端视图一致。
- 当网络连接中断时，实时同步会暂停，重连后需重新请求同步。

## 数据与存储影响
- 看板状态持久化在 `.vk-data/kanban.json`（Rust 模式）或 `.opencode/kanban.json`（Node 模式）。

## 权限与风险
- 依赖 WebSocket 连接可用。
- 并发操作以服务端最新状态为准，客户端应以 `kanban:sync` 结果覆盖本地视图。

## 可观测性
- 客户端监听 `kanban:error` 展示操作失败信息。
- 服务端通过 Socket 事件流可追踪看板状态变更传播。

## 测试与验证
- 执行 `pnpm --filter client test` 验证看板拖拽和列内排序行为。
- 启动两个客户端窗口，执行拖拽/删除后确认另一端实时刷新。

## 相关变更
- `packages/client/src/hooks/useKanban.ts`
- `packages/client/src/components/kanban/KanbanBoard.tsx`
- `packages/client/src/components/kanban/KanbanColumn.tsx`
- `crates/api-server/src/socket.rs`
