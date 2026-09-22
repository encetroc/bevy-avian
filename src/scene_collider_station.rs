use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const SCENE_POSITION: Vec3 = Vec3::new(6.0, 0.0, 2.2);
const SCENE_ROTATION: f32 = 0.12;
const DROP_RADIUS: f32 = 0.2;
const DROP_HEIGHT: f32 = 2.4;

/// The mesh nodes included in the small hierarchy-based scene.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SceneMeshKind {
    Ramp,
    Platform,
    Pillar,
}

impl SceneMeshKind {
    const ALL: [Self; 3] = [Self::Ramp, Self::Platform, Self::Pillar];

    const fn label(self) -> &'static str {
        match self {
            Self::Ramp => "RAMP",
            Self::Platform => "PLATFORM",
            Self::Pillar => "PILLAR",
        }
    }

    fn local_transform(self) -> Transform {
        match self {
            Self::Ramp => Transform::from_translation(Vec3::new(-1.1, 0.65, 0.0))
                .with_rotation(Quat::from_rotation_z(-0.15))
                .with_scale(Vec3::new(1.35, 0.35, 1.1)),
            Self::Platform => Transform::from_translation(Vec3::new(0.0, 0.45, 0.0))
                .with_scale(Vec3::new(1.0, 0.6, 1.1)),
            Self::Pillar => Transform::from_translation(Vec3::new(1.1, 0.9, 0.0))
                .with_scale(Vec3::new(0.65, 1.8, 1.0)),
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Ramp => Color::srgb(0.3, 0.75, 0.98),
            Self::Platform => Color::srgb(0.98, 0.68, 0.22),
            Self::Pillar => Color::srgb(0.45, 0.9, 0.48),
        }
    }

    fn drop_transform(self, scene_transform: Transform) -> Transform {
        let mesh_transform = self.local_transform();
        let local_drop = mesh_transform.translation
            + mesh_transform.rotation * Vec3::Y * (mesh_transform.scale.y * 0.5 + DROP_HEIGHT);
        Transform::from_translation(scene_transform.transform_point(local_drop))
    }
}

/// Marks the root of the scene whose descendants receive generated colliders.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SceneColliderLabRoot;

/// Marks mesh nodes authored in the scene hierarchy.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneColliderLabMesh {
    pub kind: SceneMeshKind,
}

/// Added when hierarchy collider generation has completed for the scene root.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SceneColliderGenerated;

/// Marks one dynamic body dropped onto a scene mesh.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneColliderLabDrop {
    pub target: SceneMeshKind,
}

/// Owns the simple scene hierarchy and Avian's automatic descendant colliders.
pub struct SceneColliderStationPlugin;

impl Plugin for SceneColliderStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_systems(
                Startup,
                (spawn_scene_collider_lab, spawn_scene_collider_station_ui),
            )
            .add_systems(PreUpdate, add_scene_collider_constructor)
            .add_systems(
                Update,
                draw_scene_collider_labels.run_if(resource_exists::<GizmoConfigStore>),
            )
            .add_systems(PostUpdate, reset_scene_collider_lab);
    }
}

