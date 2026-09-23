# 55: Add interpolation and extrapolation comparisons

**What to build:** Add fast-moving objects and toggles for evaluating rendered transform interpolation and extrapolation.

**Blocked by:** 06, 52

**Status:** complete

## Acceptance criteria

- [x] The visual mode can be changed at runtime.
- [x] Physics state remains independent of the rendering mode.

## Tests

- [x] Headless checks cover raw/interpolation/extrapolation mode changes on fast-moving comparison bodies.
- [x] Pause physics and verify mode changes do not advance simulation state.

Visual appearance was not inspected because this ticket was verified headlessly without opening a window.
