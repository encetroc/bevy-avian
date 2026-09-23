use avian3d::prelude::*;
use bevy::prelude::*;

use crate::collision_layers::{SandboxLayer, layers_for};

const CAST_COLOR: Color = Color::srgb(0.15, 0.85, 1.0);
const HIT_COLOR: Color = Color::srgb(1.0, 0.65, 0.15);
const NORMAL_COLOR: Color = Color::srgb(0.3, 1.0, 0.4);
const EMPTY_COLOR: Color = Color::srgb(0.55, 0.65, 0.75);
const PROBE_COLOR: Color = Color::srgba(0.25, 0.75, 1.0, 0.9);
const PROBE_RADIUS: f32 = 0.42;
const CAST_DISTANCE: f32 = 4.2;
const PROBE_START: Vec3 = Vec3::new(-8.0, 1.0, 3.0);
const OBSTACLE_POSITION: Vec3 = Vec3::new(-5.0, 1.0, 3.0);
const OBSTACLE_SIZE: Vec3 = Vec3::new(0.55, 1.5, 1.2);

/// Marks the moving entity that owns the continuously active shape caster.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapeCasterDemo;

/// Marks the obstacle used by the continuously active shape-caster demo.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapeCasterDemoObstacle;

/// Moves the demo entity back and forth through the obstacle-detection path.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ShapeCasterDemoMotion {
    pub speed: f32,
    pub min_x: f32,
    pub max_x: f32,
    pub direction: f32,
}

impl Default for ShapeCasterDemoMotion {
    fn default() -> Self {
        Self {
            speed: 1.4,
            min_x: -8.0,
            max_x: -3.6,
            direction: 1.0,
        }
    }
}

/// Owns the moving entity and its continuously updated shape-caster result.
pub struct ShapeCasterStationPlugin;

impl Plugin for ShapeCasterStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_shape_caster_demo)
            .add_systems(
                Update,
                (
                    move_shape_caster_demo,
                    draw_shape_caster_demo.run_if(resource_exists::<GizmoConfigStore>),
                ),
            );
    }
}

fn spawn_shape_caster_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let probe_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Sphere::new(PROBE_RADIUS)));
    let obstacle_mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let probe_material = materials
        .as_mut()
        .map(|materials| materials.add(PROBE_COLOR));
    let obstacle_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.9, 0.3, 0.18)));

    let mut probe = commands.spawn((
        ShapeCasterDemo,
        ShapeCasterDemoMotion::default(),
        ShapeCaster::new(
            Collider::sphere(PROBE_RADIUS),
            Vec3::ZERO,
            Quat::IDENTITY,
            Dir3::X,
        )
        .with_max_distance(CAST_DISTANCE)
        .with_max_hits(1),
        Transform::from_translation(PROBE_START),
        Name::new("Continuously Active ShapeCaster"),
    ));
    if let (Some(mesh), Some(material)) = (probe_mesh, probe_material) {
        probe.insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    let mut obstacle = commands.spawn((
        ShapeCasterDemoObstacle,
        RigidBody::Static,
        layers_for(SandboxLayer::World),
        Collider::cuboid(OBSTACLE_SIZE.x, OBSTACLE_SIZE.y, OBSTACLE_SIZE.z),
        Transform::from_translation(OBSTACLE_POSITION),
        Name::new("ShapeCaster Demo Obstacle"),
    ));
    if let (Some(mesh), Some(material)) = (obstacle_mesh, obstacle_material) {
        obstacle.insert((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(OBSTACLE_POSITION).with_scale(OBSTACLE_SIZE),
        ));
    }
}

fn move_shape_caster_demo(
    time: Res<Time>,
    mut probes: Query<(&mut Transform, &mut ShapeCasterDemoMotion), With<ShapeCasterDemo>>,
) {
    let delta = time.delta_secs();
    for (mut transform, mut motion) in &mut probes {
        let mut next_x = transform.translation.x + motion.direction * motion.speed * delta;
        if next_x > motion.max_x {
            next_x = motion.max_x - (next_x - motion.max_x);
            motion.direction = -1.0;
        } else if next_x < motion.min_x {
            next_x = motion.min_x + (motion.min_x - next_x);
            motion.direction = 1.0;
        }
        transform.translation.x = next_x;
    }
}