fn spawn_scene_collider_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let scene_transform = Transform::from_translation(SCENE_POSITION)
        .with_rotation(Quat::from_rotation_y(SCENE_ROTATION))
        .with_scale(Vec3::new(1.15, 1.0, 0.85));

    // This entity represents the loaded scene instance. The hierarchy
    // constructor visits every descendant Mesh3d and creates its Collider.
    let scene = commands
        .spawn((
            SceneColliderLabRoot,
            RigidBody::Static,
            scene_transform,
            Name::new("Scene Collider Lab - Loaded Scene"),
        ))
        .id();
    commands
        .entity(scene)
        .observe(mark_scene_colliders_generated);

    for kind in SceneMeshKind::ALL {
        let local_transform = kind.local_transform();
        let mut mesh_node = commands.spawn((
            ChildOf(scene),
            SceneColliderLabMesh { kind },
            local_transform,
            Name::new(format!("Scene Mesh - {}", kind.label())),
        ));

        if let Some(meshes) = meshes.as_mut() {
            mesh_node.insert(Mesh3d(meshes.add(Cuboid::default())));
        }
        if let Some(materials) = materials.as_mut() {
            mesh_node.insert(MeshMaterial3d(materials.add(kind.color())));
        }
    }

    for kind in SceneMeshKind::ALL {
        let transform = kind.drop_transform(scene_transform);
        let mut drop = commands.spawn((
            SceneColliderLabDrop { target: kind },
            StationObject::new('C', transform),
            RigidBody::Dynamic,
            Collider::sphere(DROP_RADIUS),
            Friction::new(0.25),
            Restitution::ZERO,
            SleepingDisabled,
            LockedAxes::new()
                .lock_translation_x()
                .lock_translation_z()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            transform,
            Name::new(format!("Scene Collider Drop - {}", kind.label())),
        ));

        if let Some(meshes) = meshes.as_mut() {
            mesh_node(&mut drop, meshes.add(Sphere::new(DROP_RADIUS)));
        }
        if let Some(materials) = materials.as_mut() {
            drop.insert(MeshMaterial3d(
                materials.add(kind.color().with_luminance(0.75)),
            ));
        }
    }
}

fn mesh_node(drop: &mut EntityCommands, mesh: Handle<Mesh>) {
    drop.insert(Mesh3d(mesh));
}

fn add_scene_collider_constructor(world: &mut World) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<SceneColliderLabRoot>,
            Without<ColliderConstructorHierarchy>,
            Without<SceneColliderGenerated>,
        )>()
        .iter(world)
        .collect();
    for root in roots {
        world
            .entity_mut(root)
            .insert(ColliderConstructorHierarchy::new(
                ColliderConstructor::TrimeshFromMesh,
            ));
    }
}

fn mark_scene_colliders_generated(
    _ready: On<ColliderConstructorHierarchyReady>,
    mut commands: Commands,
) {
    commands
        .entity(_ready.entity)
        .insert(SceneColliderGenerated);
}

#[derive(Component)]
struct SceneColliderStationPanel;

fn spawn_scene_collider_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(800.0),
                top: px(458.0),
                width: px(350.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            SceneColliderStationPanel,
            Name::new("Scene Collider Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("C — SCENE COLLIDER GENERATION"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "One scene hierarchy owns three mesh nodes. Avian generates a collider for every mesh without per-mesh collider setup.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            for kind in SceneMeshKind::ALL {
                parent.spawn((
                    Text::new(format!("{}    generated collider + drop", kind.label())),
                    TextFont::from_font_size(13.0),
                    TextColor(kind.color()),
                ));
            }
            parent.spawn((
                Text::new("F1: show generated colliders   R: reset station C"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
        });
}

fn draw_scene_collider_labels(
    mut gizmos: Gizmos,
    roots: Query<(&GlobalTransform, Option<&SceneColliderGenerated>), With<SceneColliderLabRoot>>,
    meshes: Query<(&SceneColliderLabMesh, &GlobalTransform)>,
    drops: Query<(&SceneColliderLabDrop, &Transform)>,
) {
    for (transform, generated) in &roots {
        let status = if generated.is_some() {
            "SCENE HIERARCHY\nCOLLIDERS READY"
        } else {
            "SCENE HIERARCHY\nGENERATING COLLIDERS"
        };
        gizmos.text(
            Isometry3d::new(
                transform.translation() + Vec3::new(0.0, 2.6, 0.0),
                Quat::IDENTITY,
            ),
            status,
            0.3,
            Vec2::ZERO,
            Color::srgb(1.0, 0.42, 0.15),
        );
    }

    for (mesh, transform) in &meshes {
        gizmos.text(
            Isometry3d::new(transform.translation() + Vec3::Y * 0.3, Quat::IDENTITY),
            &format!("{}\nMESH / F1 COLLIDER", mesh.kind.label()),
            0.2,
            Vec2::ZERO,
            mesh.kind.color(),
        );
    }

    for (drop, transform) in &drops {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.32, Quat::IDENTITY),
            &format!("{} DROP", drop.target.label()),
            0.18,
            Vec2::ZERO,
            drop.target.color().with_luminance(0.8),
        );
    }
}

