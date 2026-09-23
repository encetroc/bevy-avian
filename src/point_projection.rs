use avian3d::{math::Scalar, prelude::*};
use bevy::prelude::*;

use crate::cursor_hover::CursorRay;

const PROJECTION_COLOR: Color = Color::srgb(1.0, 0.3, 0.85);
const SOURCE_COLOR: Color = Color::srgb(0.8, 0.55, 1.0);
const MARKER_SIZE: f32 = 0.16;

/// The result of the most recent point-projection click.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PointProjectionState {
    /// The closest point on an eligible collider, or `None` after an empty click.
    pub projection: Option<PointProjection>,
    /// The world-space point used as the input to the projection query.
    pub source_point: Option<Vec3>,
}

/// Owns click-driven point projection and its gizmo presentation.
pub struct PointProjectionPlugin;

impl Plugin for PointProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorRay>()
            .init_resource::<PointProjectionState>()
            .add_systems(
                Update,
                (
                    update_point_projection,
                    draw_point_projection.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

/// Projects the point where the cursor ray first meets an eligible collider.
///
/// The ray hit supplies a stable 3D point for the click, while the separate
/// point-projection query demonstrates Avian's closest-point operation. Both
/// queries use the same collider-and-body predicate so decorative meshes and
/// other non-physical scenery cannot produce a result.
fn update_point_projection(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_ray: Res<CursorRay>,
    spatial_query: SpatialQuery,
    eligible_colliders: Query<(), (With<Collider>, With<RigidBody>)>,
    mut state: ResMut<PointProjectionState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let next = cursor_ray.ray.and_then(|ray| {
        let filter = SpatialQueryFilter::default();
        let hit = spatial_query.cast_ray_predicate(
            ray.origin,
            ray.direction,
            Scalar::MAX,
            true,
            &filter,
            &|entity| eligible_colliders.contains(entity),
        )?;
        let source_point = ray.origin + ray.direction * hit.distance;
        let projection =
            spatial_query.project_point_predicate(source_point, false, &filter, &|entity| {
                eligible_colliders.contains(entity)
            })?;
        Some((source_point, projection))
    });

    state.set_if_neq(match next {
        Some((source_point, projection)) => PointProjectionState {
            projection: Some(projection),
            source_point: Some(source_point),
        },
        None => PointProjectionState::default(),
    });
}

fn draw_point_projection(
    mut gizmos: Gizmos,
    state: Res<PointProjectionState>,
    names: Query<&Name>,
) {
    let Some(projection) = state.projection.as_ref() else {
        return;
    };

    let point = projection.point;
    gizmos.cross(point, MARKER_SIZE, PROJECTION_COLOR);

    if let Some(source_point) = state.source_point {
        gizmos.line(source_point, point, SOURCE_COLOR);
    }

    let name = names
        .get(projection.entity)
        .map(Name::as_str)
        .unwrap_or("Unknown collider");
    let label = format!(
        "POINT PROJECTION\n{name}\nPoint: ({:.2}, {:.2}, {:.2})",
        point.x, point.y, point.z,
    );
    gizmos.text(
        Isometry3d::new(point + Vec3::Y * 0.65, Quat::IDENTITY),
        &label,
        0.35,
        Vec2::ZERO,
        PROJECTION_COLOR,
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::{ButtonState, InputPlugin, mouse::MouseButtonInput},
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn projection_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            PointProjectionPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn spawn_box(app: &mut App, name: &str, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Collider::cuboid(0.5, 0.5, 0.5),
                Transform::from_translation(position),
                Name::new(name.to_owned()),
            ))
            .id()
    }

    fn click_at(app: &mut App, target: Vec3) {
        let direction = Dir3::new(target.normalize()).expect("test ray direction");
        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::ZERO, direction);
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    #[test]
    fn click_stores_the_target_collider_and_a_surface_point() {
        let mut app = projection_app();
        let target = spawn_box(&mut app, "Projection Cube", Vec3::new(0.0, 0.0, -4.0));
        app.update();

        click_at(&mut app, Vec3::new(0.0, 0.0, -4.0));

        let state = app.world().resource::<PointProjectionState>();
        let projection = state.projection.as_ref().expect("projection result");
        assert_eq!(projection.entity, target);
        assert_eq!(state.source_point, Some(projection.point));
        assert!((projection.point.z + 3.75).abs() < 1e-4);
        assert!(projection.is_inside);
    }

    #[test]
    fn projection_follows_clicks_near_faces_and_edges() {
        let mut app = projection_app();
        let target = spawn_box(&mut app, "Projection Cube", Vec3::new(0.0, 0.0, -4.0));
        app.update();

        for click in [Vec3::new(0.0, 0.0, -4.0), Vec3::new(2.0, 2.0, -4.0)] {
            click_at(&mut app, click);
            let projection = app
                .world()
                .resource::<PointProjectionState>()
                .projection
                .clone()
                .expect("projection result");
            assert_eq!(projection.entity, target);
            let local = projection.point - Vec3::new(0.0, 0.0, -4.0);
            assert!(local.abs().max_element() >= 0.25 - 1e-4);
            assert!(local.abs().cmple(Vec3::splat(0.25 + 1e-4)).all());
            assert!(projection.is_inside);
        }
    }

    #[test]
    fn click_with_no_eligible_collider_clears_the_projection() {
        let mut app = projection_app();
        app.world_mut().spawn((
            Collider::cuboid(0.5, 0.5, 0.5),
            Transform::from_xyz(0.0, 0.0, -4.0),
            Name::new("Collider Without Body"),
        ));
        app.update();

        click_at(&mut app, Vec3::new(0.0, 0.0, -4.0));

        assert_eq!(
            app.world().resource::<PointProjectionState>(),
            &PointProjectionState::default()
        );
    }
}