fn draw_shape_caster_demo(
    mut gizmos: Gizmos,
    casters: Query<(&ShapeCaster, &ShapeHits), With<ShapeCasterDemo>>,
    names: Query<&Name>,
) {
    for (caster, hits) in &casters {
        let origin = caster.global_origin();
        let direction = caster.global_direction().as_vec3();
        let hit = hits.iter_sorted().next();
        let path_end = hit
            .as_ref()
            .map(|hit| origin + direction * hit.distance)
            .unwrap_or(origin + direction * CAST_DISTANCE);

        gizmos.line(origin, path_end, CAST_COLOR);
        gizmos.sphere(origin, PROBE_RADIUS, PROBE_COLOR);
        if hit.is_some() {
            gizmos.line(path_end, origin + direction * CAST_DISTANCE, EMPTY_COLOR);
        }

        let Some(hit) = hit else {
            gizmos.text(
                Isometry3d::new(path_end + Vec3::Y * 0.7, Quat::IDENTITY),
                "SHAPE CASTER\nNO HIT",
                0.3,
                Vec2::ZERO,
                CAST_COLOR,
            );
            continue;
        };

        gizmos.cross(hit.point1, 0.14, HIT_COLOR);
        gizmos.arrow(hit.point1, hit.point1 + hit.normal1 * 0.75, NORMAL_COLOR);
        let name = names
            .get(hit.entity)
            .map(Name::as_str)
            .unwrap_or("Unknown collider");
        let label = format!(
            "SHAPE CASTER HIT\n{name}\nDistance: {:.2} m\nNormal: ({:.2}, {:.2}, {:.2})",
            hit.distance, hit.normal1.x, hit.normal1.y, hit.normal1.z,
        );
        gizmos.text(
            Isometry3d::new(hit.point1 + Vec3::Y * 0.9, Quat::IDENTITY),
            &label,
            0.3,
            Vec2::ZERO,
            HIT_COLOR,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn shape_caster_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            ShapeCasterStationPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn run_physics_frame(app: &mut App) {
        app.update();
        app.update();
    }

    fn probe_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<ShapeCasterDemo>>()
            .iter(world)
            .next()
            .expect("shape caster demo probe")
    }

    fn first_hit(app: &mut App) -> Option<ShapeHitData> {
        let world = app.world_mut();
        let mut casters = world.query_filtered::<&ShapeHits, With<ShapeCasterDemo>>();
        casters
            .iter(world)
            .next()
            .and_then(|hits| hits.iter_sorted().next())
    }

    #[test]
    fn station_spawns_an_enabled_shape_caster_with_a_required_hit_buffer() {
        let mut app = shape_caster_app();
        run_physics_frame(&mut app);

        let probe = probe_entity(&mut app);
        let entity = app.world().entity(probe);
        let caster = entity.get::<ShapeCaster>().expect("shape caster component");

        assert!(caster.enabled);
        assert_eq!(caster.max_hits, 1);
        assert_eq!(caster.max_distance, CAST_DISTANCE);
        assert!(entity.contains::<ShapeHits>());
        assert!(entity.contains::<ShapeCasterDemoMotion>());
        assert!(first_hit(&mut app).is_some());
    }

    #[test]
    fn moving_the_probe_updates_the_existing_shape_cast_result() {
        let mut app = shape_caster_app();
        run_physics_frame(&mut app);

        let probe = probe_entity(&mut app);
        let initial_hit = first_hit(&mut app).expect("obstacle should be ahead of probe");
        assert!(initial_hit.distance > 0.0);

        app.world_mut().entity_mut(probe).insert((
            Transform::from_translation(Vec3::new(-4.0, 1.0, 3.0)),
            ShapeCasterDemoMotion {
                speed: 0.0,
                ..default()
            },
        ));
        run_physics_frame(&mut app);
        assert!(first_hit(&mut app).is_none());
        assert!(app.world().entity(probe).contains::<ShapeCaster>());

        app.world_mut().entity_mut(probe).insert((
            Transform::from_translation(PROBE_START),
            ShapeCasterDemoMotion {
                speed: 0.0,
                ..default()
            },
        ));
        run_physics_frame(&mut app);
        let return_hit = first_hit(&mut app).expect("obstacle should be ahead again");
        assert!((return_hit.distance - initial_hit.distance).abs() < 0.01);
    }

    #[test]
    fn automatic_motion_changes_the_probe_transform_without_restarting_the_caster() {
        let mut app = shape_caster_app();
        run_physics_frame(&mut app);

        let probe = probe_entity(&mut app);
        let initial_position = app
            .world()
            .entity(probe)
            .get::<Transform>()
            .expect("probe transform")
            .translation;
        run_physics_frame(&mut app);
        let moved_position = app
            .world()
            .entity(probe)
            .get::<Transform>()
            .expect("probe transform")
            .translation;

        assert_ne!(moved_position, initial_position);
        assert!(app.world().entity(probe).contains::<ShapeHits>());
        assert!(app.world().entity(probe).contains::<ShapeCaster>());
    }
}
