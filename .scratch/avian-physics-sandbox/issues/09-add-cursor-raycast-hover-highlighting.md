# 09: Add cursor raycast hover highlighting

**What to build:** Highlight the physical object under the cursor and show its basic identity.

**Blocked by:** 04, 06

**Status:** ready-for-human

## Acceptance criteria

- [x] The highlighted object follows cursor movement.
- [x] Moving off an object clears the hover state.

## Tests

- [x] Move the cursor across several objects.
- [x] Verify non-physical scenery is not reported as a selectable body.
