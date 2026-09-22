# 26: Add scene collider generation

**What to build:** Load a simple Bevy scene and generate colliders for its mesh hierarchy.

**Blocked by:** 23

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Scene meshes receive usable colliders without manual per-mesh setup.
- [ ] Generated colliders preserve the scene hierarchy transforms.

## Tests

- [ ] Load the scene and inspect all generated colliders.
- [ ] Drop a body onto each scene mesh.
