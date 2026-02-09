# 服务：Agent Memory

## 概述
`service/agent-memory` 为 Agent 执行链路提供跨任务记忆能力，覆盖“任务前注入上下文 + 任务后自动沉淀记忆”。
该能力支持项目级与主机级隔离，并提供双存储（网关本地 + Rust 中央）与管理 API。

## 入口
- UI：`Memory` 全局页面（CRUD、搜索、过滤、启停开关）
- API：`/api/memory/settings`、`/api/memory/items` 等 REST 路由
- CLI：无

## 行为与边界
- 任务执行前：根据 `hostId + projectId` 检索记忆并注入 Prompt。
- 任务执行后：优先规则抽取，候选不足时使用 LLM 兜底抽取并写入。
- 注入顺序：`pinned project` -> `project` -> `host preference`。
- 一期检索仅使用 SQLite FTS5/BM25，不包含向量检索。
- Memory 组件异常时降级，不阻断任务主流程。

## 数据与存储影响
- 网关本地存储：
  - SQLite：`<MEMORY_DATA_DIR>/memory.sqlite`
  - Markdown 镜像：
    - 项目范围：`<project>/.opencode/memory/MEMORY.md`
    - 每日记录：`<project>/.opencode/memory/daily/YYYY-MM-DD.md`
    - 主机范围：`<MEMORY_DATA_DIR>/.opencode/memory/global/...`
- Rust 中央存储（镜像与管理层）：
  - `.vk-data/memory/settings.json`
  - `.vk-data/memory/items.json`

## 网关协议扩展
- `memory:request`（Server -> Gateway）：`{ requestId, action, payload }`
- `memory:response`（Gateway -> Server）：`{ requestId, ok, data?, error? }`
- `memory:sync`（Gateway -> Server）：`{ hostId, projectId?, op, items[] }`

## 环境变量
- `MEMORY_ENABLE`
- `MEMORY_GATEWAY_STORE_ENABLE`
- `MEMORY_RUST_STORE_ENABLE`
- `MEMORY_AUTO_WRITE_ENABLE`
- `MEMORY_PROMPT_INJECTION_ENABLE`
- `MEMORY_INJECTION_TOKEN_BUDGET`
- `MEMORY_RETRIEVAL_TOP_K`
- `MEMORY_LLM_EXTRACT_ENABLE`
- `MEMORY_DATA_DIR`

## 可观测性
- 任务阶段会产生日志事件（注入、抽取、写入、同步结果）。
- 失败会记录 `task:event(type=log|error)`，并标记为降级路径。

## 测试与验证
- Gateway 单测：配置解析、检索排序、预算截断、抽取分支、故障降级。
- Rust 单测：Settings、CRUD、代理模式、双写模式、全关模式。
- 前端测试：`useMemoryApi`、页面 CRUD、过滤与开关交互。

## 相关文档
- `docs/features/service-agent-memory.md`
- `docs/features/index.md`
- `docs/plans/2026-02-06-project-bound-task-execution-implementation.md`
