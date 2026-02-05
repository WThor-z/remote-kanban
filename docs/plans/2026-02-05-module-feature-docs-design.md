# Module-Level Feature Docs Design

Date: 2026-02-05

## Overview

Create a module-level feature catalog under `docs/features/`, with one document
per top-level module and a single index for discovery. This complements the
existing user-facing feature doc (`Task Commands`) by separating user features
from module documentation.

## Goals

- One module = one document with a consistent template.
- A single index grouped by layer (crates, packages, services).
- Clear responsibilities, entry points, data impact, and verification steps.
- Lightweight maintenance with minimal process overhead.

## Non-Goals

- Auto-generated documentation or enforced CI checks.
- Deep implementation details or internal refactor notes.

## Structure

Directory layout:

```
docs/features/
  index.md
  _template.md
  task-commands.md
  crate-api-server.md
  crate-agent-runner.md
  crate-git-worktree.md
  crate-vk-core.md
  package-protocol.md
  package-server.md
  package-client.md
  package-pty-manager.md
  service-agent-gateway.md
```

## Index Format

`docs/features/index.md` contains two sections:

1) User Features
2) Module Catalog

Each section uses the same table:

| Feature | Summary | Status | Owner | Updated | Link |
|--------|---------|--------|-------|---------|------|

Status values: Stable, Active, Experimental, Deprecated.

## Template Guidance

Use `docs/features/_template.md` and keep the content focused on:

- Summary: module responsibilities and scope.
- Entry Points: HTTP/WS/CLI or package exports.
- Behavior and Boundaries: primary behaviors and limits.
- Data and Storage Impact: files, directories, or persistent stores.
- Permissions and Risks: destructive actions or access requirements.
- Observability: events, logs, or metrics.
- Test and Verification: minimal, repeatable checks.
- Related Changes: PRs/issues if available.

## Scope: Module Docs to Add

Crates:
- vk-core
- api-server
- agent-runner
- git-worktree

Packages:
- protocol
- server
- client
- pty-manager

Services:
- agent-gateway

## Maintenance

- Update module docs when public behavior, entry points, or data paths change.
- Update the index row date whenever the module doc changes.
- If a module is not currently wired, mark it as "present but not wired".

## Acceptance Checklist

- Summary and entry points are accurate.
- Data paths are explicit and correct.
- Observability signals are listed.
- Verification steps are actionable.
- Index updated with correct grouping and dates.
