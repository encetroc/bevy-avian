use avian3d::prelude::*;
use bevy::prelude::*;

use crate::collision_layers::{SandboxLayer, layers_for};

const RAY_COLOR: Color = Color::srgb(0.15, 0.85, 1.0);
const RAY_HIT_COLOR: Color = Color::srgb(1.0, 0.65, 0.15);
const RAY_NORMAL_COLOR: Color = Color::srgb(0.3, 1.0, 0.4);
const RAY_LENGTH: f32 = 4.5;
const RAY_NORMAL_LENGTH: f32 = 0.7;
const CASTER_POSITION: Vec3 = Vec3::new(-8.0, 1.0, 6.0);
const NEAR_SURFACE_POSITION: Vec3 = Vec3::new(-6.8, 1.0, 6.0);
const FAR_SURFACE_POSITION: Vec3 = Vec3::new(-4.8, 1.0, 6.0);
const SURFACE_SIZE: Vec3 = Vec3::new(0.35, 1.5, 1.0);

/// Marks the station G entity that owns the continuously active ray caster.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RayCasterDemo;

/// Marks one of the physical surfaces used by the ray caster demonstration.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RayCasterDemoSurface;

/// Owns station G's component-based ray casting demonstration.
pub struct RayCasterStationPlugin;

impl Plugin for RayCasterStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_raycaster_demo).add_systems(
            Update,
            draw_raycaster_demo.run_if(resource_exists::<GizmoConfigStore>),
        );
    }
}

fn spawn_raycaster_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let near_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.95, 0.35, 0.2)));
    let far_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.3, 0.7, 0.95)));

    commands.spawn((
        RayCasterDemo,
        RayCaster::new(Vec3::ZERO, Dir3::X)
            .with_max_distance(RAY_LENGTH)
            .with_max_hits(1),
        Transform::from_translation(CASTER_POSITION),
        Name::new("RayCaster Demo Probe"),
    ));

    spawn_surface(
        &mut commands,
        "RayCaster Near Surface",
        NEAR_SURFACE_POSITION,
        mesh.as_ref(),
        near_material.as_ref(),
    );
    spawn_surface(
        &mut commands,
        "RayCaster Far Surface",
        FAR_SURFACE_POSITION,
        mesh.as_ref(),
        far_material.as_ref(),
    );
}

fn spawn_surface(
    commands: &mut Commands,
    name: &'static str,
    position: Vec3,
    mesh: Option<&Handle<Mesh>>,
    material: Option<&Handle<StandardMaterial>>,
) -> Entity {
    let mut surface = commands.spawn((
        RayCasterDemoSurface,
        RigidBody::Static,
        layers_for(SandboxLayer::World),
        Collider::cuboid(SURFACE_SIZE.x, SURFACE_SIZE.y, SURFACE_SIZE.z),
        Transform::from_translation(position),
        Name::new(name),
    ));
    if let (Some(mesh), Some(material)) = (mesh, material) {
        surface.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position).with_scale(SURFACE_SIZE),
        ));
    }
    surface.id()
}

