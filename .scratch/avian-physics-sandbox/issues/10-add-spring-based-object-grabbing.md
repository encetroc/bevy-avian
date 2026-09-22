# 10: Add spring-based object grabbing

**What to build:** Allow left-click dragging to pull dynamic objects toward a world-space target without teleporting them.

**Blocked by:** 09

**Status:** ready-for-human

## Acceptance criteria

- [x] Grabbed objects retain physics and collide with the environment.
- [x] Objects lag behind rapid cursor movement and release naturally.

## Tests

- [x] Drag a light and heavy object at different speeds.
- [x] Move a grabbed object into another body and verify collision response.
