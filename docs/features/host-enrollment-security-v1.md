# Host Enrollment & Gateway JWT Security v1

## Goal
- Replace static gateway token with host-scoped JWT validation in multi-tenant mode.
- Add host enrollment lifecycle APIs (enroll/rotate/disable).

## New APIs
- `POST /api/v1/orgs/{orgId}/hosts/enroll`
- `POST /api/v1/orgs/{orgId}/hosts/rotate-token`
- `POST /api/v1/orgs/{orgId}/hosts/disable`

## Host JWT Claims
Host JWT includes:
- `sub`
- `org_id`
- `role`
- `exp`
- `host_id`
- `token_version`

## Gateway Validation Rules
When `FEATURE_MULTI_TENANT=true`:
1. Gateway websocket requires bearer JWT.
2. `host_id` claim must match websocket query `hostId`.
3. Host enrollment must exist for `(org_id, host_id)`.
4. Enrollment must be enabled.
5. Enrollment `token_version` must match claim `token_version`.

When `FEATURE_MULTI_TENANT=false`:
- Gateway keeps existing static token behavior for local compatibility.
