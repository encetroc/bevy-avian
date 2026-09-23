# 64: Add gameplay material definitions

**What to build:** Define Wood, Ceramic, Stone, Metal, and Glass gameplay materials separately from Avian physics components.

**Blocked by:** 14, 19

**Status:** done

## Acceptance criteria

- [x] Materials can provide density, friction, restitution, break threshold, and wind response defaults.
- [x] Avian remains directly usable without being hidden behind a replacement abstraction.

## Tests

- [x] Assign different materials to identical objects.
- [x] Verify defaults configure physics and gameplay state independently.
