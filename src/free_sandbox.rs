use avian3d::prelude::*;
use bevy::prelude::*;

use crate::collision_layers::{SandboxLayer, layers_for};

/// The extra north-south extent dedicated to the free experimentation area.
pub const FREE_SANDBOX_CENTER_Z: f32 = 15.0;

/// Adds a roomy, connected construction course beyond the original station grid.
pub struct FreeSandboxPlugin;

impl Plugin for FreeSandboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_free_sandbox);
    }
}

#[derive(Component)]
struct FreeSandboxStructure;

#[derive(Clone, Copy)]
struct StructureDefinition {
    name: &'static str,
    position: Vec3,
    size: Vec3,
    rotation_z: f32,
    color: Color,
}

const STRUCTURES: &[StructureDefinition] = &[
    StructureDefinition {
        name: "Free Sandbox Ramp",
        position: Vec3::new(-7.0, 0.37, FREE_SANDBOX_CENTER_Z),
        size: Vec3::new(4.0, 0.12, 3.0),
        rotation_z: 0.12,
        color: Color::srgb(0.22, 0.48, 0.58),
    },
    StructureDefinition {
        name: "Free Sandbox Stair 1",
        position: Vec3::new(-3.1, 0.15, FREE_SANDBOX_CENTER_Z),
        size: Vec3::new(0.7, 0.3, 3.0),
        rotation_z: 0.0,
        color: Color::srgb(0.30, 0.48, 0.34),
    },
    StructureDefinition {
        name: "Free Sandbox Stair 2",
        position: Vec3::new(-2.4, 0.3, FREE_SANDBOX_CENTER_Z),
        size: Vec3::new(0.7, 0.6, 3.0),
        rotation_z: 0.0,
        color: Color::srgb(0.30, 0.48, 0.34),
    },
    StructureDefinition {
        name: "Free Sandbox Stair 3",
        position: Vec3::new(-1.7, 0.45, FREE_SANDBOX_CENTER_Z),
        size: Vec3::new(0.7, 0.9, 3.0),
        rotation_z: 0.0,
        color: Color::srgb(0.30, 0.48, 0.34),
    },
    StructureDefinition {
        name: "Free Sandbox Stair 4",
        position: Vec3::new(-1.0, 0.6, FREE_SANDBOX_CENTER_Z),
        size: Vec3::new(0.7, 1.2, 3.0),
        rotation_z: 0.0,
        color: Color::srgb(0.30, 0.48, 0.34),
    },
    StructureDefinition {
        name: "Free Sandbox Platform",
        position: Vec3::new(0.65, 1.2, FREE_SANDBOX_CENTER_Z),
        size: Vec3::new(2.6, 0.4, 3.0),
        rotation_z: 0.0,
        color: Color::srgb(0.55, 0.37, 0.22),
    },
    StructureDefinition {
        name: "Free Sandbox Tabletop",
        position: Vec3::new(4.2, 0.9, FREE_SANDBOX_CENTER_Z - 2.0),
        size: Vec3::new(3.0, 0.2, 2.0),
        rotation_z: 0.0,
        color: Color::srgb(0.64, 0.39, 0.20),
    },
    StructureDefinition {
        name: "Free Sandbox Table Leg 1",
        position: Vec3::new(3.0, 0.4, FREE_SANDBOX_CENTER_Z - 2.8),
        size: Vec3::new(0.2, 0.8, 0.2),
        rotation_z: 0.0,
        color: Color::srgb(0.45, 0.27, 0.15),
    },
    StructureDefinition {
        name: "Free Sandbox Table Leg 2",
        position: Vec3::new(5.4, 0.4, FREE_SANDBOX_CENTER_Z - 2.8),
        size: Vec3::new(0.2, 0.8, 0.2),
        rotation_z: 0.0,
        color: Color::srgb(0.45, 0.27, 0.15),
    },
    StructureDefinition {
        name: "Free Sandbox Table Leg 3",
        position: Vec3::new(3.0, 0.4, FREE_SANDBOX_CENTER_Z - 1.2),
        size: Vec3::new(0.2, 0.8, 0.2),
        rotation_z: 0.0,
        color: Color::srgb(0.45, 0.27, 0.15),
    },
    StructureDefinition {
        name: "Free Sandbox Table Leg 4",
        position: Vec3::new(5.4, 0.4, FREE_SANDBOX_CENTER_Z - 1.2),
        size: Vec3::new(0.2, 0.8, 0.2),
        rotation_z: 0.0,
        color: Color::srgb(0.45, 0.27, 0.15),
    },
    StructureDefinition {
        name: "Free Sandbox Shelf Lower",
        position: Vec3::new(8.0, 0.55, FREE_SANDBOX_CENTER_Z + 1.3),
        size: Vec3::new(3.0, 0.2, 2.2),
        rotation_z: 0.0,
        color: Color::srgb(0.62, 0.48, 0.26),
    },
    StructureDefinition {
        name: "Free Sandbox Shelf Upper",
        position: Vec3::new(8.0, 1.55, FREE_SANDBOX_CENTER_Z + 1.3),
        size: Vec3::new(3.0, 0.2, 2.2),
        rotation_z: 0.0,
        color: Color::srgb(0.62, 0.48, 0.26),
    },
    StructureDefinition {
        name: "Free Sandbox Shelf Back",
        position: Vec3::new(8.0, 1.1, FREE_SANDBOX_CENTER_Z + 2.35),
        size: Vec3::new(3.0, 2.2, 0.15),
        rotation_z: 0.0,
        color: Color::srgb(0.48, 0.34, 0.20),
    },
    StructureDefinition {
        name: "Free Sandbox Shelf Side Left",
        position: Vec3::new(6.55, 1.1, FREE_SANDBOX_CENTER_Z + 1.3),
        size: Vec3::new(0.15, 2.2, 2.2),
        rotation_z: 0.0,
        color: Color::srgb(0.48, 0.34, 0.20),
    },
    StructureDefinition {
        name: "Free Sandbox Shelf Side Right",
        position: Vec3::new(9.45, 1.1, FREE_SANDBOX_CENTER_Z + 1.3),
        size: Vec3::new(0.15, 2.2, 2.2),
        rotation_z: 0.0,
        color: Color::srgb(0.48, 0.34, 0.20),
    },
    StructureDefinition {
        name: "Free Sandbox Divider Wall",
        position: Vec3::new(-8.0, 1.0, FREE_SANDBOX_CENTER_Z + 4.0),
        size: Vec3::new(3.5, 2.0, 0.25),
        rotation_z: 0.0,
        color: Color::srgb(0.32, 0.38, 0.46),
    },
];

