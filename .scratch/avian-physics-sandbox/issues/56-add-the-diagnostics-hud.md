# 56: Add the diagnostics HUD

**What to build:** Display FPS, dynamic bodies, sleeping bodies, collider count, contact count, entity count, and physics timing metrics.

**Blocked by:** 51, 52

**Status:** complete

## Acceptance criteria

- [x] The HUD updates during normal simulation.
- [x] Metrics remain usable during stress tests.

## Tests

- [x] Compare metrics before and after spawning bodies (headless test).
- [x] Pause the simulation and verify timing/state labels remain coherent (headless test).
