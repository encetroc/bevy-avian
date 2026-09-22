# 14: Make core rigid-body properties editable

**What to build:** Expose runtime editing for mass, gravity scale, damping, body type, and linear/angular velocity.

**Blocked by:** 13

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Each supported value can be edited without recompiling.
- [ ] Changed values affect the next simulation frames.

## Tests

- [ ] Change mass and compare push response.
- [ ] Change velocity and gravity scale and observe the resulting motion.
