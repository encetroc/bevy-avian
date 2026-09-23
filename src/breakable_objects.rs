use std::collections::HashSet;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    collision_layers::{SandboxLayer, layers_for},
    gameplay_materials::{BreakThreshold, GameplayMaterial},
    object_spawn_palette::{SpawnPreset, SpawnedSandboxObject},
    stations::StationObject,
};

/// Marks a material body that is replaced by debris after a sufficiently strong impact.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BreakableObject;

/// Marks ordinary dynamic debris created when a breakable body is destroyed.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BreakageFragment;

/// Owns impact-threshold breakage for breakable free-sandbox objects.
pub struct BreakableObjectsPlugin;

impl Plugin for BreakableObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, break_on_impact);
    }
}

#[derive(Clone, Copy)]
struct FragmentDefinition {
    offset: Vec3,
    size: Vec3,
}

const CERAMIC_FRAGMENTS: [FragmentDefinition; 4] = [
    FragmentDefinition {
        offset: Vec3::new(-0.22, 0.18, -0.2),
        size: Vec3::new(0.38, 0.38, 0.38),
    },
    FragmentDefinition {
        offset: Vec3::new(0.22, 0.18, -0.2),
        size: Vec3::new(0.38, 0.38, 0.38),
    },
    FragmentDefinition {
        offset: Vec3::new(-0.22, -0.18, 0.2),
        size: Vec3::new(0.38, 0.38, 0.38),
    },
    FragmentDefinition {
        offset: Vec3::new(0.22, -0.18, 0.2),
        size: Vec3::new(0.38, 0.38, 0.38),
    },
];
const WOOD_FRAGMENTS: [FragmentDefinition; 3] = [
    FragmentDefinition {
        offset: Vec3::new(-0.38, 0.0, 0.0),
        size: Vec3::new(0.42, 0.48, 0.62),
    },
    FragmentDefinition {
        offset: Vec3::new(0.0, 0.0, 0.0),
        size: Vec3::new(0.42, 0.48, 0.62),
    },
    FragmentDefinition {
        offset: Vec3::new(0.38, 0.0, 0.0),
        size: Vec3::new(0.42, 0.48, 0.62),
    },
];
const STONE_FRAGMENTS: [FragmentDefinition; 4] = [
    FragmentDefinition {
        offset: Vec3::new(-0.2, 0.2, -0.2),
        size: Vec3::new(0.4, 0.4, 0.4),
    },
    FragmentDefinition {
        offset: Vec3::new(0.2, 0.2, 0.2),
        size: Vec3::new(0.4, 0.4, 0.4),
    },
    FragmentDefinition {
        offset: Vec3::new(-0.2, -0.2, 0.2),
        size: Vec3::new(0.4, 0.4, 0.4),
    },
    FragmentDefinition {
        offset: Vec3::new(0.2, -0.2, -0.2),
        size: Vec3::new(0.4, 0.4, 0.4),
    },
];

fn fragments(material: GameplayMaterial) -> &'static [FragmentDefinition] {
    match material {
        GameplayMaterial::Ceramic => &CERAMIC_FRAGMENTS,
        GameplayMaterial::Wood => &WOOD_FRAGMENTS,
        GameplayMaterial::Stone => &STONE_FRAGMENTS,
        GameplayMaterial::Metal | GameplayMaterial::Glass => &[],
    }
}

/// Contact impulses are converted to application force units before comparing with material defaults.
fn impact_exceeds_threshold(impulse: f32, step_seconds: f32, threshold: f32) -> bool {
    step_seconds.is_finite()
        && step_seconds > 0.0
        && impulse.is_finite()
        && impulse.max(0.0) / step_seconds > threshold
}

fn claim_break(
    entity: Entity,
    impulse: f32,
    step_seconds: f32,
    threshold: f32,
    already_broken: &mut HashSet<Entity>,
) -> bool {
    impact_exceeds_threshold(impulse, step_seconds, threshold) && already_broken.insert(entity)
}

