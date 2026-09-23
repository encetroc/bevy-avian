use avian3d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

use crate::cursor_hover::CursorRay;

const VOLUME_SIZE: Vec3 = Vec3::splat(0.8);
const VOLUME_HALF_EXTENT: f32 = VOLUME_SIZE.x * 0.5;
const VOLUME_START: Vec3 = Vec3::new(-8.0, 1.0, 7.5);
const CURSOR_PLANE_Y: f32 = 1.0;
const KEYBOARD_MOVE_SPEED: f32 = 3.0;
const CLEAR_COLOR: Color = Color::srgb(0.15, 0.95, 0.3);
const INTERSECTING_COLOR: Color = Color::srgb(0.95, 0.15, 0.12);

/// Marks the movable volume used by the intersection-testing demonstration.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IntersectionTestVolume;

/// The result of the latest intersection test for the debug volume.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct IntersectionTestResult {
    /// The colliders currently intersecting the debug volume.
    pub intersections: Vec<Entity>,
}

impl IntersectionTestResult {
    /// Returns whether the debug volume currently overlaps any collider.
    pub fn is_intersecting(&self) -> bool {
        !self.intersections.is_empty()
    }
}

/// Headless-friendly movement commands for the intersection volume.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum IntersectionTestAction {
    /// Place the volume at an absolute world-space position.
    #[allow(dead_code, reason = "headless callers use this placement seam")]
    MoveTo(Vec3),
    /// Move the volume by a world-space offset.
    MoveBy(Vec3),
}

#[derive(Resource)]
struct IntersectionTestMaterials {
    clear: Handle<StandardMaterial>,
    intersecting: Handle<StandardMaterial>,
}

/// Owns the movable volume and the continuously updated shape intersection test.
pub struct IntersectionTestingPlugin;

