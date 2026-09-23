# 73: Add optional physics validation mode

**What to build:** Expose a development-only Avian validation configuration for diagnosing invalid physics setup.

**Blocked by:** 56

**Status:** complete

## Acceptance criteria

- [x] Validation can be enabled without changing normal defaults (`dev-physics-validation` Cargo feature; disabled by default).
- [x] Validation mode is reported in the console and diagnostics HUD; the headless physics smoke test runs without a crash.

## Tests

- [x] Run the Bevy physics app headlessly with validation enabled (no GUI/window, as required).
- [x] Run the full headless station suite with validation enabled; diagnostics HUD data remains available in smoke and stress tests.