#[expect(
    clippy::type_complexity,
    reason = "the query captures all data needed to replace one breakable body"
)]
fn break_on_impact(
    mut commands: Commands,
    contact_graph: Res<ContactGraph>,
    physics_time: Res<Time<Physics>>,
    breakables: Query<
        (
            Entity,
            &GameplayMaterial,
            &BreakThreshold,
            &Transform,
            Option<&LinearVelocity>,
            Option<&StationObject>,
            Option<&SpawnedSandboxObject>,
        ),
        (With<BreakableObject>, Without<BreakageFragment>),
    >,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let step_seconds = physics_time.delta_secs();
    if step_seconds <= 0.0 {
        return;
    }

    let mut already_broken = HashSet::new();
    for pair in contact_graph.iter_active_touching() {
        let impulse = pair.total_normal_impulse_magnitude();
        if impulse <= 0.0 {
            continue;
        }
        let participants = [
            pair.body1.unwrap_or(pair.collider1),
            pair.body2.unwrap_or(pair.collider2),
        ];
        for entity in participants {
            let Ok((entity, material_kind, threshold, transform, velocity, station, spawned)) =
                breakables.get(entity)
            else {
                continue;
            };
            if !claim_break(
                entity,
                impulse,
                step_seconds,
                threshold.0,
                &mut already_broken,
            ) {
                continue;
            }
            let preset = spawned.map_or_else(
                || match material_kind {
                    GameplayMaterial::Ceramic => SpawnPreset::Ceramic,
                    GameplayMaterial::Wood => SpawnPreset::Wood,
                    GameplayMaterial::Stone => SpawnPreset::Stone,
                    GameplayMaterial::Metal => SpawnPreset::HeavyBlock,
                    GameplayMaterial::Glass => SpawnPreset::Cube,
                },
                |spawned| spawned.preset,
            );
            spawn_fragments(
                &mut commands,
                meshes.as_deref_mut(),
                materials.as_deref_mut(),
                *material_kind,
                transform,
                velocity.map_or(Vec3::ZERO, |velocity| velocity.0),
                station.map_or('J', |station| station.station),
                preset,
            );
            commands.entity(entity).despawn();
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "fragment spawning receives the source body state and optional rendering assets"
)]
fn spawn_fragments(
    commands: &mut Commands,
    mut meshes: Option<&mut Assets<Mesh>>,
    materials: Option<&mut Assets<StandardMaterial>>,
    material_kind: GameplayMaterial,
    source_transform: &Transform,
    source_velocity: Vec3,
    station: char,
    preset: SpawnPreset,
) {
    let defaults = material_kind.defaults();
    let color = match material_kind {
        GameplayMaterial::Ceramic => Color::srgb(0.88, 0.85, 0.78),
        GameplayMaterial::Wood => Color::srgb(0.55, 0.31, 0.12),
        GameplayMaterial::Stone => Color::srgb(0.45, 0.48, 0.52),
        GameplayMaterial::Metal => Color::srgb(0.48, 0.52, 0.6),
        GameplayMaterial::Glass => Color::srgb(0.65, 0.85, 0.9),
    };
    let shared_material = materials.map(|materials| materials.add(color));

    for (index, fragment) in fragments(material_kind).iter().enumerate() {
        let rotation = source_transform.rotation;
        let position = source_transform.translation + rotation * fragment.offset;
        let transform = Transform::from_translation(position).with_rotation(rotation);
        let mesh = meshes
            .as_deref_mut()
            .map(|meshes| meshes.add(Cuboid::from_size(fragment.size)));
        let mut entity = commands.spawn((
            BreakageFragment,
            SpawnedSandboxObject { preset },
            StationObject::new(station, transform),
            RigidBody::Dynamic,
            layers_for(SandboxLayer::Objects),
            Collider::cuboid(fragment.size.x, fragment.size.y, fragment.size.z),
            ColliderDensity(defaults.density),
            Friction::new(defaults.friction),
            Restitution::new(defaults.restitution),
            LinearVelocity(
                source_velocity
                    + Vec3::new(
                        (index as f32 - 1.5) * 0.6,
                        1.0 + (index % 2) as f32 * 0.3,
                        (1.5 - index as f32) * 0.6,
                    ),
            ),
            transform,
            Name::new(format!("{} fragment", material_kind_name(material_kind))),
        ));
        if let Some(mesh) = mesh {
            entity.insert(Mesh3d(mesh));
        }
        if let Some(material) = shared_material.clone() {
            entity.insert(MeshMaterial3d(material));
        }
    }
}

fn material_kind_name(material: GameplayMaterial) -> &'static str {
    match material {
        GameplayMaterial::Ceramic => "Ceramic",
        GameplayMaterial::Wood => "Wood",
        GameplayMaterial::Stone => "Stone",
        GameplayMaterial::Metal => "Metal",
        GameplayMaterial::Glass => "Glass",
    }
}

#[cfg(test)]
mod tests {
    use bevy::mesh::MeshPlugin;

    use super::*;

    fn spawn_test_fragments(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (material, preset) in [
            (GameplayMaterial::Ceramic, SpawnPreset::Ceramic),
            (GameplayMaterial::Wood, SpawnPreset::Wood),
            (GameplayMaterial::Stone, SpawnPreset::Stone),
        ] {
            spawn_fragments(
                &mut commands,
                Some(&mut meshes),
                Some(&mut materials),
                material,
                &Transform::from_xyz(0.0, 4.0, 0.0),
                Vec3::ZERO,
                'J',
                preset,
            );
        }
    }

    #[test]
    fn impacts_below_and_above_each_breakable_material_threshold() {
        let entity = Entity::PLACEHOLDER;
        let mut already_broken = HashSet::new();
        for material in [
            GameplayMaterial::Ceramic,
            GameplayMaterial::Wood,
            GameplayMaterial::Stone,
        ] {
            let threshold = material.defaults().break_threshold;
            assert!(!claim_break(
                entity,
                threshold * 0.9,
                1.0,
                threshold,
                &mut already_broken,
            ));
            assert!(impact_exceeds_threshold(threshold * 1.1, 1.0, threshold));
        }
        assert!(claim_break(entity, 20.0, 1.0, 10.0, &mut already_broken));
        assert!(!claim_break(entity, 20.0, 1.0, 10.0, &mut already_broken));
        assert!(!impact_exceeds_threshold(10.0, 0.0, 1.0));
    }

    #[test]
    fn predefined_fragments_are_dynamic_physical_bodies_and_not_breakable() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), MeshPlugin));
        app.init_resource::<Assets<StandardMaterial>>()
            .add_systems(Startup, spawn_test_fragments);
        app.finish();
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<BreakageFragment>>();
        let fragments: Vec<_> = query.iter(world).collect();
        assert_eq!(
            fragments.len(),
            CERAMIC_FRAGMENTS.len() + WOOD_FRAGMENTS.len() + STONE_FRAGMENTS.len()
        );
        for entity in fragments {
            let fragment = app.world().entity(entity);
            assert_eq!(fragment.get::<RigidBody>(), Some(&RigidBody::Dynamic));
            assert!(fragment.contains::<Collider>());
            assert!(fragment.contains::<LinearVelocity>());
            assert!(!fragment.contains::<BreakableObject>());
        }
    }
}