impl Plugin for IntersectionTestingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorRay>()
            .init_resource::<IntersectionTestResult>()
            .add_message::<IntersectionTestAction>()
            .add_systems(Startup, spawn_intersection_test_volume)
            .add_systems(
                PreUpdate,
                queue_keyboard_intersection_actions.after(InputSystems),
            )
            .add_systems(
                Update,
                (
                    move_intersection_test_volume,
                    update_intersection_test_result,
                    update_intersection_test_material,
                    draw_intersection_test_label.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

fn spawn_intersection_test_volume(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let material_handles = materials.as_mut().map(|materials| {
        (
            materials.add(StandardMaterial {
                base_color: CLEAR_COLOR,
                emissive: LinearRgba::rgb(0.04, 0.3, 0.06),
                ..default()
            }),
            materials.add(StandardMaterial {
                base_color: INTERSECTING_COLOR,
                emissive: LinearRgba::rgb(0.35, 0.03, 0.02),
                ..default()
            }),
        )
    });

    let mut volume = commands.spawn((
        IntersectionTestVolume,
        RigidBody::Kinematic,
        Sensor,
        Collider::cuboid(VOLUME_HALF_EXTENT, VOLUME_HALF_EXTENT, VOLUME_HALF_EXTENT),
        Position(VOLUME_START),
        Rotation::default(),
        Transform::from_translation(VOLUME_START),
        Name::new("Intersection Test Volume"),
    ));

    if let Some(meshes) = meshes.as_mut() {
        volume.insert(Mesh3d(meshes.add(Cuboid::from_size(VOLUME_SIZE))));
    }
    if let Some((clear, intersecting)) = material_handles {
        volume.insert(MeshMaterial3d(clear.clone()));
        commands.insert_resource(IntersectionTestMaterials {
            clear,
            intersecting,
        });
    }
}

fn queue_keyboard_intersection_actions(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut actions: MessageWriter<IntersectionTestAction>,
) {
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction.z += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        direction.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::PageDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::PageUp) {
        direction.y += 1.0;
    }

    if direction != Vec3::ZERO {
        actions.write(IntersectionTestAction::MoveBy(
            direction.normalize() * KEYBOARD_MOVE_SPEED * time.delta_secs(),
        ));
    }
}

fn move_intersection_test_volume(
    cursor_ray: Res<CursorRay>,
    mut actions: MessageReader<IntersectionTestAction>,
    mut volumes: Query<(&mut Position, &mut Transform), With<IntersectionTestVolume>>,
) {
    let Ok((mut position, mut transform)) = volumes.single_mut() else {
        return;
    };

    let mut next_position = position.0;
    for action in actions.read() {
        match action {
            IntersectionTestAction::MoveTo(target) => next_position = *target,
            IntersectionTestAction::MoveBy(offset) => next_position += *offset,
        }
    }

    // When a camera ray is available, project it onto the horizontal test plane.
    // This makes the volume follow the same cursor ray used by the raycast station
    // while keeping direct actions available to headless tests and keyboard control.
    if let Some(target) = cursor_ray.ray.and_then(cursor_plane_target) {
        next_position = target;
    }

    if next_position != position.0 {
        position.0 = next_position;
        transform.translation = next_position;
    }
}

fn cursor_plane_target(ray: Ray3d) -> Option<Vec3> {
    if ray.direction.y.abs() < f32::EPSILON {
        return None;
    }

    let distance = (CURSOR_PLANE_Y - ray.origin.y) / ray.direction.y;
    (distance >= 0.0).then(|| ray.origin + ray.direction * distance)
}

fn update_intersection_test_result(
    spatial_query: SpatialQuery,
    volumes: Query<(Entity, &Position, &Rotation, &Collider), With<IntersectionTestVolume>>,
    mut result: ResMut<IntersectionTestResult>,
) {
    let Ok((volume, position, rotation, shape)) = volumes.single() else {
        return;
    };

    result.intersections = spatial_query.shape_intersections(
        shape,
        position.0,
        rotation.0,
        &SpatialQueryFilter::from_excluded_entities([volume]),
    );
}

fn update_intersection_test_material(
    result: Res<IntersectionTestResult>,
    materials: Option<Res<IntersectionTestMaterials>>,
    mut volumes: Query<&mut MeshMaterial3d<StandardMaterial>, With<IntersectionTestVolume>>,
) {
    let Some(materials) = materials else {
        return;
    };

    let handle = if result.is_intersecting() {
        &materials.intersecting
    } else {
        &materials.clear
    };
    for mut material in &mut volumes {
        material.0 = handle.clone();
    }
}

fn draw_intersection_test_label(
    mut gizmos: Gizmos,
    result: Res<IntersectionTestResult>,
    volumes: Query<&Position, With<IntersectionTestVolume>>,
) {
    let Ok(position) = volumes.single() else {
        return;
    };

    let color = if result.is_intersecting() {
        INTERSECTING_COLOR
    } else {
        CLEAR_COLOR
    };
    let state = if result.is_intersecting() {
        "INTERSECTING"
    } else {
        "CLEAR"
    };
    gizmos.text(
        Isometry3d::new(
            position.0 + Vec3::Y * (VOLUME_SIZE.y + 0.35),
            Quat::IDENTITY,
        ),
        &format!("INTERSECTION TEST\n{state}"),
        0.3,
        Vec2::ZERO,
        color,
    );
}

#[cfg(test)]
mod tests {
    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};
    use std::time::Duration;

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn intersection_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            IntersectionTestingPlugin,
        ))
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn volume_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<IntersectionTestVolume>>()
            .iter(world)
            .next()
            .expect("intersection test volume")
    }

    fn move_volume_to(app: &mut App, position: Vec3) {
        app.world_mut()
            .resource_mut::<Messages<IntersectionTestAction>>()
            .write(IntersectionTestAction::MoveTo(position));
        app.update();
    }

    fn color(app: &App) -> Color {
        let volume = app.world().entity(
            app.world()
                .iter_entities()
                .find(|entity| entity.contains::<IntersectionTestVolume>())
                .expect("intersection volume")
                .id(),
        );
        let material = volume
            .get::<MeshMaterial3d<StandardMaterial>>()
            .expect("volume material");
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(&material.0)
            .expect("standard material")
            .base_color
    }

    #[test]
    fn volume_starts_clear_and_uses_the_green_material() {
        let mut app = intersection_app();
        app.update();

        assert!(
            !app.world()
                .resource::<IntersectionTestResult>()
                .is_intersecting()
        );
        assert_eq!(color(&app), CLEAR_COLOR);
    }

    #[test]
    fn moving_the_volume_updates_intersection_and_color_immediately() {
        let mut app = intersection_app();
        let target = app
            .world_mut()
            .spawn((
                RigidBody::Static,
                Collider::cuboid(0.5, 0.5, 0.5),
                Position(Vec3::ZERO),
                Rotation::default(),
                Transform::default(),
                Name::new("Intersection Target"),
            ))
            .id();
        app.update();

        move_volume_to(&mut app, Vec3::ZERO);
        let result = app.world().resource::<IntersectionTestResult>();
        assert!(result.is_intersecting());
        assert!(result.intersections.contains(&target));
        assert_eq!(color(&app), INTERSECTING_COLOR);

        move_volume_to(&mut app, Vec3::new(3.0, 1.0, 0.0));
        assert!(
            !app.world()
                .resource::<IntersectionTestResult>()
                .is_intersecting()
        );
        assert_eq!(color(&app), CLEAR_COLOR);
    }

    #[test]
    fn cursor_ray_moves_the_volume_across_the_test_plane() {
        let mut app = intersection_app();
        app.update();
        let volume = volume_entity(&mut app);

        app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::new(0.0, 4.0, 0.0),
            Dir3::new(Vec3::new(1.0, -1.0, 0.0)).expect("cursor direction"),
        );
        app.update();

        let position = app
            .world()
            .entity(volume)
            .get::<Position>()
            .expect("volume position")
            .0;
        assert!((position - Vec3::new(3.0, CURSOR_PLANE_Y, 0.0)).length() < 1e-4);
    }
}
