# 33: Add joint inspection and runtime controls

**What to build:** Allow selecting a joint and inspecting or changing its enabled state, anchors, compliance, damping, and available stress information.

**Blocked by:** 28, 29, 30, 31, 32

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Joint selection identifies both connected entities and the joint type.
- [ ] Enable, disable, and delete controls work at runtime.

## Tests

- [ ] Inspect every joint type.
- [ ] Disable, re-enable, and delete selected joints while bodies are active.
