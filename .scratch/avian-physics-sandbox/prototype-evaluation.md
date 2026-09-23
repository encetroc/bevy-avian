# Prototype evaluation scenarios and observation log

This is the repeatable, windowless evaluation record for ticket 68. Each automated case constructs a fresh Bevy `App` with `MinimalPlugins`, a fixed 60 Hz manual timestep, and the relevant feature plugin; test teardown is the clean reset. No window, display server, or GUI is used. These tests validate simulation/input seams, not rendered appearance or real frame rate.

## Repeatable scenarios

Run any row independently with `cargo test <filter> -- --exact` (or use the grouped commands below). The test creates a clean starting state; run it again for a clean repeat.

| Scenario | Clean setup and action | Observable pass condition | Headless test |
| --- | --- | --- | --- |
| Picking | Fresh app; click a physics body, then non-physics scenery through the mesh-picking observer. | Physics body is selected; scenery does not replace it. Picking alone does not grab. | `picking_comparison::tests::mesh_pick_observer_records_physics_entity_but_ignores_scenery` |
| Dragging | Fresh app/player/body; hold grab modifier + left mouse, move the cursor ray, simulate fixed steps, then release. | Body follows through spring force rather than teleportation; heavy bodies lag more; release removes spring force while preserving velocity. | `object_grabbing::tests::target_motion_pulls_without_teleporting_and_heavy_bodies_lag_more`; `object_grabbing::tests::release_removes_spring_but_preserves_natural_velocity` |
| Throwing | Fresh app/body; grab, move the target, invoke right-click throw (repeat with configured strength). | Grab ends and the body receives target momentum; changing strength changes throw velocity. | `object_grabbing::tests::right_click_throw_releases_the_body_and_preserves_target_momentum`; `object_grabbing::tests::configured_throw_strength_scales_low_and_high_target_speeds` |
| Stacking | Fresh station-J app; disturb the prepared stack and advance fixed steps. | Bodies move, come to rest, and eligible objects enter sleeping state. | `stacking_station::tests::disturbed_objects_move_and_the_stacks_can_come_to_rest_and_sleep` |
| Composition | Fresh app; activate composition, select plank and supports, then apply a collision. | Fixed joints are created and the relative poses remain constrained after impact. | `object_composition::tests::plank_and_two_supports_remain_connected_after_a_collision` |
| Sleeping | Fresh station-J app; exercise sleep/wake/toggle controls on eligible bodies. | Only eligible lab bodies are affected and sleeping counts/state respond to controls. | `stacking_station::tests::sleep_controls_count_sleep_wake_and_toggle_only_eligible_lab_bodies` |
| Stress | Fresh station-I app for each count; select cubes, spawn a load, assert count, clear it, then repeat. The test runs 10, 100, 500, and 1,000 bodies in separate cleared trials. | Requested dynamic body count is reached exactly; clear returns the load to zero before the next trial. | `counted_body_spawning::tests::each_count_control_creates_the_requested_number_of_bodies` |

Grouped clean-reset runs:

```sh
cargo test picking_comparison::tests::mesh_pick_observer_records_physics_entity_but_ignores_scenery
cargo test object_grabbing::tests::target_motion_pulls_without_teleporting_and_heavy_bodies_lag_more
cargo test object_grabbing::tests::release_removes_spring_but_preserves_natural_velocity
cargo test object_grabbing::tests::right_click_throw_releases_the_body_and_preserves_target_momentum
cargo test object_grabbing::tests::configured_throw_strength_scales_low_and_high_target_speeds
cargo test stacking_station::tests::disturbed_objects_move_and_the_stacks_can_come_to_rest_and_sleep
cargo test stacking_station::tests::sleep_controls_count_sleep_wake_and_toggle_only_eligible_lab_bodies
cargo test object_composition::tests::plank_and_two_supports_remain_connected_after_a_collision
cargo test counted_body_spawning::tests::each_count_control_creates_the_requested_number_of_bodies
```

## Observation log

Environment: Bevy 0.19.1 / Avian3D 0.7; simulation-only `MinimalPlugins`; fixed 60 Hz test timestep. Test evidence is from isolated headless unit tests. An FPS value is not meaningful because these runs create no rendered frames. The current load-count test validates spawning and clearing, not sustained load performance; physics timing/contact/sleeping observations for those loads are therefore explicitly unmeasured rather than inferred from spawn counts.

| Trial / load | Bodies | Sleeping bodies | Contacts | Physics time | FPS | Qualitative notes |
| --- | ---: | ---: | ---: | --- | --- | --- |
| Spawn/clear, 100 | 100 reached; then 0 | Not sampled | Not sampled | Not sampled | N/A (no renderer) | Exact count and clear-to-zero asserted in a fresh app. |
| Spawn/clear, 500 | 500 reached; then 0 | Not sampled | Not sampled | Not sampled | N/A (no renderer) | Exact count and clear-to-zero asserted in a fresh app. |
| Spawn/clear, 1,000 | 1,000 reached; then 0 | Not sampled | Not sampled | Not sampled | N/A (no renderer) | Exact count and clear-to-zero asserted in a fresh app. |
| HUD count smoke test | 512 dynamic bodies | 512 component-marked sleeping bodies | Not sampled | Physics step timing field is asserted present, numeric value not captured | N/A (no renderer) | HUD remains responsive and reports the spawned body/collider/entity counts. |
| Stacking/sleeping | Station fixture | Sleeping transition asserted | Not logged | Not logged | N/A (no renderer) | Disturbed stack settles; dedicated control test checks sleep/wake/toggle behavior. |
| Picking / dragging / throwing / composition | Small isolated fixtures | N/A | Collision assertions cover throw and composition cases | Fixed-step behavior asserted; numeric cost not sampled | N/A (no renderer) | Input, spring-grab, momentum transfer, and jointed composition assertions pass independently. |

Executed on this session: all nine scenario/stress commands listed above, plus `cargo test diagnostics_hud::tests::hud_remains_responsive_with_a_stress_sized_body_count`; all passed individually in fresh headless test apps. `cargo test` passed all 212 tests; `cargo fmt --check` passed. `cargo clippy --all-targets -- -D warnings` is blocked by existing `clippy::type_complexity` findings in `src/camera.rs`, `src/friction_station.rs`, and `src/interpolation_comparison.rs` (outside ticket scope).

For a rendered/manual evaluation, replace `N/A`/`Not sampled` with measured values and record machine, renderer, resolution, build profile, test duration, and whether the measurement is a steady-state sample. Do not compare the headless test timings with rendered FPS.
