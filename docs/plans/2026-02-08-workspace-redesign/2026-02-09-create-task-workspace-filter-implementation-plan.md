# Create Task Modal Workspace Filter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let users scope project choices by workspace when creating tasks, using the workspace-aware project listing added in earlier milestones.

**Architecture:** Add a lightweight `useWorkspaces` hook to fetch workspace summaries, then wire `CreateTaskModal` with a workspace selector that passes `workspaceId` into `useProjects`. Keep task creation payload unchanged (`projectId` remains source of truth) and make filtering purely a UX enhancement.

**Tech Stack:** TypeScript + React hooks/components (`packages/client`), Vitest + Testing Library.

---

### Task 1: Add failing modal test for workspace-driven project filtering

**Files:**
- Create: `packages/client/src/components/task/__tests__/CreateTaskModal.test.tsx`

**Step 1: Write failing tests first**
- Add tests asserting:
  - modal shows a workspace selector field,
  - selecting a workspace makes `useProjects` receive `{ workspaceId: <id> }`.

**Step 2: RED run**
- Run: `pnpm --filter client test -- src/components/task/__tests__/CreateTaskModal.test.tsx`
- Expected: FAIL because modal currently lacks workspace UI and does not pass workspace filters to `useProjects`.

### Task 2: Implement workspace hook + modal integration

**Files:**
- Create: `packages/client/src/hooks/useWorkspaces.ts`
- Modify: `packages/client/src/components/task/CreateTaskModal.tsx`
- Modify: `packages/client/src/lexicon/consoleLexicon.ts`

**Step 1: Minimal implementation**
- Add `useWorkspaces` hook to fetch `/api/workspaces` and expose `workspaces`, `isLoading`, `error`, `hasWorkspaces`.
- In `CreateTaskModal`:
  - add workspace selector state,
  - pass `{ workspaceId }` into `useProjects` when selected,
  - reset selected project/model when workspace changes,
  - show workspace field copy and empty/loading states.

**Step 2: GREEN run**
- Re-run the focused test command.
- Expected: PASS.

### Task 3: Verification + progress logging

**Files:**
- Modify: `docs/plans/2026-02-08-workspace-redesign/progress.md`

**Step 1: Client checks**
- `pnpm --filter client test -- src/components/task/__tests__/CreateTaskModal.test.tsx`
- `pnpm --filter client test`

**Step 2: Safety checks**
- `cargo test -p vk-core -p agent-runner -p api-server`
- `pnpm run test:scripts`

**Step 3: Record evidence**
- Append M9 RED/GREEN + verification results to `progress.md`.
