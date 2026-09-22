use avian3d::prelude::*;
use bevy::prelude::*;

mod camera;
mod collider_debug;
mod cursor_hover;
mod dynamic_objects;
mod object_grabbing;
mod player;
mod stations;

use camera::CameraFollowPlugin;
#[cfg(test)]
use camera::FixedFollowCamera;
use collider_debug::ColliderDebugPlugin;
use cursor_hover::CursorHoverPlugin;
use dynamic_objects::DynamicObjectsPlugin;
use object_grabbing::ObjectGrabbingPlugin;
use player::PlayerPlugin;
use stations::StationLayoutPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            ColliderDebugPlugin,
            CursorHoverPlugin,
            SandboxPlugin,
            CameraFollowPlugin,
            ArenaPlugin,
            DynamicObjectsPlugin,
            ObjectGrabbingPlugin,
            PlayerPlugin,
            StationLayoutPlugin,
        ))
        .run();
}

/// Owns the initial world for the physics sandbox.
///
/// The sandbox starts with only the presentation entities needed to view a 3D
/// world. Physics geometry and interactive bodies are added by later features.
struct SandboxPlugin;

impl Plugin for SandboxPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.08)))
            .add_systems(Startup, spawn_sandbox_scene);
    }
}

/// Half the width and depth of the walkable area, in world units.
pub const ARENA_HALF_EXTENT: f32 = 10.0;
/// Thickness of the floor and enclosing walls.
pub const ARENA_WALL_THICKNESS: f32 = 0.5;
/// Height of each enclosing wall above the floor.
pub const ARENA_WALL_HEIGHT: f32 = 4.0;
/// Edge length of the dynamic body used to prove the arena is active.
pub const ARENA_TEST_BODY_SIZE: f32 = 1.0;

#[derive(Component)]
struct ArenaFloor;

#[derive(Component)]
struct ArenaWall;

#[derive(Component)]
struct ArenaTestBody;

/// Owns the static boundary and the small body used to exercise it.
struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
            .add_systems(Startup, spawn_physics_arena);
    }
}

fn spawn_sandbox_scene(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 5_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-1.0, -2.0, -1.5), Dir3::Y),
        Name::new("Sandbox Light"),
    ));
}

fn spawn_physics_arena(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let wall_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.16, 0.22, 0.32)));
    let body_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.86, 0.35, 0.2)));
    let wall_visuals = BoxVisuals {
        mesh: mesh.as_ref(),
        material: wall_material.as_ref(),
    };
    let body_visuals = BoxVisuals {
        mesh: mesh.as_ref(),
        material: body_material.as_ref(),
    };

    let floor_size = Vec3::new(
        ARENA_HALF_EXTENT * 2.0 + ARENA_WALL_THICKNESS * 2.0,
        ARENA_WALL_THICKNESS,
        ARENA_HALF_EXTENT * 2.0 + ARENA_WALL_THICKNESS * 2.0,
    );
    spawn_box(
        &mut commands,
        "Arena Floor",
        Vec3::new(0.0, -ARENA_WALL_THICKNESS * 0.5, 0.0),
        floor_size,
        RigidBody::Static,
        Some(ArenaFloor),
        wall_visuals,
    );

    let wall_center = ARENA_HALF_EXTENT + ARENA_WALL_THICKNESS * 0.5;
    let wall_size_x = Vec3::new(ARENA_WALL_THICKNESS, ARENA_WALL_HEIGHT, floor_size.z);
    let wall_size_z = Vec3::new(floor_size.x, ARENA_WALL_HEIGHT, ARENA_WALL_THICKNESS);
    for (name, position, size) in [
        (
            "Arena Wall +X",
            Vec3::new(wall_center, ARENA_WALL_HEIGHT * 0.5, 0.0),
            wall_size_x,
        ),
        (
            "Arena Wall -X",
            Vec3::new(-wall_center, ARENA_WALL_HEIGHT * 0.5, 0.0),
            wall_size_x,
        ),
        (
            "Arena Wall +Z",
            Vec3::new(0.0, ARENA_WALL_HEIGHT * 0.5, wall_center),
            wall_size_z,
        ),
        (
            "Arena Wall -Z",
            Vec3::new(0.0, ARENA_WALL_HEIGHT * 0.5, -wall_center),
            wall_size_z,
        ),
    ] {
        spawn_box(
            &mut commands,
            name,
            position,
            size,
            RigidBody::Static,
            Some(ArenaWall),
            wall_visuals,
        );
    }

    spawn_box(
        &mut commands,
        "Arena Test Body",
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::splat(ARENA_TEST_BODY_SIZE),
        RigidBody::Dynamic,
        Some(ArenaTestBody),
        body_visuals,
    );
}

