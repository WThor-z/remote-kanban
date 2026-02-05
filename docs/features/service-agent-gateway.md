# Service: agent-gateway

## Summary
Remote gateway service that connects to the server via WebSocket and executes tasks using OpenCode.

## Entry Points
- UI: N/A
- API: WebSocket to `${GATEWAY_SERVER_URL}/agent/ws?hostId=...` with `Authorization: Bearer $GATEWAY_AUTH_TOKEN`
- CLI: `pnpm --filter @vk/agent-gateway dev`

## Behavior and Boundaries
- Registers host capabilities and handles `registered`/`ping` messages.
- Listens for `task:*` commands and `models:request`.
- Executes tasks via the OpenCode SDK and streams events back to the server.
- Always starts an embedded OpenCode server (port configurable via `OPENCODE_PORT`).

## Data and Storage Impact
- Writes work artifacts in the configured working directory (cwd).

## Permissions and Risks
- Executes commands and file changes on the gateway host.
- Requires valid OpenCode CLI/SDK installation and network access to the server.

## Observability
- Emits gateway task events (`task:started`, `task:event`, `task:completed`, `task:failed`).

## Test and Verification
- Run `pnpm --filter @vk/agent-gateway test`.

## Related Changes
- Protocol types in `crates/api-server/src/gateway/protocol.rs`.
