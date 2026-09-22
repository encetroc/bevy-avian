# 57: Add counted body spawning

**What to build:** Add a station for spawning 10, 100, 500, and 1000 bodies plus Clear.

**Blocked by:** 03, 06

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Each spawn action creates the requested number of bodies.
- [ ] Clear removes the spawned bodies and leaves station geometry.

## Tests

- [ ] Run every count and compare diagnostics.
- [ ] Clear after each run and verify counts return to baseline.
