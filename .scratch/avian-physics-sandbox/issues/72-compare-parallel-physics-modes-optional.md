# 72: Compare parallel physics modes (optional)

**What to build:** Compare stress-test behavior with parallel physics enabled and disabled.

**Blocked by:** 57

**Status:** complete

## Acceptance criteria

- [x] Both configurations can run the same workload using the same headless test with Avian's `parallel` Cargo feature enabled or disabled.
- [x] Test output records headless update-throughput FPS and frame time, last-step physics time, contacts, finite body positions, and failures for each mode.

## Tests

- [x] Run identical 10, 100, 500, and 1,000 body counts in both modes.
- [x] Compare frame time, physics time, contacts, and stability output from `parallel_physics_comparison`.
- [x] `cargo test parallel_physics_comparison -- --nocapture`
- [x] `cargo test --no-default-features parallel_physics_comparison -- --nocapture`

The benchmark is headless: it does not open a window. Run the commands separately on the same machine to compare the emitted observations; the default build is parallel and `--no-default-features` is serial.
