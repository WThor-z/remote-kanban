# 执行编排 API v1 (`/api/v1/executions`)

## 目标
- 提供以 `execution` 为中心的编排接口，而不是仅依赖 `task` 子路由。
- 保持旧接口 `/api/tasks/*` 可用（兼容层保留）。
- 输出可追踪事件字段：`executionId`、`orgId`、`traceId`、`seq`、`ts`。

## 新增接口
- `POST /api/v1/executions`
- `GET /api/v1/executions/{id}`
- `POST /api/v1/executions/{id}/input`
- `POST /api/v1/executions/{id}/stop`
- `GET /api/v1/executions/{id}/events`

## 请求示例
```bash
curl -X POST http://127.0.0.1:8081/api/v1/executions ^
  -H "Content-Type: application/json" ^
  -d "{\"taskId\":\"<task-uuid>\",\"agentType\":\"opencode\",\"traceId\":\"trace-e3-001\",\"orgId\":\"org-default\"}"
```

## 事件响应示例（节选）
```json
{
  "events": [
    {
      "executionId": "1f0b5b2e-....",
      "orgId": "org-default",
      "traceId": "trace-e3-001",
      "seq": 1,
      "ts": 1760000000000,
      "taskId": "d6de....",
      "hostId": "host-dev",
      "payload": {
        "id": "....",
        "session_id": "....",
        "task_id": "....",
        "timestamp": "2026-02-19T...",
        "event_type": "agent_event",
        "type": "message",
        "content": "..."
      }
    }
  ],
  "hasMore": false
}
```

## 兼容与行为
- 旧路由仍可执行：`/api/tasks/{id}/execute|status|stop|input|worktree`。
- 网关执行会持久化到 run store，可通过 `execution_id` 反查。
- `stop/input` 在 execution 非运行态下采用“可重复调用”的无破坏语义（返回 accepted=false 或已停止消息）。
