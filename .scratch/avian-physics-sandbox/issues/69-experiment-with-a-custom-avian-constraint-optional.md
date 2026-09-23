# 69: Experiment with a custom Avian constraint (optional)

**What to build:** Implement one minimal custom constraint to evaluate Avian extensibility.

**Blocked by:** 28, 29, 30, 31, 32

**Status:** complete

## Acceptance criteria

- [x] The constraint has one documented purpose and demonstration: a custom XPBD center-distance constraint keeps the demo body's center 2 m from a static anchor.
- [x] It can be enabled and observed without affecting other stations; the toggle is isolated to the free-sandbox demonstration.

## Tests

- [x] Run the custom-constraint station repeatedly (headless test creates and simulates the station three times).
- [x] Disable it and verify the demonstration returns to unconstrained behavior, then re-enable and verify the distance constraint again.
