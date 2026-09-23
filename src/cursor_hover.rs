use avian3d::{math::Scalar, prelude::*};
use bevy::{input::InputSystems, prelude::*, window::PrimaryWindow};

use crate::camera::FixedFollowCamera;
use crate::player::Player;
use crate::spatial_query_filters::{SpatialQueryFilterSet, SpatialQueryFilterState};

const HOVER_COLOR: Color = Color::srgb(1.0, 0.9, 0.15);
const UNREACHABLE_HOVER_COLOR: Color = Color::srgb(0.95, 0.2, 0.15);
const RAY_COLOR: Color = Color::srgb(0.2, 0.8, 1.0);
const RAY_HIT_COLOR: Color = Color::srgb(1.0, 0.65, 0.15);
const SURFACE_NORMAL_COLOR: Color = Color::srgb(0.3, 1.0, 0.4);
const HOVER_LABEL_HEIGHT: f32 = 0.35;
const RAY_VISUAL_LENGTH: f32 = 30.0;
const SURFACE_NORMAL_LENGTH: f32 = 0.75;

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

/// The interaction status of the physical body currently under the cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoverReachability {
    /// The player can start a grab on the hovered body.
    #[default]
    Reachable,
    /// The hovered body is visible but outside the interaction radius.
    Unreachable,
}

impl HoverReachability {
    pub const fn is_reachable(self) -> bool {
        matches!(self, Self::Reachable)
    }
}

/// Tunable distance used to decide whether a hovered body can be grabbed.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct GrabDistanceSettings {
    /// Maximum world-space distance from the player to a hovered body.
    pub max_distance: f32,
}

impl Default for GrabDistanceSettings {
    fn default() -> Self {
        Self { max_distance: 5.0 }
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
    /// The collider surface normal at the hit point, in world space.
    pub normal: Vec3,
    /// Whether the player is close enough to start a grab.
    pub reachability: HoverReachability,
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
            .init_resource::<GrabDistanceSettings>()
            .init_resource::<SpatialQueryFilterState>()
            .add_systems(Startup, create_hover_material)
            .add_systems(PreUpdate, update_cursor_ray.after(InputSystems))
            .add_systems(
                Update,
                (
                    update_hover_state,
                    update_hover_reachability,
                    apply_hover_material,
                    draw_cursor_raycast.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain()
                    .after(SpatialQueryFilterSet::ApplyControls),
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
pub(crate) fn update_hover_state(
    cursor_ray: Res<CursorRay>,
    spatial_query: SpatialQuery,
    filter_state: Res<SpatialQueryFilterState>,
    selectable_bodies: Query<(), (With<Collider>, With<RigidBody>)>,
    names: Query<&Name>,
    mut hover: ResMut<HoverState>,
) {
    let next_hover = cursor_ray.ray.and_then(|ray| {
        let filter = filter_state.query_filter();
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
                    normal: hit.normal,
                    reachability: HoverReachability::Reachable,
                }
            })
    });

    hover.set_if_neq(HoverState { object: next_hover });
}

/// Updates interaction eligibility without removing hover feedback for distant bodies.
pub(crate) fn update_hover_reachability(
    settings: Res<GrabDistanceSettings>,
    players: Query<&Position, With<Player>>,
    bodies: Query<&Position>,
    mut hover: ResMut<HoverState>,
) {
    let Some(object) = hover.object.as_ref() else {
        return;
    };

    let reachability = players
        .iter()
        .next()
        .and_then(|player| bodies.get(object.entity).ok().map(|body| (player, body)))
        .map(|(player, body)| is_within_grab_distance(player.0, body.0, settings.max_distance))
        .map_or(HoverReachability::Reachable, |reachable| {
            if reachable {
                HoverReachability::Reachable
            } else {
                HoverReachability::Unreachable
            }
        });

    if object.reachability == reachability {
        return;
    }

    let mut next_object = object.clone();
    next_object.reachability = reachability;
    hover.set_if_neq(HoverState {
        object: Some(next_object),
    });
}

pub(crate) fn is_within_grab_distance(
    player_position: Vec3,
    body_position: Vec3,
    max_distance: f32,
) -> bool {
    if !max_distance.is_finite() || max_distance < 0.0 {
        return false;
    }

    // Keep the exact boundary inclusive despite small physics/integration
    // rounding differences between the player and body positions.
    let allowed_distance = max_distance + 1e-5;
    player_position.distance_squared(body_position) <= allowed_distance * allowed_distance
}

#[derive(Resource)]
struct HoverHighlightMaterial(Handle<StandardMaterial>);

#[derive(Resource)]
struct HoverUnreachableMaterial(Handle<StandardMaterial>);

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
    commands.insert_resource(HoverUnreachableMaterial(materials.add(StandardMaterial {
        base_color: UNREACHABLE_HOVER_COLOR,
        emissive: LinearRgba::rgb(0.8, 0.05, 0.02),
        ..default()
    })));
}

/// Swaps the hovered mesh to a shared highlight material and restores the
/// original material as soon as the cursor moves away.
fn apply_hover_material(
    mut commands: Commands,
    hover: Res<HoverState>,
    highlight_material: Option<Res<HoverHighlightMaterial>>,
    unreachable_material: Option<Res<HoverUnreachableMaterial>>,
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
    let Some(unreachable_material) = unreachable_material else {
        return;
    };

    for (entity, mut material, original) in &mut materials {
        if let Some(object) = hover
            .object
            .as_ref()
            .filter(|object| object.entity == entity)
        {
            if original.is_none() {
                let original_handle = material.0.clone();
                material.0 = if object.reachability.is_reachable() {
                    highlight_material.0.clone()
                } else {
                    unreachable_material.0.clone()
                };
                commands
                    .entity(entity)
                    .insert(HoverOriginalMaterial(original_handle));
            } else if object.reachability.is_reachable() {
                material.0 = highlight_material.0.clone();
            } else {
                material.0 = unreachable_material.0.clone();
            }
        } else if let Some(original) = original {
            material.0 = original.0.clone();
            commands.entity(entity).remove::<HoverOriginalMaterial>();
        }
    }
}

