# 26: Add scene collider generation

**What to build:** Load a simple Bevy scene and generate colliders for its mesh hierarchy.

**Blocked by:** 23

**Status:** complete

## Acceptance criteria

- [x] Scene meshes receive usable colliders without manual per-mesh setup.
- [x] Generated colliders preserve the scene hierarchy transforms.

## Tests

- [x] Load the scene and inspect all generated colliders.
- [x] Drop a body onto each scene mesh.
