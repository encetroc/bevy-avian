use avian3d::{
    math::{AdjustPrecision, AsF32},
    prelude::*,
};
use bevy::{input::InputSystems, prelude::*};

use crate::camera::{CameraFollowTarget, FixedFollowCamera};

/// The player's capsule radius in world units.
pub const PLAYER_RADIUS: f32 = 0.45;
/// The length of the cylindrical part of the player's capsule.
pub const PLAYER_CAPSULE_LENGTH: f32 = 1.0;
/// The player's starting position above the arena floor.
pub const PLAYER_START_POSITION: Vec3 = Vec3::new(0.0, 1.0, 0.0);

/// A marker for the controllable kinematic player.
#[derive(Component)]
pub struct Player;

/// The movement input captured for the next fixed physics step.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput(pub Vec2);

/// Marks a player that is currently supported by a walkable surface.
#[derive(Component)]
pub struct PlayerGrounded;

/// Tunable settings for the kinematic player controller.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayerMovementSettings {
    /// Horizontal movement speed in world units per second.
    pub speed: f32,
    /// Downward acceleration applied while the player is airborne.
    pub gravity: f32,
    /// Maximum downward speed.
    pub terminal_velocity: f32,
}

impl Default for PlayerMovementSettings {
    fn default() -> Self {
        Self {
            speed: 5.0,
            gravity: 20.0,
            terminal_velocity: 50.0,
        }
    }
}

/// Owns player spawning, keyboard input, and collision-aware movement.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerMovementSettings>()
            // Bevy's keyboard state is updated by InputSystems in PreUpdate. Capture
            // it afterward so the fixed-step controller always sees current input.
            .add_systems(PreUpdate, capture_player_input.after(InputSystems))
            // Move-and-slide runs before Avian's FixedPostUpdate physics schedule,
            // matching the kinematic-controller contract: this system owns the
            // player's position integration and Avian owns collision queries.
            .add_systems(
                FixedUpdate,
                (apply_player_movement, move_player_and_slide).chain(),
            )
            .add_systems(Startup, spawn_player);
    }
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Capsule3d::new(PLAYER_RADIUS, PLAYER_CAPSULE_LENGTH)));
    let material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.95, 0.68, 0.18)));

    let mut player = commands.spawn((
        Player,
        CameraFollowTarget,
        RigidBody::Kinematic,
        // MoveAndSlide performs the position integration and prevents the
        // default kinematic integration from bypassing collision response.
        CustomPositionIntegration,
        SpeculativeMargin(0.0),
        LockedAxes::ROTATION_LOCKED,
        Collider::capsule(PLAYER_RADIUS, PLAYER_CAPSULE_LENGTH),
        LinearVelocity::default(),
        PlayerInput::default(),
        Transform::from_translation(PLAYER_START_POSITION),
        Name::new("Controllable Player"),
    ));

    if let (Some(mesh), Some(material)) = (mesh, material) {
        player.insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}

fn capture_player_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut players: Query<&mut PlayerInput, With<Player>>,
) {
    let horizontal =
        keyboard_input.pressed(KeyCode::KeyD) as i8 - keyboard_input.pressed(KeyCode::KeyA) as i8;
    let vertical =
        keyboard_input.pressed(KeyCode::KeyW) as i8 - keyboard_input.pressed(KeyCode::KeyS) as i8;
    let input = Vec2::new(horizontal as f32, vertical as f32).clamp_length_max(1.0);

    for mut player_input in &mut players {
        player_input.0 = input;
    }
}

fn apply_player_movement(
    settings: Res<PlayerMovementSettings>,
    camera: Option<Single<&Transform, With<FixedFollowCamera>>>,
    mut players: Query<(&PlayerInput, &mut LinearVelocity, Has<PlayerGrounded>), With<Player>>,
    time: Res<Time>,
) {
    let camera_rotation = camera
        .map(|camera| camera.rotation)
        .unwrap_or(Quat::IDENTITY);
    let delta_secs = time.delta_secs();

    for (input, mut velocity, grounded) in &mut players {
        // WASD is expressed in camera-local XZ space. Removing the camera's
        // vertical component keeps movement constrained to the arena floor.
        let local_direction = Vec3::new(input.0.x, 0.0, -input.0.y);
        let mut world_direction = camera_rotation * local_direction;
        world_direction.y = 0.0;
        world_direction = world_direction.normalize_or_zero();

        let horizontal_velocity = world_direction * settings.speed;
        velocity.x = horizontal_velocity.x.adjust_precision();
        velocity.z = horizontal_velocity.z.adjust_precision();

        if grounded && velocity.y < 0.0 {
            velocity.y = 0.0;
        } else {
            velocity.y = (velocity.y - settings.gravity * delta_secs)
                .max(-settings.terminal_velocity)
                .adjust_precision();
        }
    }
}