/// Draws the camera ray and exposes the first physical collider hit.
///
/// The ray and hit details use the same [`CursorRay`] and [`HoverState`] that
/// drive hover selection, so the visualization cannot report a different
/// collider than the one selected by the interaction systems.
fn draw_cursor_raycast(mut gizmos: Gizmos, cursor_ray: Res<CursorRay>, hover: Res<HoverState>) {
    let Some(ray) = cursor_ray.ray else {
        return;
    };

    gizmos.ray(ray.origin, ray.direction * RAY_VISUAL_LENGTH, RAY_COLOR);

    let Some(object) = hover.object.as_ref() else {
        gizmos.text(
            Isometry3d::new(
                ray.origin + ray.direction * (RAY_VISUAL_LENGTH * 0.7),
                Quat::IDENTITY,
            ),
            "RAYCAST: no hit",
            0.4,
            Vec2::ZERO,
            RAY_COLOR,
        );
        return;
    };

    let hit_color = if object.reachability.is_reachable() {
        RAY_HIT_COLOR
    } else {
        UNREACHABLE_HOVER_COLOR
    };
    gizmos.cross(object.hit_position, 0.12, hit_color);
    gizmos.arrow(
        object.hit_position,
        object.hit_position + object.normal * SURFACE_NORMAL_LENGTH,
        SURFACE_NORMAL_COLOR,
    );

    let label = format!(
        "RAYCAST HIT\nEntity: {} ({:?})\nDistance: {:.2} m\nPosition: ({:.2}, {:.2}, {:.2})\nNormal: ({:.2}, {:.2}, {:.2})",
        object.name,
        object.entity,
        object.distance,
        object.hit_position.x,
        object.hit_position.y,
        object.hit_position.z,
        object.normal.x,
        object.normal.y,
        object.normal.z,
    );
    gizmos.text(
        Isometry3d::new(
            object.hit_position + Vec3::Y * HOVER_LABEL_HEIGHT,
            Quat::IDENTITY,
        ),
        &label,
        0.4,
        Vec2::ZERO,
        hit_color,
    );
}

#[cfg(test)]
mod tests {
    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};
    use std::time::Duration;

    use super::*;
    use crate::player::Player;

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

    fn spawn_player(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                Player,
                Position(position),
                Transform::from_translation(position),
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
        assert_eq!(hover.as_ref().map(|object| object.normal), Some(Vec3::Z));
        assert!(app.world().entity(body).contains::<HoverOriginalMaterial>());
    }

    #[test]
    fn raycast_reports_first_hit_distance_position_and_surface_normal() {
        let mut app = hover_app();
        let near = spawn_body(&mut app, "Near Cube", Vec3::new(0.0, 0.0, -3.0));
        let far = spawn_body(&mut app, "Far Cube", Vec3::new(0.0, 0.0, -6.0));
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -6.0));

        let hit = app
            .world()
            .resource::<HoverState>()
            .object
            .as_ref()
            .expect("ray should hit the near body");
        assert_eq!(hit.entity, near);
        assert_ne!(hit.entity, far);
        assert!((hit.distance - 2.75).abs() < 1e-4, "hit: {hit:?}");
        assert!((hit.hit_position - Vec3::new(0.0, 0.0, -2.75)).length() < 1e-4);
        assert_eq!(hit.normal, Vec3::Z);
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
    fn hover_marks_bodies_outside_the_player_radius_as_unreachable() {
        let mut app = hover_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let body = spawn_body(&mut app, "Distant Cube", Vec3::new(0.0, 0.0, -6.0));
        app.world_mut()
            .insert_resource(GrabDistanceSettings { max_distance: 5.0 });
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -6.0));

        let hover = app.world().resource::<HoverState>().object.clone().unwrap();
        assert_eq!(hover.entity, body);
        assert_eq!(hover.reachability, HoverReachability::Unreachable);
        assert_eq!(
            app.world()
                .entity(body)
                .get::<MeshMaterial3d<StandardMaterial>>()
                .unwrap()
                .0,
            app.world().resource::<HoverUnreachableMaterial>().0
        );
        assert_eq!(
            app.world().entity(player).get::<Position>().unwrap().0,
            Vec3::ZERO
        );
    }

    #[test]
    fn moving_the_player_updates_hover_reachability_at_the_boundary() {
        let mut app = hover_app();
        let player = spawn_player(&mut app, Vec3::ZERO);
        let body = spawn_body(&mut app, "Boundary Cube", Vec3::new(0.0, 0.0, -5.0));
        app.world_mut()
            .insert_resource(GrabDistanceSettings { max_distance: 5.0 });
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .reachability,
            HoverReachability::Reachable
        );

        app.world_mut()
            .entity_mut(player)
            .get_mut::<Position>()
            .unwrap()
            .0 = Vec3::new(0.0, 0.0, 0.01);
        app.update();
        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .reachability,
            HoverReachability::Unreachable
        );

        app.world_mut()
            .entity_mut(player)
            .get_mut::<Position>()
            .unwrap()
            .0 = Vec3::ZERO;
        app.update();
        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .reachability,
            HoverReachability::Reachable
        );
        assert_eq!(
            app.world().entity(body).get::<Name>().unwrap().as_str(),
            "Boundary Cube"
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
