# 任务运行历史与事件回放

## 概述
该功能提供任务执行历史（Runs）列表、事件时间线查询与历史消息回放，帮助定位失败原因、复盘执行过程和复用历史上下文。

## 入口
- UI：`packages/client/src/components/run/RunHistoryPanel.tsx`、`packages/client/src/components/task/TaskDetailPanel.tsx`
- API：`GET /api/tasks/{id}/runs`、`GET /api/tasks/{id}/runs/{run_id}/events`、`GET /api/tasks/{id}/runs/{run_id}/messages`
- CLI：无

## 行为与边界
- 任务详情面板的 Runs 标签页可查看历史 run 列表，并按状态、时长、事件数展示摘要。
- 进入单次 run 后可按 `eventType` 与 `agentEventType` 过滤事件，并支持分页加载更多。
- 当任务当前无活跃会话时，聊天面板会回退展示最近一次 run 的持久化消息。
- 任务或 run 不存在时返回空列表或 `404`，前端按空态处理而不阻塞页面。

## 数据与存储影响
- 运行数据持久化在 `runs/<task_id>/<run_id>/` 下，包含 `run.json`、`events.jsonl`、`messages.jsonl`。
- 删除任务运行记录会移除对应 run 目录内容。

## 权限与风险
- 运行日志可能包含命令输出与路径信息，需避免泄露敏感内容。

## 可观测性
- run 摘要包含 `status`、`startedAt/endedAt`、`durationMs`、`eventCount` 等字段。
- 事件流支持按类型过滤，便于定位特定阶段异常。

## 测试与验证
- 执行 `pnpm --filter client test` 验证 Runs 面板列表、筛选和分页展示。
- 执行 `cargo test -p api-server` 验证 runs/events/messages 路由返回。

## 相关变更
- `packages/client/src/components/run/RunHistoryPanel.tsx`
- `packages/client/src/hooks/useTaskRuns.ts`
- `packages/client/src/hooks/useRunEvents.ts`
- `crates/api-server/src/routes/task.rs`
- `crates/agent-runner/src/persistence.rs`
