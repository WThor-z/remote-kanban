# Package: client

## Summary
React UI for kanban task management and AI execution control.

## Entry Points
- UI: `packages/client/src/App.tsx`
- API: Connects to REST and Socket.IO endpoints via hooks
- CLI: `pnpm --filter client dev`

## Behavior and Boundaries
- Renders kanban board, task detail panel, and execution controls.
- Uses REST APIs for task creation/execution status and Socket.IO for realtime sync.

## Data and Storage Impact
- None (client-side state only).

## Permissions and Risks
- Requires access to backend Socket.IO and REST endpoints.

## Observability
- Displays gateway status and task execution updates in the UI.

## Test and Verification
- Run `pnpm --filter client test`.

## Related Changes
- Depends on `@opencode-vibe/protocol`.
