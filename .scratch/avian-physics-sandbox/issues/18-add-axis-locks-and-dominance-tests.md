# 18: Add axis locks and dominance tests

**What to build:** Add runtime translation/rotation locks and a dominance comparison.

**Blocked by:** 14

**Status:** ready-for-human

## Acceptance criteria

- [x] Each translation and rotation axis can be locked.
- [x] Dominance-labelled bodies produce different collision outcomes where supported.

## Tests

- [x] Apply force to each locked-axis body.
- [x] Collide bodies with different dominance values and inspect the result.
