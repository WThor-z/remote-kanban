# Package: server

## Summary
Node.js Socket.IO server that manages kanban state and task-agent sessions.

## Entry Points
- UI: N/A
- API: Socket.IO server started by `packages/server/src/index.ts`
- CLI: `pnpm --filter @opencode-vibe/server dev`

## Behavior and Boundaries
- Handles `kanban:*` events for task creation, movement, and deletion.
- Manages agent sessions and task execution via `TaskSessionManager`.
- Designed for local development with Socket.IO clients.

## Data and Storage Impact
- Persists kanban state to `.opencode/kanban.json` and task session history under `.opencode/tasks/` (per task JSON files, relative to the server working directory).

## Permissions and Risks
- Writes to `.opencode/` in the working directory.

## Observability
- Emits Socket.IO events (`kanban:*`, `task:*`, `agent:*`).

## Test and Verification
- Run `pnpm --filter @opencode-vibe/server test`.

## Related Changes
- Depends on `@opencode-vibe/protocol`.