#[allow(clippy::type_complexity)]
fn reset_scene_collider_lab(
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
        With<SceneColliderLabDrop>,
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
    use std::time::Duration;

    use bevy::{
        mesh::MeshPlugin, time::TimeUpdateStrategy, world_serialization::WorldSerializationPlugin,
    };

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn scene_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            WorldSerializationPlugin,
            MeshPlugin,
            PhysicsPlugins::default(),
            SceneColliderStationPlugin,
        ))
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    #[test]
    fn scene_meshes_receive_generated_colliders_without_manual_setup() {
        let mut app = scene_app();
        run_steps(&mut app, 3);

        let world = app.world_mut();
        let mut roots = world.query::<(&SceneColliderLabRoot, &SceneColliderGenerated)>();
        assert_eq!(roots.iter(world).count(), 1);

        let mut meshes = world.query::<(&SceneColliderLabMesh, &Mesh3d, &Collider)>();
        let generated: Vec<_> = meshes.iter(world).map(|(mesh, _, _)| mesh.kind).collect();
        assert_eq!(generated, SceneMeshKind::ALL);
    }

    #[test]
    fn generated_colliders_keep_scene_hierarchy_transforms() {
        let mut app = scene_app();
        run_steps(&mut app, 2);

        let world = app.world_mut();
        let scene_transform = world
            .query_filtered::<&Transform, With<SceneColliderLabRoot>>()
            .single(world)
            .expect("scene root transform");
        let scene_global = GlobalTransform::from(*scene_transform);
        let mut meshes = world.query::<(&SceneColliderLabMesh, &Transform, &GlobalTransform)>();

        for (mesh, local_transform, global_transform) in meshes.iter(world) {
            let expected = scene_global.transform_point(local_transform.translation);
            assert!(
                global_transform.translation().distance(expected) < 1e-5,
                "{} lost its hierarchy transform: {:?} != {:?}",
                mesh.kind.label(),
                global_transform.translation(),
                expected
            );
        }
    }

    #[test]
    fn bodies_drop_onto_each_generated_scene_collider() {
        let mut app = scene_app();
        run_steps(&mut app, 2);

        let mut drops = app.world_mut().query::<(Entity, &SceneColliderLabDrop)>();
        let drops: Vec<_> = drops
            .iter(app.world())
            .map(|(entity, drop)| (entity, drop.target))
            .collect();
        let mut meshes = app.world_mut().query::<(Entity, &SceneColliderLabMesh)>();
        let meshes: Vec<_> = meshes
            .iter(app.world())
            .map(|(entity, mesh)| (entity, mesh.kind))
            .collect();

        let initial_y: Vec<_> = drops
            .iter()
            .map(|(entity, _)| app.world().entity(*entity).get::<Position>().unwrap().0.y)
            .collect();
        run_steps(&mut app, 240);

        for ((drop_entity, target), initial_y) in drops.iter().zip(initial_y) {
            let final_position = app
                .world()
                .entity(*drop_entity)
                .get::<Position>()
                .unwrap()
                .0;
            assert!(
                final_position.y < initial_y - 1.0,
                "{} did not fall: {final_position:?}",
                target.label()
            );
            let mesh_entity = meshes
                .iter()
                .find(|(_, kind)| kind == target)
                .map(|(entity, _)| *entity)
                .expect("matching scene mesh");
            assert!(
                app.world()
                    .resource::<ContactGraph>()
                    .contains(*drop_entity, mesh_entity),
                "{} drop did not contact its generated collider",
                target.label()
            );
        }
    }
}
