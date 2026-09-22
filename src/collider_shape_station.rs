use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const SHAPE_Z: f32 = -5.0;
const DROP_START_Y: f32 = 4.5;
const DROP_RADIUS: f32 = 0.18;

/// The primitive collider shapes compared by station C.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ColliderShapeKind {
    Sphere,
    Cuboid,
    Capsule,
    Cylinder,
    Cone,
}

impl ColliderShapeKind {
    /// Every primitive demonstrated by the collider-shape laboratory.
    pub const ALL: [Self; 5] = [
        Self::Sphere,
        Self::Cuboid,
        Self::Capsule,
        Self::Cylinder,
        Self::Cone,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Sphere => "SPHERE",
            Self::Cuboid => "CUBOID",
            Self::Capsule => "CAPSULE",
            Self::Cylinder => "CYLINDER",
            Self::Cone => "CONE",
        }
    }

    const fn lane_x(self) -> f32 {
        match self {
            Self::Sphere => 4.3,
            Self::Cuboid => 5.2,
            Self::Capsule => 6.1,
            Self::Cylinder => 7.0,
            Self::Cone => 7.9,
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Sphere => Color::srgb(0.35, 0.82, 1.0),
            Self::Cuboid => Color::srgb(0.98, 0.72, 0.22),
            Self::Capsule => Color::srgb(0.72, 0.45, 0.98),
            Self::Cylinder => Color::srgb(0.35, 0.9, 0.5),
            Self::Cone => Color::srgb(0.98, 0.35, 0.32),
        }
    }

    /// The actual Avian collider for this demonstration.
    pub fn collider(self) -> Collider {
        match self {
            Self::Sphere => Collider::sphere(0.35),
            Self::Cuboid => Collider::cuboid(0.68, 0.68, 0.68),
            Self::Capsule => Collider::capsule(0.26, 0.52),
            Self::Cylinder => Collider::cylinder(0.32, 0.68),
            Self::Cone => Collider::cone(0.32, 0.72),
        }
    }

    fn visual_mesh(self) -> Mesh {
        // The solid meshes are intentionally a little larger than their
        // colliders. With F1 enabled, Avian's orange wireframe is visibly
        // distinct from the solid mesh instead of disappearing into it.
        match self {
            Self::Sphere => Sphere::new(0.44).into(),
            Self::Cuboid => Cuboid::from_size(Vec3::splat(0.86)).into(),
            Self::Capsule => Capsule3d::new(0.31, 0.62).into(),
            Self::Cylinder => Cylinder::new(0.38, 0.82).into(),
            Self::Cone => Cone::new(0.38, 0.86).into(),
        }
    }

    const fn visual_half_height(self) -> f32 {
        match self {
            Self::Sphere => 0.44,
            Self::Cuboid => 0.43,
            Self::Capsule => 0.62,
            Self::Cylinder => 0.41,
            Self::Cone => 0.43,
        }
    }
}

/// Marks the static primitive displayed by the collider-shape laboratory.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColliderShapeLabObject {
    pub kind: ColliderShapeKind,
}

/// Marks the matching dynamic object dropped onto one primitive.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColliderShapeLabDrop {
    pub target: ColliderShapeKind,
}

/// Owns station C's primitive collider comparison and its drop tests.
pub struct ColliderShapeStationPlugin;

impl Plugin for ColliderShapeStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_systems(
                Startup,
                (spawn_collider_shape_lab, spawn_collider_shape_station_ui),
            )
            .add_systems(
                Update,
                draw_collider_shape_labels.run_if(resource_exists::<GizmoConfigStore>),
            )
            .add_systems(PostUpdate, reset_collider_shape_lab);
    }
}