#[derive(Clone, Copy)]
struct BoxVisuals<'a> {
    mesh: Option<&'a Handle<Mesh>>,
    material: Option<&'a Handle<StandardMaterial>>,
}

fn spawn_box(
    commands: &mut Commands,
    name: &str,
    position: Vec3,
    size: Vec3,
    body: RigidBody,
    marker: Option<impl Bundle>,
    visuals: BoxVisuals<'_>,
) -> Entity {
    let mut entity = commands.spawn((
        body,
        Collider::cuboid(size.x, size.y, size.z),
        Transform::from_translation(position),
        Name::new(name.to_owned()),
    ));
    if let Some(marker) = marker {
        entity.insert(marker);
    }
    if let (Some(mesh), Some(material)) = (visuals.mesh, visuals.material) {
        entity.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position).with_scale(size),
        ));
    }
    entity.id()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn arena_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            ArenaPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn arena_test_body(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<Entity, With<ArenaTestBody>>();
        bodies.iter(world).next().expect("arena test body")
    }

    fn run_physics_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    #[test]
    fn avian_plugins_initialize_physics_resources() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsPlugins::default()));

        assert!(app.world().contains_resource::<Gravity>());
        assert!(app.world().contains_resource::<Time<Physics>>());
    }

    #[test]
    fn sandbox_plugin_spawns_the_initial_3d_scene() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SandboxPlugin, CameraFollowPlugin));
        app.update();

        let world = app.world_mut();
        let mut cameras = world.query_filtered::<Entity, With<FixedFollowCamera>>();
        assert_eq!(cameras.iter(world).count(), 1);

        let mut lights = world.query_filtered::<Entity, With<DirectionalLight>>();
        assert_eq!(lights.iter(world).count(), 1);
    }

    #[test]
    fn arena_plugin_spawns_floor_walls_and_dynamic_test_body() {
        let mut app = arena_app();
        app.update();

        let world = app.world_mut();
        let mut floors = world.query_filtered::<Entity, With<ArenaFloor>>();
        assert_eq!(floors.iter(world).count(), 1);

        let mut walls = world.query_filtered::<Entity, With<ArenaWall>>();
        assert_eq!(walls.iter(world).count(), 4);

        let mut bodies = world.query_filtered::<Entity, (With<ArenaTestBody>, With<RigidBody>)>();
        assert_eq!(bodies.iter(world).count(), 1);
        assert_eq!(world.resource::<Gravity>().0, Vec3::new(0.0, -9.81, 0.0));
    }

    #[test]
    fn dynamic_test_body_falls_and_settles_on_the_floor() {
        let mut app = arena_app();
        app.update();
        let body = arena_test_body(&mut app);
        let initial_y = app.world().entity(body).get::<Position>().unwrap().0.y;

        run_physics_steps(&mut app, 180);

        let final_y = app.world().entity(body).get::<Position>().unwrap().0.y;
        assert!(
            final_y < initial_y - 1.0,
            "body did not fall: {initial_y} -> {final_y}"
        );
        assert!(
            (ARENA_TEST_BODY_SIZE * 0.5 - 0.1..=ARENA_TEST_BODY_SIZE * 0.5 + 0.1)
                .contains(&final_y),
            "body did not settle on the floor: {final_y}"
        );
    }

    #[test]
    fn dynamic_test_body_stays_inside_each_wall() {
        for direction in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z] {
            let mut app = arena_app();
            app.update();
            let body = arena_test_body(&mut app);
            app.world_mut().entity_mut(body).insert((
                Position(Vec3::new(0.0, 1.0, 0.0)),
                Transform::from_xyz(0.0, 1.0, 0.0),
                LinearVelocity(direction * 30.0),
            ));

            run_physics_steps(&mut app, 120);

            let final_position = app.world().entity(body).get::<Position>().unwrap().0;
            let distance_to_wall = direction.dot(final_position);
            assert!(
                distance_to_wall <= ARENA_HALF_EXTENT - ARENA_TEST_BODY_SIZE * 0.5 + 0.2,
                "body escaped toward {direction:?}: {final_position:?}"
            );
        }
    }
}
