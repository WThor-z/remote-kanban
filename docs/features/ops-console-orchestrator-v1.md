# Ops Console Orchestrator v1

## Goal
- Promote an operations-first control plane for remote agent execution.
- Keep Kanban available as a secondary workflow view.
- Preserve `/api/tasks/*` compatibility while introducing `/api/v1/ops/*`.

## New APIs
- `GET /api/v1/ops/summary`
  - Aggregated host, execution, and memory settings snapshot.
- `GET /api/v1/ops/executions`
  - Execution list for ops dashboard with filters: `status`, `orgId`, `hostId`, `taskId`, `offset`, `limit`.
- `GET /api/v1/ops/audit`
  - Audit stream for key operator actions with filters and pagination.

## Execution APIs Expanded
- `GET /api/v1/executions`
  - List execution records with filters and pagination.
- Existing v1 endpoints remain:
  - `POST /api/v1/executions`
  - `GET /api/v1/executions/{id}`
  - `POST /api/v1/executions/{id}/input`
  - `POST /api/v1/executions/{id}/stop`
  - `GET /api/v1/executions/{id}/events`

## Audit Coverage
- Audit records are persisted for:
  - execution creation (`/api/v1/executions`)
  - execution input forwarding/ignored
  - execution stop accepted/inferred/ignored
  - legacy task execute/stop/input routes

## Compatibility Hardening
- `/api/tasks/*` and task execution/session routes are still available.
- Legacy routes now return:
  - `Deprecation: true`
  - `Link: </api/v1/executions>; rel="successor-version"`

## Feature Flags
- `FEATURE_ORCHESTRATOR_V1=true|false` (default `true`)
  - Controls mounting of `/api/v1/executions` and `/api/v1/ops`.
- `FEATURE_MULTI_TENANT=true|false` (default `false`, reserved)
- `FEATURE_MEMORY_ENHANCED=true|false` (default `true`, exposed via `/health`)

## Health Endpoint
- `GET /health` now includes `featureFlags`:
  - `multiTenant`
  - `orchestratorV1`
  - `memoryEnhanced`
