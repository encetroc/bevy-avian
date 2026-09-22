# 14: Make core rigid-body properties editable

**What to build:** Expose runtime editing for mass, gravity scale, damping, body type, and linear/angular velocity.

**Blocked by:** 13

**Status:** ready-for-human

## Acceptance criteria

- [x] Each supported value can be edited without recompiling.
- [x] Changed values affect the next simulation frames.

## Tests

- [x] Change mass and compare push response.
- [x] Change velocity and gravity scale and observe the resulting motion.
