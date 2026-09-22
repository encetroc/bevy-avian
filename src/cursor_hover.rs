use avian3d::{math::Scalar, prelude::*};
use bevy::{input::InputSystems, prelude::*, window::PrimaryWindow};

use crate::camera::FixedFollowCamera;

const HOVER_COLOR: Color = Color::srgb(1.0, 0.9, 0.15);
const HOVER_LABEL_HEIGHT: f32 = 0.35;

/// The ray generated from the current cursor position.
///
/// The resource is also a deliberate headless-test seam: tests can set a ray
/// directly when no window or camera exists.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CursorRay {
    /// The current camera ray, or `None` when the cursor is outside the window.
    pub ray: Option<Ray3d>,
}

impl CursorRay {
    /// Sets the ray used by the hover query.
    pub fn set(&mut self, origin: Vec3, direction: Dir3) {
        self.ray = Some(Ray3d::new(origin, direction));
    }

    /// Clears the current cursor ray.
    pub fn clear(&mut self) {
        self.ray = None;
    }
}

/// The physical body currently under the cursor and the identity shown to the user.
#[derive(Clone, Debug, PartialEq)]
pub struct HoverInfo {
    /// The entity hit by the cursor ray.
    pub entity: Entity,
    /// The entity's [`Name`] or a stable entity fallback.
    pub name: String,
    /// The point where the cursor ray touched the collider.
    pub hit_position: Vec3,
    /// The distance from the camera ray origin to the hit.
    pub distance: f32,
}

/// Current cursor hover state. `None` means that the cursor is over empty space
/// or only over non-physical scenery.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct HoverState {
    pub object: Option<HoverInfo>,
}

