use std::fs;

use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    breakable_objects::BreakableObject,
    gameplay_materials::{BreakThreshold, GameplayMaterial, WindResponse},
    liquid_containers::LiquidContainer,
    object_spawn_palette::{SpawnPreset, SpawnedSandboxObject},
    stations::StationObject,
};

pub const SNAPSHOT_PATH: &str = "sandbox-snapshot.json";

/// Snapshot schema intentionally contains data rather than Bevy entity IDs or asset handles.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct SandboxSnapshot {
    version: u32,
    objects: Vec<SandboxObjectSnapshot>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct SandboxObjectSnapshot {
    preset: SpawnPreset,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
    linear_velocity: [f32; 3],
    angular_velocity: [f32; 3],
    color: [f32; 4],
    gameplay_material: Option<GameplayMaterial>,
    breakable: bool,
    collider_density: Option<f32>,
    friction: Option<f32>,
    restitution: Option<f32>,
    break_threshold: Option<f32>,
    wind_response: Option<f32>,
    liquid_capacity: Option<f32>,
    liquid_amount: Option<f32>,
}

#[derive(Resource, Default)]
struct SnapshotStatus(String);

/// Saves and reloads palette-spawned free-sandbox objects with F5 and F9.
pub struct SandboxSerializationPlugin;

impl Plugin for SandboxSerializationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnapshotStatus>()
            .add_systems(Update, handle_snapshot_keys);
    }
}

fn handle_snapshot_keys(world: &mut World) {
    let keyboard = world.resource::<ButtonInput<KeyCode>>();
    let save = keyboard.just_pressed(KeyCode::F5);
    let load = keyboard.just_pressed(KeyCode::F9);
    if save {
        let result = snapshot_json(world)
            .map_err(|error| error.to_string())
            .and_then(|json| fs::write(SNAPSHOT_PATH, json).map_err(|error| error.to_string()));
        world.resource_mut::<SnapshotStatus>().0 = match result {
            Ok(()) => format!("Saved sandbox snapshot to {SNAPSHOT_PATH}"),
            Err(error) => format!("Could not save sandbox snapshot: {error}"),
        };
    } else if load {
        let result = fs::read_to_string(SNAPSHOT_PATH)
            .map_err(|error| error.to_string())
            .and_then(|json| restore_json(world, &json).map_err(|error| error.to_string()));
        world.resource_mut::<SnapshotStatus>().0 = match result {
            Ok(()) => format!("Loaded sandbox snapshot from {SNAPSHOT_PATH}"),
            Err(error) => format!("Could not load sandbox snapshot: {error}"),
        };
    }
}

fn snapshot_json(world: &mut World) -> Result<String, serde_json::Error> {
    let snapshot = capture_snapshot(world);
    serde_json::to_string_pretty(&snapshot)
}

