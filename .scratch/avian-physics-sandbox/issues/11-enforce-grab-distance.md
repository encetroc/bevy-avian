# 11: Enforce grab distance

**What to build:** Prevent grabbing objects outside a configurable maximum interaction radius while retaining hover feedback.

**Blocked by:** 10

**Status:** ready-for-human

## Acceptance criteria

- [x] Objects inside the radius can be grabbed.
- [x] Objects outside the radius show an unreachable state and cannot be grabbed.

## Tests

- [x] Test objects at the radius boundary.
- [x] Move the player and verify grab eligibility changes with distance.