fn move_player_and_slide(
    mut commands: Commands,
    mut players: Query<(Entity, &mut Transform, &mut LinearVelocity, &Collider), With<Player>>,
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
) {
    for (entity, mut transform, mut velocity, collider) in &mut players {
        let mut grounded = false;
        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation.adjust_precision(),
            transform.rotation.adjust_precision(),
            velocity.0,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &SpatialQueryFilter::from_excluded_entities([entity]),
            |hit| {
                // A capsule can walk on surfaces up to a 60 degree slope. The
                // sandbox currently has flat floors, but the threshold makes
                // the grounded state useful for future static geometry.
                grounded |= hit.normal.y > 0.5;
                MoveAndSlideHitResponse::Accept
            },
        );

        transform.translation = output.position.f32();
        velocity.0 = output.projected_velocity;

        if grounded {
            commands.entity(entity).insert(PlayerGrounded);
        } else {
            commands.entity(entity).remove::<PlayerGrounded>();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;
    const ARENA_HALF_EXTENT: f32 = 10.0;
    const ARENA_WALL_THICKNESS: f32 = 0.5;
    const ARENA_WALL_HEIGHT: f32 = 4.0;

    fn player_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            PlayerPlugin,
        ))
        .add_systems(Startup, spawn_test_arena)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn spawn_test_arena(mut commands: Commands) {
        let floor_size = Vec3::new(
            ARENA_HALF_EXTENT * 2.0 + ARENA_WALL_THICKNESS * 2.0,
            ARENA_WALL_THICKNESS,
            ARENA_HALF_EXTENT * 2.0 + ARENA_WALL_THICKNESS * 2.0,
        );
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(floor_size.x, floor_size.y, floor_size.z),
            Transform::from_xyz(0.0, -ARENA_WALL_THICKNESS * 0.5, 0.0),
        ));

        let wall_center = ARENA_HALF_EXTENT + ARENA_WALL_THICKNESS * 0.5;
        let wall_size_x = Vec3::new(ARENA_WALL_THICKNESS, ARENA_WALL_HEIGHT, floor_size.z);
        let wall_size_z = Vec3::new(floor_size.x, ARENA_WALL_HEIGHT, ARENA_WALL_THICKNESS);
        for (position, size) in [
            (
                Vec3::new(wall_center, ARENA_WALL_HEIGHT * 0.5, 0.0),
                wall_size_x,
            ),
            (
                Vec3::new(-wall_center, ARENA_WALL_HEIGHT * 0.5, 0.0),
                wall_size_x,
            ),
            (
                Vec3::new(0.0, ARENA_WALL_HEIGHT * 0.5, wall_center),
                wall_size_z,
            ),
            (
                Vec3::new(0.0, ARENA_WALL_HEIGHT * 0.5, -wall_center),
                wall_size_z,
            ),
        ] {
            commands.spawn((
                RigidBody::Static,
                Collider::cuboid(size.x, size.y, size.z),
                Transform::from_translation(position),
            ));
        }
    }

    fn player_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<Player>>();
        players.iter(world).next().expect("controllable player")
    }

    fn player_position(app: &mut App) -> Vec3 {
        let player = player_entity(app);
        app.world()
            .entity(player)
            .get::<Transform>()
            .expect("player transform")
            .translation
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn hold_key(app: &mut App, key: KeyCode, steps: usize) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        run_steps(app, steps);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        run_steps(app, 2);
    }

    #[test]
    fn player_spawns_as_a_non_tipping_capsule_controller() {
        let mut app = player_app();
        app.update();
        let player = player_entity(&mut app);
        let entity = app.world().entity(player);

        assert!(entity.contains::<Collider>());
        assert_eq!(entity.get::<RigidBody>(), Some(&RigidBody::Kinematic));
        assert!(entity.contains::<CustomPositionIntegration>());
        assert!(entity.contains::<LockedAxes>());
        assert_eq!(player_position(&mut app), PLAYER_START_POSITION);
    }

    #[test]
    fn each_wasd_direction_moves_predictably_on_the_ground_plane() {
        for (key, expected_direction) in [
            (KeyCode::KeyW, Vec3::NEG_Z),
            (KeyCode::KeyA, Vec3::NEG_X),
            (KeyCode::KeyS, Vec3::Z),
            (KeyCode::KeyD, Vec3::X),
        ] {
            let mut app = player_app();
            app.update();
            let initial = player_position(&mut app);

            hold_key(&mut app, key, 30);

            let final_position = player_position(&mut app);
            let displacement = final_position - initial;
            assert!(
                displacement.dot(expected_direction) > 1.0,
                "{key:?} moved {:?}, expected {:?}",
                displacement,
                expected_direction
            );
            assert!(displacement.y.abs() < 0.1, "movement left the ground plane");
            assert!(
                displacement.cross(expected_direction).length() < 0.1,
                "{key:?} moved diagonally: {displacement:?}"
            );
        }
    }

    #[test]
    fn player_settles_on_the_floor_and_is_marked_grounded() {
        let mut app = player_app();
        app.update();
        run_steps(&mut app, 120);

        let player = player_entity(&mut app);
        let entity = app.world().entity(player);
        let position = entity.get::<Transform>().unwrap().translation;
        assert!(
            (0.85..=1.05).contains(&position.y),
            "player floated: {position:?}"
        );
        assert!(entity.contains::<PlayerGrounded>());
    }

    #[test]
    fn player_stops_at_each_wall_without_tunneling_or_tipping() {
        for (key, direction) in [
            (KeyCode::KeyD, Vec3::X),
            (KeyCode::KeyA, Vec3::NEG_X),
            (KeyCode::KeyS, Vec3::Z),
            (KeyCode::KeyW, Vec3::NEG_Z),
        ] {
            let mut app = player_app();
            app.update();
            hold_key(&mut app, key, 180);

            let player = player_entity(&mut app);
            let entity = app.world().entity(player);
            let position = entity.get::<Transform>().unwrap().translation;
            let distance_to_wall = direction.dot(position);
            assert!(
                distance_to_wall <= ARENA_HALF_EXTENT - PLAYER_RADIUS + 0.25,
                "{key:?} tunneled through wall at {position:?}"
            );
            assert!(position.y > 0.8, "player fell through floor: {position:?}");
            assert_eq!(
                entity.get::<Transform>().unwrap().rotation,
                Quat::IDENTITY,
                "player tipped while pushing against {key:?}"
            );
        }
    }
}
