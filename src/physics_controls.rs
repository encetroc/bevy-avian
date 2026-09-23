use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;

const PAUSE_KEY: KeyCode = KeyCode::KeyP;
const STEP_KEY: KeyCode = KeyCode::KeyN;
const SPEED_KEYS: [(KeyCode, f32); 4] = [
    (KeyCode::Digit1, 0.25),
    (KeyCode::Digit2, 0.5),
    (KeyCode::Digit3, 1.0),
    (KeyCode::Digit4, 2.0),
];
const SUBSTEP_KEYS: [(KeyCode, u32); 4] = [
    (KeyCode::Digit5, 1),
    (KeyCode::Digit6, 2),
    (KeyCode::Digit7, 4),
    (KeyCode::Digit8, 8),
];

/// Owns keyboard controls for pausing, resuming, and stepping Avian physics.
pub(crate) struct PhysicsControlsPlugin;

impl Plugin for PhysicsControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_physics_controls_help)
            .add_systems(
                Update,
                (handle_physics_controls, update_physics_status).chain(),
            );
    }
}

#[derive(Component)]
struct PhysicsStatusText;

fn spawn_physics_controls_help(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: px(12),
            padding: UiRect::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.85)),
        Text::new(
            "PHYSICS: RUNNING  |  SPEED: 1x  |  SUBSTEPS: 6  |  1-4: SPEED  5:1 6:2 7:4 8:8 SUBSTEPS  P: PAUSE/RESUME  N: STEP",
        ),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        PhysicsStatusText,
    ));
}

/// Handles input in an exclusive system because stepping must run Avian's
/// complete physics schedule exactly once while its normal clock is paused.
fn handle_physics_controls(world: &mut World) {
    let (toggle_pause, step, selected_speed, selected_substeps) = {
        let input = world.resource::<ButtonInput<KeyCode>>();
        (
            input.just_pressed(PAUSE_KEY),
            input.just_pressed(STEP_KEY),
            SPEED_KEYS
                .iter()
                .find_map(|(key, speed)| input.just_pressed(*key).then_some(*speed)),
            SUBSTEP_KEYS
                .iter()
                .find_map(|(key, count)| input.just_pressed(*key).then_some(*count)),
        )
    };

    if let Some(speed) = selected_speed {
        world
            .resource_mut::<Time<Physics>>()
            .set_relative_speed(speed);
    }
    if let Some(substeps) = selected_substeps {
        world.resource_mut::<SubstepCount>().0 = substeps;
    }

    if toggle_pause {
        let mut time = world.resource_mut::<Time<Physics>>();
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    }

    if step {
        // A step always leaves simulation paused, even if requested while running.
        world.resource_mut::<Time<Physics>>().pause();
        let step_duration = world.resource::<Time<Fixed>>().timestep();
        world
            .resource_mut::<Time<Physics>>()
            .advance_by(step_duration);
        world.run_schedule(PhysicsSchedule);
        // Avian's regular fixed-schedule driver must see zero delta later this frame
        // or it would repeat this manually-triggered step.
        world
            .resource_mut::<Time<Physics>>()
            .advance_by(Duration::ZERO);
    }
}

