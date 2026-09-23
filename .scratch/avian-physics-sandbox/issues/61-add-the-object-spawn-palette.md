# 61: Add the object-spawn palette

**What to build:** Add a Tab developer palette for cubes, balls, planks, barrels, heavy blocks, and bouncy balls.

**Blocked by:** 23, 60

**Status:** ready-for-agent

## Acceptance criteria

- [x] Tab opens and closes the palette.
- [x] Each preset spawns at the cursor world position or near the player.

## Tests

- [x] Headless tests spawn every preset and verify collider shape, mesh/material defaults, density, restitution, and cursor/player placement.
- [x] Headless tests verify Tab palette visibility and spawning fallback near the player.