fn spawn_free_sandbox(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for structure in STRUCTURES {
        let material = materials
            .as_mut()
            .map(|materials| materials.add(structure.color));
        let transform = Transform::from_translation(structure.position)
            .with_rotation(Quat::from_rotation_z(structure.rotation_z));
        let mut entity = commands.spawn((
            FreeSandboxStructure,
            RigidBody::Static,
            Collider::cuboid(structure.size.x, structure.size.y, structure.size.z),
            layers_for(SandboxLayer::World),
            transform,
            Name::new(structure.name),
        ));
        if let (Some(meshes), Some(material)) = (meshes.as_deref_mut(), material) {
            entity.insert((
                Mesh3d(meshes.add(Cuboid::from_size(structure.size))),
                MeshMaterial3d(material),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use crate::{
        ArenaPlugin,
        player::{Player, PlayerPlugin},
    };

    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    fn sandbox_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            FreeSandboxPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            STEP,
        )));
        app.finish();
        app
    }

    #[test]
    fn free_sandbox_spawns_collidable_static_environment_features() {
        let mut app = sandbox_app();
        app.update();

        let world = app.world_mut();
        let mut structures = world.query::<(&Name, &RigidBody, &Collider, &Transform)>();
        let found: Vec<_> = structures
            .iter(world)
            .map(|(name, body, _, transform)| {
                assert_eq!(*body, RigidBody::Static, "{}", name);
                assert!(transform.scale.min_element() > 0.0, "{}", name);
                name.as_str().to_owned()
            })
            .filter(|name| name.starts_with("Free Sandbox "))
            .collect();

        for feature in [
            "Ramp",
            "Stair",
            "Platform",
            "Tabletop",
            "Shelf",
            "Divider Wall",
        ] {
            assert!(
                found.iter().any(|name| name.contains(feature)),
                "missing collidable {feature}; got {found:?}"
            );
        }
        assert_eq!(found.len(), STRUCTURES.len());
    }

    #[test]
    fn dropped_objects_land_on_the_ramp_stairs_platform_table_and_shelf() {
        let mut app = sandbox_app();
        app.add_systems(Startup, spawn_support_test_bodies);
        app.update();

        for _ in 0..180 {
            app.update();
        }

        let world = app.world_mut();
        let mut bodies = world.query::<(&SupportTestObject, &Position)>();
        for (object, position) in bodies.iter(world) {
            let expected_height = match object {
                SupportTestObject::Ramp => 0.68,
                SupportTestObject::Stairs => 1.45,
                SupportTestObject::Platform => 1.65,
                SupportTestObject::Table => 1.25,
                SupportTestObject::Shelf => 1.9,
            };
            assert!(
                (expected_height - 0.15..=expected_height + 0.15).contains(&position.0.y),
                "{object:?} object did not rest on its support: {:?}",
                position.0
            );
        }
        assert_eq!(bodies.iter(world).count(), 5);
    }

    #[test]
    fn player_can_walk_through_the_full_free_sandbox_course() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            ArenaPlugin,
            FreeSandboxPlugin,
            PlayerPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            STEP,
        )));
        app.finish();
        app.update();

        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<Player>>();
        let player = players.single(world).unwrap();
        let start = Vec3::new(-9.6, 1.0, FREE_SANDBOX_CENTER_Z);
        let mut entity = world.entity_mut(player);
        entity.get_mut::<Transform>().unwrap().translation = start;
        entity.get_mut::<Position>().unwrap().0 = start;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);

        for _ in 0..240 {
            app.update();
        }

        let final_position = app
            .world()
            .entity(player)
            .get::<Transform>()
            .unwrap()
            .translation;
        assert!(
            final_position.x > 0.5,
            "player could not traverse the ramp and stairs: {final_position:?}"
        );
        assert!(
            (0.8..=1.1).contains(&final_position.y),
            "player left the walkable floor while crossing the course: {final_position:?}"
        );
    }

    #[derive(Component, Debug)]
    enum SupportTestObject {
        Ramp,
        Stairs,
        Platform,
        Table,
        Shelf,
    }

    fn spawn_support_test_bodies(mut commands: Commands) {
        for (object, position) in [
            (
                SupportTestObject::Ramp,
                Vec3::new(-7.0, 4.0, FREE_SANDBOX_CENTER_Z),
            ),
            (
                SupportTestObject::Stairs,
                Vec3::new(-1.0, 4.0, FREE_SANDBOX_CENTER_Z),
            ),
            (
                SupportTestObject::Platform,
                Vec3::new(0.65, 4.0, FREE_SANDBOX_CENTER_Z),
            ),
            (
                SupportTestObject::Table,
                Vec3::new(4.2, 4.0, FREE_SANDBOX_CENTER_Z - 2.0),
            ),
            (
                SupportTestObject::Shelf,
                Vec3::new(8.0, 4.0, FREE_SANDBOX_CENTER_Z + 1.3),
            ),
        ] {
            commands.spawn((
                object,
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                Transform::from_translation(position),
            ));
        }
    }
}
