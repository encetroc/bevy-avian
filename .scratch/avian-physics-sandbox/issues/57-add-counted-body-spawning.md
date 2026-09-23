# 57: Add counted body spawning

**What to build:** Add a station for spawning 10, 100, 500, and 1000 bodies plus Clear.

**Blocked by:** 03, 06

**Status:** complete

## Acceptance criteria

- [x] Each spawn action creates the requested number of bodies.
- [x] Clear removes the spawned bodies and leaves station geometry.

## Tests

- [x] Run every count and compare dynamic-body/collider diagnostics.
- [x] Clear after each run and verify counts return to baseline.
