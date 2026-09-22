use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const STATION_CODE: char = 'E';
const ANCHOR_CENTER: Vec3 = Vec3::new(1.65, 2.15, -1.05);
const ANCHOR_SIZE: Vec3 = Vec3::new(0.5, 0.5, 0.5);
const OBJECT_SIZE: Vec3 = Vec3::new(0.7, 1.8, 0.7);
const OBJECT_CENTER: Vec3 = Vec3::new(
    ANCHOR_CENTER.x,
    ANCHOR_CENTER.y - OBJECT_SIZE.y * 0.5 - ANCHOR_SIZE.y * 0.5,
    ANCHOR_CENTER.z,
);
const OBJECT_HALF_HEIGHT: f32 = OBJECT_SIZE.y * 0.5;
const OBJECT_DENSITY: f32 = 0.75;
const ANGULAR_IMPULSE: f32 = 8.0;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const ANCHOR_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const ROTATION_COLOR: Color = Color::srgb(0.55, 0.8, 1.0);

/// Identifies the static body that provides the spherical joint's anchor.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SphericalJointAnchor;

/// Identifies the hanging dynamic body in the spherical-joint demonstration.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SphericalJointObject;

/// Marks the Avian joint that keeps the hanging object connected to its anchor.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SphericalJointDemo;

/// A headless and UI-friendly command for stressing one rotation axis or
/// restoring the hanging object to its starting pose.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SphericalJointStationAction {
    pub action: SphericalJointAction,
}

/// The controls exposed by the spherical-joint station.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SphericalJointAction {
    AngularImpulseX,
    AngularImpulseY,
    AngularImpulseZ,
}

/// Owns the hanging-object spherical-joint demonstration in station E.
pub struct SphericalJointStationPlugin;

