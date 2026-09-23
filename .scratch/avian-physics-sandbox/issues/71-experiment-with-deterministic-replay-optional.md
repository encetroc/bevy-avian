# 71: Experiment with deterministic replay (optional)

**What to build:** Run a predefined physics scenario repeatedly and compare its outcomes.

**Blocked by:** 52, 57

**Status:** complete

## Acceptance criteria

- [x] The same tick-indexed input sequence can be replayed from a reset headless physics world.
- [x] Final position, rotation, velocity, body/collider counts, and active-contact diagnostics are compared; divergences are retained as named, before/after measurements in the failure report.

## Tests

- [x] Run the scenario twice from fresh reset worlds using the same 180-tick trace.
- [x] Compare final transforms and relevant diagnostics in `deterministic_replay::tests::replaying_the_same_inputs_from_reset_compares_final_state_and_diagnostics` (headless).
