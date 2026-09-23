use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;

const PAUSE_KEY: KeyCode = KeyCode::KeyP;
const STEP_KEY: KeyCode = KeyCode::KeyN;

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
        Text::new("PHYSICS: RUNNING  |  P: PAUSE/RESUME  |  N: SINGLE STEP"),
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
    let (toggle_pause, step) = {
        let input = world.resource::<ButtonInput<KeyCode>>();
        (input.just_pressed(PAUSE_KEY), input.just_pressed(STEP_KEY))
    };

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
    mut status: Query<&mut Text, With<PhysicsStatusText>>,
) {
    let state = if physics_time.is_paused() {
        "PAUSED"
    } else {
        "RUNNING"
    };
    for mut text in &mut status {
        **text = format!("PHYSICS: {state}  |  P: PAUSE/RESUME  |  N: SINGLE STEP");
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

    fn position(app: &App, body: Entity) -> Vec3 {
        app.world().entity(body).get::<Position>().unwrap().0
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
