# 62: Add object deletion and clearing

**What to build:** Allow Delete to remove the selected spawned object and provide a clear-spawned-objects action.

**Blocked by:** 61

**Status:** complete

## Acceptance criteria

- [x] Delete removes only the selected removable object.
- [x] Clear removes spawned objects without removing protected world geometry.

## Tests

- [x] Delete selected and unselected objects.
- [x] Use Clear after spawning multiple object types and verify protected arena geometry remains.