fn draw_raycaster_demo(
    mut gizmos: Gizmos,
    casters: Query<(&RayCaster, &RayHits), With<RayCasterDemo>>,
    names: Query<&Name>,
) {
    for (ray, hits) in &casters {
        let Some(hit) = hits.iter_sorted().next() else {
            gizmos.ray(
                ray.global_origin(),
                ray.global_direction() * RAY_LENGTH,
                RAY_COLOR,
            );
            gizmos.text(
                Isometry3d::new(
                    ray.get_global_point(RAY_LENGTH) + Vec3::Y * 0.35,
                    Quat::IDENTITY,
                ),
                "RAY CASTER\nNO HIT",
                0.3,
                Vec2::ZERO,
                RAY_COLOR,
            );
            continue;
        };

        let hit_position = ray.get_global_point(hit.distance);
        gizmos.ray(
            ray.global_origin(),
            ray.global_direction() * hit.distance,
            RAY_COLOR,
        );
        gizmos.cross(hit_position, 0.12, RAY_HIT_COLOR);
        gizmos.arrow(
            hit_position,
            hit_position + hit.normal * RAY_NORMAL_LENGTH,
            RAY_NORMAL_COLOR,
        );

        let name = names
            .get(hit.entity)
            .map(Name::as_str)
            .unwrap_or("Unknown collider");
        let label = format!(
            "RAY CASTER HIT\n{name}\nDistance: {:.2} m\nNormal: ({:.2}, {:.2}, {:.2})",
            hit.distance, hit.normal.x, hit.normal.y, hit.normal.z,
        );
        gizmos.text(
            Isometry3d::new(hit_position + Vec3::Y * 0.9, Quat::IDENTITY),
            &label,
            0.3,
            Vec2::ZERO,
            RAY_HIT_COLOR,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn raycaster_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            RayCasterStationPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn raycaster_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut casters = world.query_filtered::<Entity, With<RayCasterDemo>>();
        casters.iter(world).next().expect("ray caster demo")
    }

    fn first_hit(app: &mut App) -> Option<RayHitData> {
        let world = app.world_mut();
        let mut casters = world.query_filtered::<&RayHits, With<RayCasterDemo>>();
        casters
            .iter(world)
            .next()
            .and_then(|hits| hits.iter_sorted().next())
    }

    fn run_physics_frame(app: &mut App) {
        app.update();
        app.update();
    }

    #[test]
    fn station_spawns_an_enabled_raycaster_with_a_required_hit_buffer() {
        let mut app = raycaster_app();
        run_physics_frame(&mut app);

        let caster = raycaster_entity(&mut app);
        let entity = app.world().entity(caster);
        let ray = entity.get::<RayCaster>().expect("ray caster component");

        assert!(ray.enabled);
        assert_eq!(ray.max_hits, 1);
        assert!(entity.contains::<RayHits>());
        assert_eq!(
            entity.get::<Transform>().unwrap().translation,
            CASTER_POSITION
        );
    }

    #[test]
    fn raycaster_updates_when_its_entity_moves_across_surfaces() {
        let mut app = raycaster_app();
        run_physics_frame(&mut app);

        let caster = raycaster_entity(&mut app);
        let near_surface = app
            .world_mut()
            .query_filtered::<Entity, With<RayCasterDemoSurface>>()
            .iter(app.world())
            .next()
            .expect("near ray caster surface");
        assert_eq!(
            first_hit(&mut app).map(|hit| hit.entity),
            Some(near_surface)
        );

        app.world_mut()
            .entity_mut(caster)
            .insert(Transform::from_translation(Vec3::new(-6.2, 1.0, 6.0)));
        run_physics_frame(&mut app);

        let far_surface = app
            .world_mut()
            .query_filtered::<Entity, With<RayCasterDemoSurface>>()
            .iter(app.world())
            .find(|entity| *entity != near_surface)
            .expect("far ray caster surface");
        assert_eq!(first_hit(&mut app).map(|hit| hit.entity), Some(far_surface));

        app.world_mut()
            .entity_mut(caster)
            .insert(Transform::from_translation(Vec3::new(-4.0, 1.0, 6.0)));
        run_physics_frame(&mut app);
        assert!(first_hit(&mut app).is_none());
    }

    #[test]
    fn removing_all_targets_clears_the_continuously_updated_result() {
        let mut app = raycaster_app();
        run_physics_frame(&mut app);
        assert!(first_hit(&mut app).is_some());

        let targets: Vec<Entity> = {
            let world = app.world_mut();
            let mut surfaces = world.query_filtered::<Entity, With<RayCasterDemoSurface>>();
            surfaces.iter(world).collect()
        };
        for target in targets {
            app.world_mut().despawn(target);
        }

        run_physics_frame(&mut app);
        assert!(first_hit(&mut app).is_none());
    }
}