fn update_physics_status(
    physics_time: Res<Time<Physics>>,
    substeps: Res<SubstepCount>,
    mut status: Query<&mut Text, With<PhysicsStatusText>>,
) {
    let state = if physics_time.is_paused() {
        "PAUSED"
    } else {
        "RUNNING"
    };
    let speed = physics_time.relative_speed();
    for mut text in &mut status {
        **text = format!(
            "PHYSICS: {state}  |  SPEED: {speed}x  |  SUBSTEPS: {}  |  1-4: SPEED  5:1 6:2 7:4 8:8 SUBSTEPS  P: PAUSE/RESUME  N: STEP",
            substeps.0
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy, transform::TransformPlugin,
    };

    fn physics_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            InputPlugin,
            TransformPlugin,
            PhysicsPlugins::default(),
            PhysicsControlsPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        app.finish();
        let body = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                Position(Vec3::new(0.0, 100.0, 0.0)),
                Transform::from_xyz(0.0, 100.0, 0.0),
                LinearVelocity(Vec3::X),
                Name::new("Pause control test body"),
            ))
            .id();
        (app, body)
    }

    fn press(app: &mut App, key: KeyCode) {
        let world = app.world_mut();
        world.resource_mut::<ButtonInput<KeyCode>>().press(key);
        handle_physics_controls(world);
        world.resource_mut::<ButtonInput<KeyCode>>().release(key);
    }

    fn distance_after_equal_wall_time(speed_key: KeyCode) -> f32 {
        let (mut app, body) = physics_app();
        press(&mut app, speed_key);
        let initial_x = position(&app, body).x;
        for _ in 0..120 {
            app.update();
        }
        position(&app, body).x - initial_x
    }

    fn position(app: &App, body: Entity) -> Vec3 {
        app.world().entity(body).get::<Position>().unwrap().0
    }

    #[test]
    fn speed_settings_scale_motion_over_equal_wall_clock_intervals() {
        let measured: Vec<_> = SPEED_KEYS
            .iter()
            .map(|(key, speed)| (*speed, distance_after_equal_wall_time(*key)))
            .collect();

        let baseline = measured[2].1;
        assert!(
            baseline > 0.0,
            "1x simulation made no progress: {measured:?}"
        );
        for (speed, distance) in measured {
            let expected = baseline * speed;
            assert!(
                (distance - expected).abs() < 0.02,
                "{speed}x moved {distance}, expected approximately {expected}"
            );
        }
    }

    #[test]
    fn speed_can_change_while_running_and_pause_resume_preserves_speed() {
        let (mut app, body) = physics_app();
        app.update();
        press(&mut app, KeyCode::Digit4);
        assert_eq!(
            app.world().resource::<Time<Physics>>().relative_speed(),
            2.0
        );
        press(&mut app, PAUSE_KEY);
        let stopped_at = position(&app, body);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(position(&app, body), stopped_at);

        press(&mut app, KeyCode::Digit1);
        assert_eq!(
            app.world().resource::<Time<Physics>>().relative_speed(),
            0.25
        );
        assert!(app.world().resource::<Time<Physics>>().is_paused());
        press(&mut app, PAUSE_KEY);
        for _ in 0..8 {
            app.update();
        }
        assert!(position(&app, body).x > stopped_at.x);
        assert_eq!(
            app.world().resource::<Time<Physics>>().relative_speed(),
            0.25
        );
    }

    #[test]
    fn substep_settings_can_be_repeated_without_corrupting_bodies() {
        let (mut app, body) = physics_app();
        let second_body = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                Position(Vec3::new(1.0, 100.0, 0.0)),
                Transform::from_xyz(1.0, 100.0, 0.0),
                LinearVelocity::default(),
                Name::new("Substep control joint test body"),
            ))
            .id();
        app.world_mut().spawn(
            FixedJoint::new(body, second_body)
                .with_local_anchor1(Vec3::X * 0.5)
                .with_local_anchor2(Vec3::NEG_X * 0.5),
        );
        let mut previous_x = position(&app, body).x;

        for (key, expected_count) in SUBSTEP_KEYS {
            press(&mut app, key);
            assert_eq!(app.world().resource::<SubstepCount>().0, expected_count);

            for _ in 0..4 {
                app.update();
            }

            let entity = app.world().entity(body);
            let current_x = entity.get::<Position>().unwrap().x;
            assert!(
                current_x > previous_x,
                "body stopped at {expected_count} substeps"
            );
            assert!(current_x.is_finite());
            assert!(entity.get::<LinearVelocity>().unwrap().0.is_finite());
            let second_position = position(&app, second_body);
            assert!(second_position.is_finite());
            let joint_distance = current_x - second_position.x;
            assert!(
                (joint_distance.abs() - 1.0).abs() < 0.1,
                "joint drifted to {joint_distance} at {expected_count} substeps"
            );
            previous_x = current_x;
        }

        for (key, expected_count) in SUBSTEP_KEYS.into_iter().rev() {
            press(&mut app, key);
            assert_eq!(app.world().resource::<SubstepCount>().0, expected_count);
            app.update();
            assert!(position(&app, body).x > previous_x);
            previous_x = position(&app, body).x;
        }
    }

    #[test]
    fn pause_stops_a_moving_body_and_resume_restarts_it() {
        let (mut app, body) = physics_app();
        app.update();
        press(&mut app, PAUSE_KEY);

        let stopped_at = position(&app, body);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(position(&app, body), stopped_at);
        assert!(app.world().resource::<Time<Physics>>().is_paused());

        press(&mut app, PAUSE_KEY);
        for _ in 0..4 {
            app.update();
        }
        assert!(position(&app, body).x > stopped_at.x + 0.02);
        assert!(!app.world().resource::<Time<Physics>>().is_paused());
    }

    #[test]
    fn each_single_step_advances_once_and_keeps_physics_paused() {
        let (mut app, body) = physics_app();
        app.update();
        press(&mut app, PAUSE_KEY);

        let paused_at = position(&app, body);
        press(&mut app, STEP_KEY);
        let after_one = position(&app, body);
        assert!(after_one.x > paused_at.x, "first step made no progress");
        assert!(app.world().resource::<Time<Physics>>().is_paused());

        press(&mut app, STEP_KEY);
        let after_two = position(&app, body);
        assert!(after_two.x > after_one.x, "second step made no progress");
        let step_distance = after_one.x - paused_at.x;
        assert!(
            ((after_two.x - after_one.x) - step_distance).abs() < 0.002,
            "steps were not discrete and equal: {paused_at:?}, {after_one:?}, {after_two:?}"
        );
        assert!(app.world().resource::<Time<Physics>>().is_paused());
    }
}
