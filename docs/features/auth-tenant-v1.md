# Auth & Tenant Isolation v1

## Goal
- Introduce organization-aware authentication primitives.
- Enforce org scoping on orchestrator and ops APIs when multi-tenant mode is enabled.

## New APIs
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`
- `GET /api/v1/me`
- `GET /api/v1/orgs`
- `POST /api/v1/orgs`

## JWT Claims
User JWT includes:
- `sub`
- `org_id`
- `role`
- `exp`

## Feature Flag Behavior
- `FEATURE_MULTI_TENANT=false` (default):
  - Existing local/dev flows continue to work without bearer token.
- `FEATURE_MULTI_TENANT=true`:
  - `/api/v1/executions/*` and `/api/v1/ops/*` require bearer JWT.
  - Route-level org filtering is forced to authenticated `org_id`.

## Scope Enforcement
- Executions list/filter is constrained by authenticated `org_id`.
- Execution detail/input/stop/events deny cross-org access.
- Ops summary/executions/audit are constrained to authenticated org scope.
