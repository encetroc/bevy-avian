# 67: Add liquid-container state

**What to build:** Add container capacity and amount state with a visible readout and controls.

**Blocked by:** 60, 64

**Status:** complete

## Acceptance criteria

- [x] The spawned barrel is a liquid container with capacity and current amount.
- [x] Selecting a container shows its amount; `=` / `-` changes its amount without replacing its dynamic rigid body.

## Tests

- [x] Create full, partial, and empty container states; values are clamped to capacity.
- [x] Headless Avian simulation confirms a dynamic container keeps simulating and retains updated amount state.
