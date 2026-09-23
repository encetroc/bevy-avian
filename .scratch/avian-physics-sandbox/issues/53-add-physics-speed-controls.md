# 53: Add physics speed controls

**What to build:** Add 0.25x, 0.5x, 1x, and 2x physics-speed settings.

**Blocked by:** 52

**Status:** complete

## Acceptance criteria

- [x] Speed can be changed while running (1-4 select 0.25x, 0.5x, 1x, and 2x).
- [x] The setting changes simulation progression without changing world scale (Avian's physics clock relative speed is adjusted).

## Tests

- [x] Measure motion over equal wall-clock intervals at each setting (headless deterministic test compares motion across equal 120-frame intervals).
- [x] Pause and resume after changing speed (headless test verifies speed changes while paused are preserved on resume).
