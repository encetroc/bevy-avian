# 56: Add the diagnostics HUD

**What to build:** Display FPS, dynamic bodies, sleeping bodies, collider count, contact count, entity count, and physics timing metrics.

**Blocked by:** 51, 52

**Status:** ready-for-agent

## Acceptance criteria

- [ ] The HUD updates during normal simulation.
- [ ] Metrics remain usable during stress tests.

## Tests

- [ ] Compare metrics before and after spawning bodies.
- [ ] Pause the simulation and verify timing/state labels remain coherent.
