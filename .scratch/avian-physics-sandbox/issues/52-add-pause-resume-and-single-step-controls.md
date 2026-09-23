# 52: Add pause, resume, and single-step controls

**What to build:** Add controls for pausing, resuming, and advancing exactly one physics step.

**Blocked by:** 06

**Status:** complete

## Acceptance criteria

- [x] Physics stops while paused (P toggles pause/resume).
- [x] Single-step advances simulation once without resuming continuous time (N pauses and advances one fixed step).

## Tests

- [x] Pause a moving body and verify its transform is stable (headless test).
- [x] Step repeatedly and verify discrete progress (headless test).
