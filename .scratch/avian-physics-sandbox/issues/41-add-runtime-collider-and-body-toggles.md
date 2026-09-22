# 41: Add runtime collider and body toggles

**What to build:** Add inspector controls to disable and re-enable a selected collider or rigid body.

**Blocked by:** 13, 36

**Status:** ready-for-agent

## Acceptance criteria

- [ ] A disabled collider stops participating in collisions.
- [ ] A disabled body stops simulating and can be restored.

## Tests

- [ ] Disable a collider while its body is moving.
- [ ] Re-enable the collider/body and verify simulation resumes correctly.
