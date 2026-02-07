# 服务：agent-gateway

## 概述
远程执行网关服务，通过 WebSocket 连接中心服务端，并使用 OpenCode 执行任务。

## 入口
- UI：无
- API：通过 `${GATEWAY_SERVER_URL}/agent/ws?hostId=...` 建立 WebSocket，需携带 `Authorization: Bearer $GATEWAY_AUTH_TOKEN`
- CLI：`pnpm --dir services/agent-gateway dev`（或在目录内执行 `pnpm dev`）
- Root 脚本：`pnpm run dev:gateway:local`（本地） / `pnpm run dev:gateway:cloud`（云端，读取 `.env.gateway`）

## 行为与边界
- 注册主机能力并处理 `registered` / `ping` 消息。
- 监听 `task:*` 与 `models:request` 指令。
- 通过 OpenCode SDK 执行任务并回传流式事件。
- 总是启动内嵌 OpenCode 服务（端口可由 `OPENCODE_PORT` 配置）。
- 支持通过 `GATEWAY_ALLOWED_PROJECT_ROOTS`（逗号分隔）限制可执行任务的 `cwd` 根路径。
- 当任务 `cwd` 不在允许列表内时，网关会拒绝执行并回传 `task:failed`（`code=CWD_NOT_ALLOWED`）。
- 若未显式配置，网关默认连接 `ws://127.0.0.1:8081`，默认 token 为 `dev-token`（本地开发友好）。
- API 端会校验 Bearer token（`GATEWAY_AUTH_TOKEN`，默认 `dev-token`），不匹配时拒绝 WebSocket 连接。

## 数据与存储影响
- 在配置的工作目录（cwd）写入执行产物。

## 权限与风险
- 会在网关主机执行命令并修改文件。
- 依赖可用的 OpenCode CLI/SDK 以及到服务端的网络连通性。

## 可观测性
- 发出网关任务事件（`task:started`、`task:event`、`task:completed`、`task:failed`）。

## 测试与验证
- 执行 `pnpm --dir services/agent-gateway test`（或在目录内执行 `pnpm test`）。
- 执行 `pnpm run test:scripts`（包含 `scripts/run-gateway.mjs` 相关配置测试）。

## 相关变更
- 协议定义见 `crates/api-server/src/gateway/protocol.rs`。
