# Memory Enhanced v1

## Goal
- Improve retrieval quality without changing core host/project scope semantics.
- Keep `scope=host|project` compatibility unchanged.

## Added Settings
- `dedupeEnabled` (bool, default `true`)
  - Prevents duplicate writes for same host/project/scope/kind/content.
- `recencyHalfLifeHours` (u32, default `72`)
  - Controls recency decay weight for retrieval ordering.
- `hitCountWeight` (f32, default `0.15`)
  - Scales impact of historical usage (`hitCount`).
- `pinnedBoost` (f32, default `1.25`)
  - Adds ranking boost for pinned items.

## Retrieval Ranking
Memory listing now sorts by weighted score:
- confidence
- recency decay (`recencyHalfLifeHours`)
- usage frequency (`hitCountWeight`)
- pinned boost (`pinnedBoost`)
- disabled penalty

## Dedupe Behavior
When `dedupeEnabled=true`, creating an item that matches:
- `hostId`
- `projectId`
- `scope`
- `kind`
- normalized content

will update the existing item (`updatedAt`, `lastUsedAt`, `hitCount`) instead of creating a new entry.

## Backward Compatibility
- New fields are deserialization-safe with defaults.
- Existing gateway payloads/settings without new fields continue to work.
