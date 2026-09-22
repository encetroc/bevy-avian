# 55: Add interpolation and extrapolation comparisons

**What to build:** Add fast-moving objects and toggles for evaluating rendered transform interpolation and extrapolation.

**Blocked by:** 06, 52

**Status:** ready-for-agent

## Acceptance criteria

- [ ] The visual mode can be changed at runtime.
- [ ] Physics state remains independent of the rendering mode.

## Tests

- [ ] Observe a fast-moving body with each mode.
- [ ] Pause physics and verify rendering does not create extra simulation steps.
