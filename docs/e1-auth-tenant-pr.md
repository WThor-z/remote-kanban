# E1 PR Package: Auth + Multi-Tenant Isolation

## Suggested PR Title
`feat(api-server): orchestrator v1 e1 auth and tenant isolation`

## Branch / Target
- Source: `feat/orch-e1-auth-tenant`
- Target: `dev`

## Scope
- Added orchestrator v1 auth module in Rust API server.
- Added endpoints:
  - `POST /api/v1/auth/register`
  - `POST /api/v1/auth/login`
  - `GET /api/v1/me`
  - `GET|POST /api/v1/orgs`
  - `GET /api/v1/orgs/{org_id}`
  - `GET|POST /api/v1/orgs/{org_id}/members`
  - `GET|POST /api/v1/orgs/{org_id}/api-keys`
- Added JWT claims contract:
  - `sub`
  - `org_id`
  - `role`
  - `exp`
- Added auth persistence file:
  - `.vk-data/auth/state.json`
- Added org isolation checks for org-scoped routes (`cross-org => 403`).

## Interface Change Notes
- `POST /api/v1/auth/register`: creates user + org + owner membership and returns access token.
- `POST /api/v1/auth/login`: supports org selection via `orgId` or `orgSlug`.
- `GET /api/v1/me`: returns user/org/role/claims from bearer token.
- `GET|POST /api/v1/orgs/{org_id}/members`: member listing and upsert.
- `GET|POST /api/v1/orgs/{org_id}/api-keys`: key listing and creation.

## Migration Notes
- No SQL migration.
- New file store only: `.vk-data/auth/state.json`.
- New env vars:
  - `VK_AUTH_JWT_SECRET`
  - `VK_AUTH_TOKEN_TTL_SECONDS` (default 8h)

## Rollback Steps
1. Revert this PR (or rollback deployed build).
2. Stop calling `/api/v1/auth/*` and `/api/v1/orgs/*` from clients.
3. Optional cleanup: remove `.vk-data/auth/state.json` if you are discarding E1 auth data.
4. Verify legacy routes are healthy (`/health`, `/api/tasks/*`, `/api/projects/*`, `/api/workspaces/*`).

## Acceptance Record
- Automated:
  - `cargo test -p api-server` => `76 passed, 0 failed`.
- New tests:
  - `routes::auth::tests::register_and_read_me`
  - `routes::orgs::tests::list_orgs_returns_current_user_orgs`
  - `routes::orgs::tests::cross_org_member_access_is_denied`

## PR Body Template
```md
## Summary
- Add orchestrator v1 E1 auth and multi-tenant isolation primitives.
- Introduce `/api/v1/auth/*`, `/api/v1/me`, `/api/v1/orgs/*`.
- Persist auth domain data in `.vk-data/auth/state.json`.

## API changes
- New endpoints:
  - `POST /api/v1/auth/register`
  - `POST /api/v1/auth/login`
  - `GET /api/v1/me`
  - `GET|POST /api/v1/orgs`
  - `GET /api/v1/orgs/{org_id}`
  - `GET|POST /api/v1/orgs/{org_id}/members`
  - `GET|POST /api/v1/orgs/{org_id}/api-keys`
- JWT claims contract: `sub`, `org_id`, `role`, `exp`

## Migration notes
- No SQL migration.
- New file store: `.vk-data/auth/state.json`
- New env vars:
  - `VK_AUTH_JWT_SECRET`
  - `VK_AUTH_TOKEN_TTL_SECONDS`

## Rollback
1. Revert this PR.
2. Stop calling `/api/v1/auth/*` and `/api/v1/orgs/*`.
3. Optionally remove `.vk-data/auth/state.json` if auth data should be discarded.

## Validation
- [x] `cargo test -p api-server` passed (`76/76`)
- [x] auth register/login/me flow tested
- [x] cross-org access denied (`403`) for org-scoped endpoint
```
