# 41: Add runtime collider and body toggles

**What to build:** Add inspector controls to disable and re-enable a selected collider or rigid body.

**Blocked by:** 13, 36

**Status:** complete

## Acceptance criteria

- [x] A disabled collider stops participating in collisions.
- [x] A disabled body stops simulating and can be restored.

## Tests

- [x] Disable a collider while its body is moving.
- [x] Re-enable the collider/body and verify simulation resumes correctly.
