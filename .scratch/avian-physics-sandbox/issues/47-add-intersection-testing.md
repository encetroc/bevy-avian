# 47: Add intersection testing

**What to build:** Add a movable debug volume that is green when clear and red when intersecting.

**Blocked by:** 42

**Status:** complete

## Acceptance criteria

- [x] The volume color reflects intersection state.
- [x] Moving the volume updates the state immediately.

## Tests

- [x] Headless tests move through empty space and an Avian collider, asserting immediate state and color changes.
- [x] The volume uses an Avian collider, so F1 debug visualization can be compared with its colored mesh.
