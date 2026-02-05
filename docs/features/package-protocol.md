# Package: protocol

## Summary
Shared TypeScript types and parsers for kanban events and agent output messages.

## Entry Points
- UI: N/A
- API: NPM package `@opencode-vibe/protocol`
- CLI: N/A

## Behavior and Boundaries
- Exports kanban event types, agent types, and execution helpers.
- Provides a simple message parser for agent output streams.
- Does not perform network I/O or persistence.

## Data and Storage Impact
- None.

## Permissions and Risks
- None.

## Observability
- None.

## Test and Verification
- Run `pnpm --filter @opencode-vibe/protocol test`.

## Related Changes
- Used by `packages/client` and `packages/server`.