fn capture_snapshot(world: &mut World) -> SandboxSnapshot {
    let mut query = world.query_filtered::<(
        &SpawnedSandboxObject,
        &Transform,
        Option<&LinearVelocity>,
        Option<&AngularVelocity>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&GameplayMaterial>,
        Option<&BreakableObject>,
        Option<&ColliderDensity>,
        Option<&Friction>,
        Option<&Restitution>,
        Option<&BreakThreshold>,
        Option<&WindResponse>,
        Option<&LiquidContainer>,
    ), With<SpawnedSandboxObject>>();
    let materials = world.get_resource::<Assets<StandardMaterial>>();
    let objects = query
        .iter(world)
        .map(
            |(
                spawned,
                transform,
                linear_velocity,
                angular_velocity,
                material_handle,
                gameplay_material,
                breakable,
                density,
                friction,
                restitution,
                break_threshold,
                wind_response,
                container,
            )| {
                let material_color = material_handle
                    .and_then(|handle| materials.and_then(|assets| assets.get(&handle.0)))
                    .map(|material| material.base_color.to_srgba())
                    .unwrap_or_else(|| spawned.preset.color().to_srgba());
                SandboxObjectSnapshot {
                    preset: spawned.preset,
                    translation: transform.translation.to_array(),
                    rotation: transform.rotation.to_array(),
                    scale: transform.scale.to_array(),
                    linear_velocity: linear_velocity.map_or(Vec3::ZERO, |v| v.0).to_array(),
                    angular_velocity: angular_velocity.map_or(Vec3::ZERO, |v| v.0).to_array(),
                    color: [
                        material_color.red,
                        material_color.green,
                        material_color.blue,
                        material_color.alpha,
                    ],
                    gameplay_material: gameplay_material.copied(),
                    breakable: breakable.is_some(),
                    collider_density: density.map(|value| value.0),
                    friction: friction.map(|value| value.dynamic_coefficient),
                    restitution: restitution.map(|value| value.coefficient),
                    break_threshold: break_threshold.map(|value| value.0),
                    wind_response: wind_response.map(|value| value.0),
                    liquid_capacity: container.map(|container| container.capacity()),
                    liquid_amount: container.map(|container| container.amount()),
                }
            },
        )
        .collect();
    SandboxSnapshot {
        version: 1,
        objects,
    }
}

fn restore_json(world: &mut World, json: &str) -> Result<(), String> {
    let snapshot: SandboxSnapshot =
        serde_json::from_str(json).map_err(|error| error.to_string())?;
    validate_snapshot(&snapshot)?;

    let mut existing = world.query_filtered::<Entity, With<SpawnedSandboxObject>>();
    let entities: Vec<_> = existing.iter(world).collect();
    for entity in entities {
        world.despawn(entity);
    }

    for object in snapshot.objects {
        let transform = Transform {
            translation: Vec3::from_array(object.translation),
            rotation: Quat::from_array(object.rotation).normalize(),
            scale: Vec3::from_array(object.scale),
        };
        let mesh = world
            .get_resource_mut::<Assets<Mesh>>()
            .map(|mut assets| Mesh3d(assets.add(object.preset.mesh())));
        let color = Color::srgba(
            object.color[0],
            object.color[1],
            object.color[2],
            object.color[3],
        );
        let material = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .map(|mut assets| MeshMaterial3d(assets.add(color)));
        let mut entity = world.spawn((
            SpawnedSandboxObject {
                preset: object.preset,
            },
            StationObject::new('J', transform),
            RigidBody::Dynamic,
            object.preset.collider(),
            Name::new(format!("Spawned {}", object.preset.label())),
            transform,
            Position(transform.translation),
            Rotation(transform.rotation),
            LinearVelocity(Vec3::from_array(object.linear_velocity)),
            AngularVelocity(Vec3::from_array(object.angular_velocity)),
        ));
        if let Some(value) = object.collider_density {
            entity.insert(ColliderDensity(value));
        }
        if let Some(value) = object.friction {
            entity.insert(Friction::new(value));
        }
        if let Some(value) = object.restitution {
            entity.insert(Restitution::new(value));
        }
        if let Some(material) = object.gameplay_material {
            entity.insert(material);
        }
        if object.breakable {
            entity.insert(BreakableObject);
        }
        if let Some(value) = object.break_threshold {
            entity.insert(BreakThreshold(value));
        }
        if let Some(value) = object.wind_response {
            entity.insert(WindResponse(value));
        }
        if let (Some(capacity), Some(amount)) = (object.liquid_capacity, object.liquid_amount) {
            entity.insert(LiquidContainer::new(capacity, amount));
        }
        if let Some(mesh) = mesh {
            entity.insert(mesh);
        }
        if let Some(material) = material {
            entity.insert(material);
        }
    }
    Ok(())
}