fn spawn_collider_shape_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for kind in ColliderShapeKind::ALL {
        let shape_position = Vec3::new(kind.lane_x(), kind.visual_half_height(), SHAPE_Z);
        let shape_transform = Transform::from_translation(shape_position);
        let shape_material = materials
            .as_mut()
            .map(|materials| materials.add(kind.color()));
        let shape_mesh = meshes.as_mut().map(|meshes| meshes.add(kind.visual_mesh()));

        let mut shape = commands.spawn((
            ColliderShapeLabObject { kind },
            RigidBody::Static,
            kind.collider(),
            shape_transform,
            Name::new(format!("Collider Shape - {}", kind.label())),
        ));
        if let (Some(mesh), Some(material)) = (shape_mesh, shape_material) {
            shape.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }

        let drop_position = Vec3::new(kind.lane_x(), DROP_START_Y, SHAPE_Z);
        let drop_transform = Transform::from_translation(drop_position);
        let drop_material = materials
            .as_mut()
            .map(|materials| materials.add(kind.color().with_luminance(0.72)));
        let drop_mesh = meshes
            .as_mut()
            .map(|meshes| meshes.add(Sphere::new(DROP_RADIUS)));
        let mut drop = commands.spawn((
            ColliderShapeLabDrop { target: kind },
            StationObject::new('C', drop_transform),
            RigidBody::Dynamic,
            Collider::sphere(DROP_RADIUS),
            Friction::new(0.3),
            Restitution::ZERO,
            SleepingDisabled,
            LockedAxes::new()
                .lock_translation_x()
                .lock_translation_z()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            drop_transform,
            Name::new(format!("Collider Shape Drop - {}", kind.label())),
        ));
        if let (Some(mesh), Some(material)) = (drop_mesh, drop_material) {
            drop.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    }
}

#[derive(Component)]
struct ColliderShapeStationPanel;

fn spawn_collider_shape_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(422.0),
                top: px(272.0),
                width: px(350.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            ColliderShapeStationPanel,
            Name::new("Collider Shape Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("C — COLLIDER SHAPE LABORATORY"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Drop a matching ball onto each primitive. Solid meshes are deliberately larger; press F1 to overlay the actual orange Avian collider.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            for kind in ColliderShapeKind::ALL {
                parent.spawn((
                    Text::new(format!("{}    mesh + collider", kind.label())),
                    TextFont::from_font_size(13.0),
                    TextColor(kind.color()),
                ));
            }
        });
}

fn draw_collider_shape_labels(
    mut gizmos: Gizmos,
    shapes: Query<(&ColliderShapeLabObject, &Transform)>,
    drops: Query<(&ColliderShapeLabDrop, &Transform)>,
) {
    for (shape, transform) in &shapes {
        gizmos.text(
            Isometry3d::new(
                transform.translation + Vec3::Y * (shape.kind.visual_half_height() + 0.28),
                Quat::IDENTITY,
            ),
            &format!("{}\nmesh / F1 collider", shape.kind.label()),
            0.27,
            Vec2::ZERO,
            shape.kind.color(),
        );
    }

    for (drop, transform) in &drops {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.3, Quat::IDENTITY),
            &format!("{} DROP", drop.target.label()),
            0.22,
            Vec2::ZERO,
            drop.target.color().with_luminance(0.8),
        );
    }
}

