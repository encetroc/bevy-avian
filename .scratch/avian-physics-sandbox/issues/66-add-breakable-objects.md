# 66: Add breakable objects

**What to build:** Add impact-threshold breakage for ceramic, wood, and stone objects using predefined replacement pieces.

**Blocked by:** 36, 64

**Status:** complete

## Acceptance criteria

- [x] Objects break only when collision impulse exceeds their threshold.
- [x] Replacement pieces remain ordinary physical bodies.

## Tests

- [x] Verify impacts below and above the ceramic, wood, and stone thresholds.
- [x] Verify the same object cannot be claimed for breakage twice during one contact event.
