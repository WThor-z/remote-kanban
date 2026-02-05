# Package: pty-manager

## Summary
Node PTY wrapper used to spawn and manage terminal processes (deprecated).

## Entry Points
- UI: N/A
- API: NPM package `@opencode-vibe/pty-manager`
- CLI: N/A

## Behavior and Boundaries
- Spawns PTY processes and provides basic I/O helpers.
- Windows uses `useConpty = false` for compatibility.
- Deprecated and retained for compatibility (per root README).

## Data and Storage Impact
- None.

## Permissions and Risks
- Spawns local processes; requires appropriate OS permissions.

## Observability
- None.

## Test and Verification
- Run `pnpm --filter @opencode-vibe/pty-manager test`.

## Related Changes
- Referenced by `packages/server` dependencies.