#[allow(clippy::type_complexity)]
fn reset_collider_shape_lab(
    mut requests: MessageReader<ResetStation>,
    mut drops: Query<
        (
            &StationObject,
            &mut Transform,
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut AngularVelocity,
        ),
        With<ColliderShapeLabDrop>,
    >,
) {
    if !requests.read().any(|request| request.code == 'C') {
        return;
    }

    for (
        initial,
        mut transform,
        mut position,
        mut rotation,
        mut linear_velocity,
        mut angular_velocity,
    ) in &mut drops
    {
        *transform = initial.initial_transform;
        position.0 = initial.initial_transform.translation;
        rotation.0 = initial.initial_transform.rotation;
        linear_velocity.0 = initial.initial_linear_velocity;
        angular_velocity.0 = initial.initial_angular_velocity;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use bevy::{
        gizmos::GizmoPlugin,
        input::{
            ButtonState, InputPlugin,
            keyboard::{Key, KeyboardInput},
        },
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::collider_debug::ColliderDebugPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    #[derive(Component)]
    struct TestFloor;

    fn shape_app(with_debug: bool) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            ColliderShapeStationPlugin,
        ));
        if with_debug {
            app.add_plugins((GizmoPlugin, ColliderDebugPlugin));
        }
        app.insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                PHYSICS_STEP,
            )))
            .add_systems(Startup, spawn_test_floor);
        app.finish();
        app
    }

    fn spawn_test_floor(mut commands: Commands) {
        commands.spawn((
            TestFloor,
            RigidBody::Static,
            Collider::cuboid(24.0, 0.5, 24.0),
            Transform::from_xyz(0.0, -0.25, 0.0),
        ));
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn entities_by_kind<T: Component>(
        app: &mut App,
        map: impl Fn(&T) -> ColliderShapeKind,
    ) -> HashMap<ColliderShapeKind, Entity> {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &T)>();
        query
            .iter(world)
            .map(|(entity, component)| (map(component), entity))
            .collect()
    }

    #[test]
    fn station_spawns_one_mesh_backed_static_collider_and_drop_for_each_primitive() {
        let mut app = shape_app(false);
        app.update();

        let world = app.world_mut();
        let mut shapes = world.query::<(
            &ColliderShapeLabObject,
            &RigidBody,
            &Collider,
            &Mesh3d,
            &Name,
        )>();
        let shapes: Vec<_> = shapes.iter(world).collect();
        assert_eq!(shapes.len(), ColliderShapeKind::ALL.len());
        for (shape, body, _, _, name) in shapes {
            assert_eq!(*body, RigidBody::Static);
            assert!(name.as_str().contains(shape.kind.label()));
        }

        let mut drops = world.query::<(&ColliderShapeLabDrop, &RigidBody, &Collider, &Mesh3d)>();
        let drops: Vec<_> = drops.iter(world).collect();
        assert_eq!(drops.len(), ColliderShapeKind::ALL.len());
        assert!(
            drops
                .iter()
                .all(|(_, body, _, _)| **body == RigidBody::Dynamic)
        );
    }

    #[test]
    fn every_drop_falls_onto_its_matching_primitive_collider() {
        let mut app = shape_app(false);
        app.update();

        let shapes = entities_by_kind(&mut app, |shape: &ColliderShapeLabObject| shape.kind);
        let drops = entities_by_kind(&mut app, |drop: &ColliderShapeLabDrop| drop.target);
        let initial_y: HashMap<_, _> = drops
            .iter()
            .map(|(kind, entity)| {
                (
                    *kind,
                    app.world()
                        .entity(*entity)
                        .get::<Position>()
                        .expect("drop position")
                        .0
                        .y,
                )
            })
            .collect();

        run_steps(&mut app, 240);

        for kind in ColliderShapeKind::ALL {
            let drop = drops[&kind];
            let shape = shapes[&kind];
            let final_y = app
                .world()
                .entity(drop)
                .get::<Position>()
                .expect("drop position")
                .0
                .y;
            assert!(final_y < initial_y[&kind] - 1.0, "{kind:?} did not fall");
            assert!(
                app.world().resource::<ContactGraph>().contains(drop, shape),
                "{kind:?} drop did not contact its matching collider"
            );
        }
    }

    #[test]
    fn f1_keeps_solid_meshes_visible_while_enabling_actual_collider_wireframes() {
        let mut app = shape_app(true);
        app.update();

        let mut shapes = app.world_mut().query::<&ColliderShapeLabObject>();
        assert_eq!(
            shapes.iter(app.world()).count(),
            ColliderShapeKind::ALL.len()
        );
        assert!(
            !app.world()
                .resource::<GizmoConfigStore>()
                .config::<PhysicsGizmos>()
                .1
                .hide_meshes
        );
        assert_eq!(
            app.world()
                .resource::<GizmoConfigStore>()
                .config::<PhysicsGizmos>()
                .1
                .collider_color,
            None
        );

        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::F1,
                logical_key: Key::F1,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();

        assert!(
            !app.world()
                .resource::<GizmoConfigStore>()
                .config::<PhysicsGizmos>()
                .1
                .hide_meshes
        );
        assert_eq!(
            app.world()
                .resource::<GizmoConfigStore>()
                .config::<PhysicsGizmos>()
                .1
                .collider_color,
            Some(Color::srgb(1.0, 0.35, 0.1))
        );
    }
}