/// Owns cursor-to-world ray generation, Avian filtering, and hover presentation.
pub struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorRay>()
            .init_resource::<HoverState>()
            .add_systems(Startup, create_hover_material)
            .add_systems(PreUpdate, update_cursor_ray.after(InputSystems))
            .add_systems(
                Update,
                (
                    update_hover_state,
                    apply_hover_material,
                    draw_hover_identity.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

/// Converts the primary window cursor position into a world-space camera ray.
///
/// A missing window or camera is intentionally a no-op so headless tests can
/// drive [`CursorRay`] directly without a hidden window entity.
fn update_cursor_ray(
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<FixedFollowCamera>>>,
    mut cursor_ray: ResMut<CursorRay>,
) {
    let (Some(window), Some(camera)) = (window, camera) else {
        return;
    };
    let (camera, camera_transform) = *camera;

    let Some(cursor_position) = window.cursor_position() else {
        cursor_ray.clear();
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        cursor_ray.clear();
        return;
    };

    cursor_ray.set(ray.origin, ray.direction);
}

/// Casts the cursor ray and accepts only entities that are actual Avian bodies.
///
/// The predicate is important because the scene contains visible station pads,
/// paths, and labels that have no physics representation and must not become
/// selectable merely because their meshes are under the cursor.
fn update_hover_state(
    cursor_ray: Res<CursorRay>,
    spatial_query: SpatialQuery,
    selectable_bodies: Query<(), (With<Collider>, With<RigidBody>)>,
    names: Query<&Name>,
    mut hover: ResMut<HoverState>,
) {
    let next_hover = cursor_ray.ray.and_then(|ray| {
        let filter = SpatialQueryFilter::default();
        spatial_query
            .cast_ray_predicate(
                ray.origin,
                ray.direction,
                Scalar::MAX,
                true,
                &filter,
                &|entity| selectable_bodies.contains(entity),
            )
            .map(|hit| {
                let name = names
                    .get(hit.entity)
                    .map(|name| name.as_str().to_owned())
                    .unwrap_or_else(|_| format!("Physics Entity {:?}", hit.entity));
                HoverInfo {
                    entity: hit.entity,
                    name,
                    hit_position: ray.origin + ray.direction * hit.distance,
                    distance: hit.distance,
                }
            })
    });

    hover.set_if_neq(HoverState { object: next_hover });
}

#[derive(Resource)]
struct HoverHighlightMaterial(Handle<StandardMaterial>);

#[derive(Component)]
struct HoverOriginalMaterial(Handle<StandardMaterial>);

fn create_hover_material(
    mut commands: Commands,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };

    commands.insert_resource(HoverHighlightMaterial(materials.add(StandardMaterial {
        base_color: HOVER_COLOR,
        emissive: LinearRgba::rgb(0.8, 0.55, 0.0),
        ..default()
    })));
}

/// Swaps the hovered mesh to a shared highlight material and restores the
/// original material as soon as the cursor moves away.
fn apply_hover_material(
    mut commands: Commands,
    hover: Res<HoverState>,
    highlight_material: Option<Res<HoverHighlightMaterial>>,
    mut materials: Query<(
        Entity,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&HoverOriginalMaterial>,
    )>,
) {
    if !hover.is_changed() {
        return;
    }
    let Some(highlight_material) = highlight_material else {
        return;
    };

    for (entity, mut material, original) in &mut materials {
        if Some(entity) == hover.object.as_ref().map(|object| object.entity) {
            if original.is_none() {
                let original_handle = material.0.clone();
                material.0 = highlight_material.0.clone();
                commands
                    .entity(entity)
                    .insert(HoverOriginalMaterial(original_handle));
            }
        } else if let Some(original) = original {
            material.0 = original.0.clone();
            commands.entity(entity).remove::<HoverOriginalMaterial>();
        }
    }
}

/// Displays the physical body's basic identity at the ray hit point.
fn draw_hover_identity(mut gizmos: Gizmos, hover: Res<HoverState>) {
    let Some(object) = hover.object.as_ref() else {
        return;
    };

    gizmos.text(
        Isometry3d::new(
            object.hit_position + Vec3::Y * HOVER_LABEL_HEIGHT,
            Quat::IDENTITY,
        ),
        &object.name,
        0.45,
        Vec2::ZERO,
        HOVER_COLOR,
    );
}

#[cfg(test)]
mod tests {
    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};
    use std::time::Duration;

    use super::*;

    fn hover_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CursorHoverPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )))
        .init_resource::<Assets<StandardMaterial>>();
        app.finish();
        app
    }

    fn spawn_body(app: &mut App, name: &str, position: Vec3) -> Entity {
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.4, 0.8),
                ..default()
            });
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Collider::cuboid(0.5, 0.5, 0.5),
                Transform::from_translation(position),
                MeshMaterial3d(material),
                Name::new(name.to_owned()),
            ))
            .id()
    }

    fn point_cursor_at(app: &mut App, target: Vec3) {
        let direction = Dir3::new(target.normalize()).expect("test ray direction");
        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::ZERO, direction);
        app.update();
    }

    #[test]
    fn raycast_filters_non_physical_scenery_and_reports_the_named_body() {
        let mut app = hover_app();
        let body = spawn_body(&mut app, "Hover Cube", Vec3::new(0.0, 0.0, -5.0));
        let scenery_mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::default());
        app.world_mut().spawn((
            Mesh3d(scenery_mesh),
            Transform::from_xyz(0.0, 0.0, -2.0),
            Name::new("Decorative Station Pad"),
        ));
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));

        let hover = &app.world().resource::<HoverState>().object;
        assert_eq!(hover.as_ref().map(|object| object.entity), Some(body));
        assert_eq!(
            hover.as_ref().map(|object| object.name.as_str()),
            Some("Hover Cube")
        );
        assert!(app.world().entity(body).contains::<HoverOriginalMaterial>());
    }

    #[test]
    fn hover_follows_the_cursor_between_physical_objects() {
        let mut app = hover_app();
        let left = spawn_body(&mut app, "Left Cube", Vec3::new(-2.0, 0.0, -5.0));
        let right = spawn_body(&mut app, "Right Cube", Vec3::new(2.0, 0.0, -5.0));
        app.update();

        point_cursor_at(&mut app, Vec3::new(-2.0, 0.0, -5.0));
        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .entity,
            left
        );

        point_cursor_at(&mut app, Vec3::new(2.0, 0.0, -5.0));
        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .entity,
            right
        );
        assert!(!app.world().entity(left).contains::<HoverOriginalMaterial>());
        assert!(
            app.world()
                .entity(right)
                .contains::<HoverOriginalMaterial>()
        );
    }

    #[test]
    fn moving_off_physics_clears_hover_and_restores_the_original_material() {
        let mut app = hover_app();
        let body = spawn_body(&mut app, "Hover Cube", Vec3::new(0.0, 0.0, -5.0));
        app.update();
        let original_material = app
            .world()
            .entity(body)
            .get::<MeshMaterial3d<StandardMaterial>>()
            .unwrap()
            .0
            .clone();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        let highlighted_material = app
            .world()
            .entity(body)
            .get::<MeshMaterial3d<StandardMaterial>>()
            .unwrap()
            .0
            .clone();
        assert_ne!(highlighted_material, original_material);

        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::ZERO, Dir3::X);
        app.update();

        assert!(app.world().resource::<HoverState>().object.is_none());
        assert!(!app.world().entity(body).contains::<HoverOriginalMaterial>());
        assert_eq!(
            app.world()
                .entity(body)
                .get::<MeshMaterial3d<StandardMaterial>>()
                .unwrap()
                .0
                .clone(),
            original_material
        );
    }
}
