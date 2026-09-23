# 37: Add sensor zones

**What to build:** Add transparent sensor volumes that report object entry and exit.

**Blocked by:** 36

**Status:** complete

## Acceptance criteria

- [x] Sensor volumes do not physically block bodies.
- [x] Entry and exit events appear in the event log.

## Tests

- [x] Headless test moves a kinematic player and dynamic object through the sensor.
- [x] Headless test verifies crossing the boundary in both directions produces ENTER and EXIT events.
