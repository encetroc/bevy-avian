# 33: Add joint inspection and runtime controls

**What to build:** Allow selecting a joint and inspecting or changing its enabled state, anchors, compliance, damping, and available stress information.

**Blocked by:** 28, 29, 30, 31, 32

**Status:** complete

## Acceptance criteria

- [x] Joint selection identifies both connected entities and the joint type.
- [x] Enable, disable, and delete controls work at runtime.

## Tests

- [x] Inspect every joint type.
- [x] Disable, re-enable, and delete selected joints while bodies are active.
