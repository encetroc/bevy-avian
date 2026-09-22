# 37: Add sensor zones

**What to build:** Add transparent sensor volumes that report object entry and exit.

**Blocked by:** 36

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Sensor volumes do not physically block bodies.
- [ ] Entry and exit events appear in the event log.

## Tests

- [ ] Move the player and an object through a sensor.
- [ ] Verify crossing the boundary in both directions produces events.
