# 10: Add spring-based object grabbing

**What to build:** Allow left-click dragging to pull dynamic objects toward a world-space target without teleporting them.

**Blocked by:** 09

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Grabbed objects retain physics and collide with the environment.
- [ ] Objects lag behind rapid cursor movement and release naturally.

## Tests

- [ ] Drag a light and heavy object at different speeds.
- [ ] Move a grabbed object into another body and verify collision response.