fn validate_snapshot(snapshot: &SandboxSnapshot) -> Result<(), String> {
    if snapshot.version != 1 {
        return Err(format!("unsupported snapshot version {}", snapshot.version));
    }
    for (index, object) in snapshot.objects.iter().enumerate() {
        let numeric_values = object
            .translation
            .into_iter()
            .chain(object.rotation)
            .chain(object.scale)
            .chain(object.linear_velocity)
            .chain(object.angular_velocity)
            .chain(object.color)
            .chain([
                object.collider_density.unwrap_or(0.0),
                object.friction.unwrap_or(0.0),
                object.restitution.unwrap_or(0.0),
                object.break_threshold.unwrap_or(0.0),
                object.wind_response.unwrap_or(0.0),
                object.liquid_capacity.unwrap_or(0.0),
                object.liquid_amount.unwrap_or(0.0),
            ]);
        if numeric_values.into_iter().any(|value| !value.is_finite()) {
            return Err(format!("object {index} contains a non-finite value"));
        }
        if object.scale.into_iter().any(|value| value <= 0.0)
            || object
                .rotation
                .iter()
                .map(|value| value * value)
                .sum::<f32>()
                <= f32::EPSILON
        {
            return Err(format!("object {index} has an invalid transform"));
        }
        if object.liquid_capacity.is_some() != object.liquid_amount.is_some() {
            return Err(format!("object {index} has incomplete container state"));
        }
        if object
            .liquid_capacity
            .is_some_and(|capacity| capacity <= 0.0)
            || object
                .liquid_amount
                .zip(object.liquid_capacity)
                .is_some_and(|(amount, capacity)| !(0.0..=capacity).contains(&amount))
        {
            return Err(format!("object {index} has invalid container state"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};
    use std::time::Duration;

    use super::*;
    use crate::gameplay_materials::GameplayMaterialsPlugin;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            GameplayMaterialsPlugin,
        ));
        app.init_resource::<Assets<StandardMaterial>>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / 60.0,
            )));
        app.finish();
        app
    }

    #[test]
    fn mixed_sandbox_snapshot_restores_type_transform_material_and_container_state() {
        let mut app = app();
        let cube_transform = Transform::from_xyz(2.0, 3.0, 4.0)
            .with_rotation(Quat::from_rotation_y(0.7))
            .with_scale(Vec3::splat(1.2));
        let custom_color = Color::srgb(0.13, 0.27, 0.81);
        let custom_material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(custom_color);
        let cube = app
            .world_mut()
            .spawn((
                SpawnedSandboxObject {
                    preset: SpawnPreset::Cube,
                },
                StationObject::new('J', cube_transform),
                RigidBody::Dynamic,
                SpawnPreset::Cube.collider(),
                ColliderDensity(3.2),
                Friction::new(0.72),
                Restitution::new(0.31),
                GameplayMaterial::Wood,
                BreakableObject,
                BreakThreshold(42.0),
                WindResponse(1.4),
                MeshMaterial3d(custom_material),
                LinearVelocity(Vec3::new(1.0, 2.0, 3.0)),
                AngularVelocity(Vec3::new(0.1, 0.2, 0.3)),
                cube_transform,
            ))
            .id();
        let barrel = app
            .world_mut()
            .spawn((
                SpawnedSandboxObject {
                    preset: SpawnPreset::Barrel,
                },
                StationObject::new('J', Transform::from_xyz(-2.0, 5.0, 1.0)),
                RigidBody::Dynamic,
                SpawnPreset::Barrel.collider(),
                LiquidContainer::new(80.0, 23.5),
                Transform::from_xyz(-2.0, 5.0, 1.0),
            ))
            .id();
        app.update();
        app.world_mut().entity_mut(cube).insert((
            ColliderDensity(3.2),
            Friction::new(0.72),
            Restitution::new(0.31),
            BreakThreshold(42.0),
            WindResponse(1.4),
        ));

        let json = snapshot_json(app.world_mut()).unwrap();
        let path =
            std::env::temp_dir().join(format!("sandbox-snapshot-{}.json", std::process::id()));
        fs::write(&path, json).unwrap();
        let json = fs::read_to_string(&path).unwrap();
        let saved = serde_json::from_str::<SandboxSnapshot>(&json).unwrap();
        assert_eq!(saved.objects.len(), 2);
        assert_eq!(
            saved
                .objects
                .iter()
                .find(|o| o.preset == SpawnPreset::Cube)
                .unwrap()
                .gameplay_material,
            Some(GameplayMaterial::Wood)
        );
        assert_eq!(
            saved
                .objects
                .iter()
                .find(|o| o.preset == SpawnPreset::Barrel)
                .unwrap()
                .liquid_amount,
            Some(23.5)
        );

        app.world_mut()
            .entity_mut(cube)
            .insert(Transform::from_xyz(90.0, 80.0, 70.0));
        app.world_mut()
            .entity_mut(barrel)
            .get_mut::<LiquidContainer>()
            .unwrap()
            .set_amount(75.0);
        app.world_mut().spawn((
            SpawnedSandboxObject {
                preset: SpawnPreset::Ball,
            },
            Transform::IDENTITY,
        ));
        restore_json(app.world_mut(), &json).unwrap();

        let world = app.world_mut();
        let mut objects = world.query::<(&SpawnedSandboxObject, &Transform)>();
        let restored: Vec<_> = objects
            .iter(world)
            .map(|(preset, transform)| (*preset, *transform))
            .collect();
        assert_eq!(restored.len(), 2);
        let world = app.world_mut();
        let mut cubes = world.query_filtered::<(Entity, &SpawnedSandboxObject, &Transform), With<SpawnedSandboxObject>>();
        let (restored_cube, _, transform) = cubes
            .iter(world)
            .find(|(_, spawned, _)| spawned.preset == SpawnPreset::Cube)
            .unwrap();
        assert_eq!(*transform, cube_transform);
        let cube_entity = app.world().entity(restored_cube);
        assert_eq!(
            cube_entity.get::<LinearVelocity>(),
            Some(&LinearVelocity(Vec3::new(1.0, 2.0, 3.0)))
        );
        assert_eq!(
            cube_entity.get::<ColliderDensity>(),
            Some(&ColliderDensity(3.2))
        );
        assert_eq!(
            cube_entity.get::<GameplayMaterial>(),
            Some(&GameplayMaterial::Wood)
        );
        assert_eq!(
            cube_entity.get::<BreakThreshold>(),
            Some(&BreakThreshold(42.0))
        );
        assert_eq!(
            cube_entity.get::<Friction>().unwrap().dynamic_coefficient,
            0.72
        );
        assert_eq!(cube_entity.get::<Restitution>().unwrap().coefficient, 0.31);
        assert_eq!(cube_entity.get::<WindResponse>(), Some(&WindResponse(1.4)));
        assert!(cube_entity.contains::<BreakableObject>());
        let color_handle = &cube_entity
            .get::<MeshMaterial3d<StandardMaterial>>()
            .unwrap()
            .0;
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(color_handle)
                .unwrap()
                .base_color,
            custom_color
        );
        let barrel_entity = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<SpawnedSandboxObject>()
                    .is_some_and(|spawned| spawned.preset == SpawnPreset::Barrel)
            })
            .unwrap();
        assert_eq!(
            barrel_entity.get::<LiquidContainer>(),
            Some(&LiquidContainer::new(80.0, 23.5))
        );
        assert!(
            barrel_entity
                .get::<Collider>()
                .unwrap()
                .shape()
                .as_cylinder()
                .is_some()
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_snapshot_does_not_clear_existing_objects() {
        let mut app = app();
        let entity = app
            .world_mut()
            .spawn((
                SpawnedSandboxObject {
                    preset: SpawnPreset::Cube,
                },
                Transform::IDENTITY,
            ))
            .id();
        let error = restore_json(app.world_mut(), r#"{"version":2,"objects":[]}"#).unwrap_err();
        assert!(error.contains("unsupported snapshot version"));
        assert!(app.world().get_entity(entity).is_ok());
    }
}
