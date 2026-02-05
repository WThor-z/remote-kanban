# Task Commands

## Summary
Create and manage tasks quickly from the command input without leaving the board. It provides a fast path for adding tasks and basic status updates via slash commands.

## Entry Points
- UI: Command input box
- API: N/A
- CLI: N/A

## Behavior and Boundaries
- Create tasks with title and optional description via `/task <title> | <desc>` or `/task add <title>`.
- Move tasks and mark done via `/task move` and `/task done`.
- Delete tasks via `/task delete <id>`.
- Only `/task` and `/todo` command variants are supported; other input does not create tasks.

## Data and Storage Impact
- Updates task data stored in `.opencode/kanban.json`.

## Permissions and Risks
- None (local task operations).

## Observability
- Client emits `kanban:create`/`kanban:move` and server broadcasts `kanban:sync` for client sync.

## Test and Verification
- Create a task via `/task` and ensure it appears on the board.

## Related Changes
- README commands section