impl Plugin for SphericalJointStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_message::<SphericalJointStationAction>()
            .add_systems(
                Startup,
                (spawn_spherical_joint_demo, spawn_spherical_joint_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_spherical_joint_station_actions,
                    update_spherical_joint_button_colors,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, apply_spherical_joint_station_actions)
            .add_systems(
                Update,
                draw_spherical_joint.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_spherical_joint_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let anchor_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(ANCHOR_SIZE)));
    let object_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(OBJECT_SIZE)));
    let anchor_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.24, 0.65, 0.78)));
    let object_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.92, 0.38, 0.18)));

    let anchor = commands
        .spawn((
            SphericalJointAnchor,
            RigidBody::Static,
            Collider::cuboid(ANCHOR_SIZE.x, ANCHOR_SIZE.y, ANCHOR_SIZE.z),
            Transform::from_translation(ANCHOR_CENTER),
            Name::new("Spherical Joint Anchor"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (anchor_mesh, anchor_material) {
        commands
            .entity(anchor)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    let object_transform = Transform::from_translation(OBJECT_CENTER);
    let object = commands
        .spawn((
            SphericalJointObject,
            StationObject::new(STATION_CODE, object_transform),
            RigidBody::Dynamic,
            Collider::cuboid(OBJECT_SIZE.x, OBJECT_SIZE.y, OBJECT_SIZE.z),
            ColliderDensity(OBJECT_DENSITY),
            Friction::new(0.55),
            Restitution::new(0.05),
            LinearDamping(0.12),
            AngularDamping(0.08),
            object_transform,
            Name::new("Spherical Joint Hanging Object"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (object_mesh, object_material) {
        commands
            .entity(object)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    // Both local anchors coincide at the bottom of the static anchor and the
    // top of the hanging object. A spherical joint removes relative
    // translation there while leaving all three rotational axes unrestricted.
    commands.spawn((
        SphericalJoint::new(anchor, object)
            .with_local_anchor1(Vec3::NEG_Y * (ANCHOR_SIZE.y * 0.5))
            .with_local_anchor2(Vec3::Y * OBJECT_HALF_HEIGHT),
        JointForces::new(),
        JointCollisionDisabled,
        SphericalJointDemo,
        Name::new("Spherical Joint Hanging Connection"),
    ));
}

#[derive(Component)]
struct SphericalJointStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum SphericalJointStationControl {
    Apply(SphericalJointAction),
    Reset,
}

fn spawn_spherical_joint_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(820.0),
                top: px(16.0),
                width: px(360.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            SphericalJointStationPanel,
            Name::new("Spherical Joint Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("E — SPHERICAL JOINT"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "A hanging body stays attached at the yellow anchor while it rotates freely around X, Y, and Z.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            let mut row = parent.spawn(spherical_joint_control_row());
            row.with_children(|row| {
                spawn_spherical_joint_control_button(
                    row,
                    "impulse X",
                    SphericalJointStationControl::Apply(
                        SphericalJointAction::AngularImpulseX,
                    ),
                );
                spawn_spherical_joint_control_button(
                    row,
                    "impulse Y",
                    SphericalJointStationControl::Apply(
                        SphericalJointAction::AngularImpulseY,
                    ),
                );
            });
            let mut row = parent.spawn(spherical_joint_control_row());
            row.with_children(|row| {
                spawn_spherical_joint_control_button(
                    row,
                    "impulse Z",
                    SphericalJointStationControl::Apply(
                        SphericalJointAction::AngularImpulseZ,
                    ),
                );
                spawn_spherical_joint_control_button(
                    row,
                    "reset station E",
                    SphericalJointStationControl::Reset,
                );
            });
            parent.spawn((
                Text::new("Yellow: connected anchor   Blue: free rotation   F1: joint debug lines"),
                TextFont::from_font_size(12.0),
                TextColor(ROTATION_COLOR),
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
        });
}

fn spherical_joint_control_row() -> Node {
    Node {
        width: percent(100.0),
        height: px(26.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_spherical_joint_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: SphericalJointStationControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(132.0),
                height: px(22.0),
                margin: UiRect::right(px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Spherical joint control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_spherical_joint_station_actions(
    controls: Query<(&Interaction, &SphericalJointStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<SphericalJointStationAction>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *control {
            SphericalJointStationControl::Apply(action) => {
                actions.write(SphericalJointStationAction { action });
            }
            SphericalJointStationControl::Reset => {
                resets.write(ResetStation { code: STATION_CODE });
            }
        }
    }
}

fn update_spherical_joint_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<SphericalJointStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn apply_spherical_joint_station_actions(
    mut actions: MessageReader<SphericalJointStationAction>,
    mut objects: Query<Forces, With<SphericalJointObject>>,
) {
    for action in actions.read().copied() {
        let impulse = match action.action {
            SphericalJointAction::AngularImpulseX => Vec3::X * ANGULAR_IMPULSE,
            SphericalJointAction::AngularImpulseY => Vec3::Y * ANGULAR_IMPULSE,
            SphericalJointAction::AngularImpulseZ => Vec3::Z * ANGULAR_IMPULSE,
        };
        for mut forces in &mut objects {
            forces.apply_angular_impulse(impulse);
        }
    }
}

fn draw_spherical_joint(
    mut gizmos: Gizmos,
    joints: Query<&SphericalJoint, With<SphericalJointDemo>>,
    bodies: Query<&Transform>,
) {
    let Some(joint) = joints.iter().next() else {
        return;
    };
    let Ok([anchor_body, object_body]) = bodies.get_many([joint.body1, joint.body2]) else {
        return;
    };
    let (Some(local_anchor1), Some(local_anchor2)) = (joint.local_anchor1(), joint.local_anchor2())
    else {
        return;
    };
    let anchor1 = anchor_body.translation + anchor_body.rotation * local_anchor1;
    let anchor2 = object_body.translation + object_body.rotation * local_anchor2;
    let anchor = (anchor1 + anchor2) * 0.5;
    let axis_length = 0.65;

    gizmos.line(anchor1, anchor2, ANCHOR_COLOR);
    gizmos.sphere(anchor, 0.14, ANCHOR_COLOR);
    for (axis, color) in [
        (Vec3::X, Color::srgb(1.0, 0.2, 0.2)),
        (Vec3::Y, Color::srgb(0.2, 1.0, 0.3)),
        (Vec3::Z, ROTATION_COLOR),
    ] {
        let axis = anchor_body.rotation * axis;
        gizmos.line(
            anchor - axis * axis_length,
            anchor + axis * axis_length,
            color,
        );
    }
    gizmos.text(
        Isometry3d::new(anchor + Vec3::Y * 0.85, Quat::IDENTITY),
        "FREE X/Y/Z ROTATION",
        0.2,
        Vec2::ZERO,
        ROTATION_COLOR,
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;
    const JOINT_ANCHOR: Vec3 = Vec3::new(
        ANCHOR_CENTER.x,
        ANCHOR_CENTER.y - ANCHOR_SIZE.y * 0.5,
        ANCHOR_CENTER.z,
    );

    fn spherical_joint_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            SphericalJointStationPlugin,
            StationLayoutPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
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

    fn object(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut objects = world.query_filtered::<Entity, With<SphericalJointObject>>();
        objects.iter(world).next().expect("spherical-joint object")
    }

    fn anchor(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut anchors = world.query_filtered::<Entity, With<SphericalJointAnchor>>();
        anchors.iter(world).next().expect("spherical-joint anchor")
    }

    fn joint(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<SphericalJointDemo>>();
        joints.iter(world).next().expect("spherical-joint entity")
    }

    fn joint_anchor(app: &mut App) -> Vec3 {
        let joint_entity = joint(app);
        let joint = app.world().entity(joint_entity);
        let joint = joint.get::<SphericalJoint>().expect("spherical joint");
        let anchor_body = app.world().entity(joint.body1);
        let object_body = app.world().entity(joint.body2);
        let anchor_transform = anchor_body.get::<Transform>().expect("anchor transform");
        let object_transform = object_body.get::<Transform>().expect("object transform");
        let anchor1 = anchor_transform.translation
            + anchor_transform.rotation * joint.local_anchor1().expect("anchor 1");
        let anchor2 = object_transform.translation
            + object_transform.rotation * joint.local_anchor2().expect("anchor 2");
        (anchor1 + anchor2) * 0.5
    }

    #[test]
    fn station_spawns_static_anchor_dynamic_object_and_unlimited_spherical_joint() {
        let mut app = spherical_joint_app();
        app.update();

        let anchor = anchor(&mut app);
        let object = object(&mut app);
        let joint_entity = joint(&mut app);
        let entity = app.world().entity(joint_entity);
        let spherical_joint = entity.get::<SphericalJoint>().unwrap();

        assert_eq!(
            [spherical_joint.body1, spherical_joint.body2],
            [anchor, object]
        );
        assert_eq!(spherical_joint.local_anchor1(), Some(Vec3::NEG_Y * 0.25));
        assert_eq!(
            spherical_joint.local_anchor2(),
            Some(Vec3::Y * OBJECT_HALF_HEIGHT)
        );
        assert_eq!(spherical_joint.swing_limit, None);
        assert_eq!(spherical_joint.twist_limit, None);
        assert!(entity.contains::<JointForces>());
        assert!(entity.contains::<JointCollisionDisabled>());
        assert_eq!(
            app.world().entity(anchor).get::<RigidBody>(),
            Some(&RigidBody::Static)
        );
        assert_eq!(
            app.world().entity(object).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
        assert!(app.world().entity(object).contains::<StationObject>());
        assert!((joint_anchor(&mut app) - JOINT_ANCHOR).length() < 0.01);
    }

    #[test]
    fn angular_impulses_from_multiple_directions_keep_the_object_connected_and_rotate_it() {
        let mut app = spherical_joint_app();
        app.update();
        let object = object(&mut app);
        let initial_rotation = app.world().entity(object).get::<Rotation>().unwrap().0;
        let initial_anchor = joint_anchor(&mut app);

        for (action, label) in [
            (SphericalJointAction::AngularImpulseX, "X"),
            (SphericalJointAction::AngularImpulseY, "Y"),
            (SphericalJointAction::AngularImpulseZ, "Z"),
        ] {
            app.world_mut()
                .resource_mut::<Messages<SphericalJointStationAction>>()
                .write(SphericalJointStationAction { action });
            run_steps(&mut app, 90);

            assert!(
                (joint_anchor(&mut app) - initial_anchor).length() < 0.08,
                "angular impulse {label} separated the object from its anchor: {:?}",
                joint_anchor(&mut app)
            );
        }

        let final_rotation = app.world().entity(object).get::<Rotation>().unwrap().0;
        assert!(
            final_rotation.angle_between(initial_rotation) > 0.2,
            "angular impulses did not produce free rotation: {final_rotation:?}"
        );
    }

    #[test]
    fn reset_restores_the_hanging_object_after_rotation() {
        let mut app = spherical_joint_app();
        app.update();
        let object = object(&mut app);
        app.world_mut()
            .resource_mut::<Messages<SphericalJointStationAction>>()
            .write(SphericalJointStationAction {
                action: SphericalJointAction::AngularImpulseY,
            });
        run_steps(&mut app, 60);
        assert!(
            app.world()
                .entity(object)
                .get::<Rotation>()
                .unwrap()
                .0
                .angle_between(Quat::IDENTITY)
                > 0.1
        );

        app.world_mut()
            .resource_mut::<Messages<ResetStation>>()
            .write(ResetStation { code: STATION_CODE });
        app.update();

        let entity = app.world().entity(object);
        assert!((entity.get::<Position>().unwrap().0 - OBJECT_CENTER).length() < 0.01);
        assert!(
            entity
                .get::<Rotation>()
                .unwrap()
                .0
                .angle_between(Quat::IDENTITY)
                < 0.01
        );
        assert!((joint_anchor(&mut app) - JOINT_ANCHOR).length() < 0.01);
    }
}
