# 66: Add breakable objects

**What to build:** Add impact-threshold breakage for ceramic, wood, and stone objects using predefined replacement pieces.

**Blocked by:** 36, 64

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Objects break only when collision impulse exceeds their threshold.
- [ ] Replacement pieces remain ordinary physical bodies.

## Tests

- [ ] Apply impacts below and above each threshold.
- [ ] Verify a broken object cannot break repeatedly from the same event.
