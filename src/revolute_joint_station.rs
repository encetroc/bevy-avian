use avian3d::prelude::*;
use bevy::prelude::*;

use crate::camera::FixedFollowCamera;
use crate::player::{Player, PlayerInput, PlayerMovementSet, PlayerMovementSettings};
use crate::stations::{ResetStation, StationObject};

const STATION_CODE: char = 'E';
const DOOR_CENTER: Vec3 = Vec3::new(0.0, 1.6, 1.6);
const DOOR_SIZE: Vec3 = Vec3::new(2.8, 2.8, 0.3);
const DOOR_HALF_WIDTH: f32 = DOOR_SIZE.x * 0.5;
const HINGE_POST_SIZE: Vec3 = Vec3::new(0.42, 3.2, 0.42);
const HINGE_ANCHOR: Vec3 = Vec3::new(-DOOR_HALF_WIDTH, DOOR_CENTER.y, DOOR_CENTER.z);
const HINGE_AXIS: Vec3 = Vec3::Y;
const OBJECT_DENSITY: f32 = 0.75;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const HINGE_AXIS_COLOR: Color = Color::srgb(1.0, 0.2, 0.75);
const HINGE_ANCHOR_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);

/// Identifies the dynamic panel in the revolute-joint demonstration.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RevoluteJointDoor;

/// Identifies the static body that provides the door's hinge pivot.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RevoluteJointHingePost;

/// Marks the Avian joint constraining the door to its hinge post.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RevoluteJointDemo;

/// A headless and UI-friendly command for resetting the hinge demonstration.
#[derive(Message, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RevoluteJointStationAction {
    pub action: RevoluteJointAction,
}

/// The controls exposed by the revolute-joint station.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RevoluteJointAction {
    #[default]
    Reset,
}

/// Owns the hinge-style door demonstration in station E.
pub struct RevoluteJointStationPlugin;

impl Plugin for RevoluteJointStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_message::<RevoluteJointStationAction>()
            .add_systems(
                Startup,
                (spawn_revolute_joint_demo, spawn_revolute_joint_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_revolute_joint_station_actions,
                    update_revolute_joint_button_colors,
                    reset_revolute_joint_station,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                push_revolute_joint_demo.after(PlayerMovementSet),
            )
            .add_systems(
                Update,
                draw_revolute_joint.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_revolute_joint_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let door_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(DOOR_SIZE)));
    let post_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(HINGE_POST_SIZE)));
    let door_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.92, 0.38, 0.18)));
    let post_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.24, 0.65, 0.78)));

    let post = commands
        .spawn((
            RevoluteJointHingePost,
            RigidBody::Static,
            Collider::cuboid(HINGE_POST_SIZE.x, HINGE_POST_SIZE.y, HINGE_POST_SIZE.z),
            Transform::from_translation(HINGE_ANCHOR),
            Name::new("Revolute Joint Hinge Post"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (post_mesh, post_material) {
        commands
            .entity(post)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    let door_transform = Transform::from_translation(DOOR_CENTER);
    let mut door = commands.spawn((
        RevoluteJointDoor,
        StationObject::new(STATION_CODE, door_transform),
        RigidBody::Dynamic,
        Collider::cuboid(DOOR_SIZE.x, DOOR_SIZE.y, DOOR_SIZE.z),
        ColliderDensity(OBJECT_DENSITY),
        Friction::new(0.55),
        Restitution::new(0.05),
        LinearDamping(0.12),
        AngularDamping(0.2),
        door_transform,
        Name::new("Revolute Joint Door"),
    ));
    if let (Some(mesh), Some(material)) = (door_mesh, door_material) {
        door.insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
    let door = door.id();

    // The post and door anchors coincide at the left edge of the door. The
    // local Y hinge axis is therefore world vertical in the initial pose, and
    // the door can swing while staying attached to the static post.
    commands.spawn((
        RevoluteJoint::new(post, door)
            .with_hinge_axis(HINGE_AXIS)
            .with_local_anchor1(Vec3::ZERO)
            .with_local_anchor2(Vec3::NEG_X * DOOR_HALF_WIDTH),
        JointForces::new(),
        JointCollisionDisabled,
        RevoluteJointDemo,
        Name::new("Revolute Joint Door Hinge"),
    ));
}

#[derive(Component)]
struct RevoluteJointStationPanel;

#[derive(Component)]
struct RevoluteJointStationControl(RevoluteJointAction);

fn spawn_revolute_joint_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(420.0),
                top: px(440.0),
                width: px(380.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            RevoluteJointStationPanel,
            Name::new("Revolute Joint Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("E — REVOLUTE JOINT"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Push, grab, or throw the door. Its hinge keeps the anchor connected while allowing rotation around vertical Y.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("Magenta axis: hinge rotation\nF1: Avian collider and joint debug lines"),
                TextFont::from_font_size(12.0),
                TextColor(HINGE_AXIS_COLOR),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            spawn_revolute_joint_control_button(
                parent,
                "reset station E",
                RevoluteJointAction::Reset,
            );
        });
}

fn spawn_revolute_joint_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    action: RevoluteJointAction,
) {
    parent
        .spawn((
            Button,
            RevoluteJointStationControl(action),
            Node {
                width: px(132.0),
                height: px(22.0),
                margin: UiRect::right(px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Revolute joint control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_revolute_joint_station_actions(
    controls: Query<(&Interaction, &RevoluteJointStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<RevoluteJointStationAction>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            actions.write(RevoluteJointStationAction { action: control.0 });
        }
    }
}

fn update_revolute_joint_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<RevoluteJointStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn reset_revolute_joint_station(
    mut actions: MessageReader<RevoluteJointStationAction>,
    mut resets: MessageWriter<ResetStation>,
) {
    if actions
        .read()
        .any(|action| action.action == RevoluteJointAction::Reset)
    {
        resets.write(ResetStation { code: STATION_CODE });
    }
}

fn push_revolute_joint_demo(
    settings: Option<Res<PlayerMovementSettings>>,
    camera: Option<Single<&Transform, With<FixedFollowCamera>>>,
    players: Query<(&Transform, &PlayerInput), With<Player>>,
    mut doors: Query<(&Transform, &mut LinearVelocity), With<RevoluteJointDoor>>,
) {
    let Some(settings) = settings else {
        return;
    };
    let camera_rotation = camera
        .map(|camera| camera.rotation)
        .unwrap_or(Quat::IDENTITY);

    for (player_transform, input) in &players {
        let local_direction = Vec3::new(input.0.x, 0.0, -input.0.y);
        let mut push_direction = camera_rotation * local_direction;
        push_direction.y = 0.0;
        push_direction = push_direction.normalize_or_zero();
        if push_direction == Vec3::ZERO {
            continue;
        }

        for (door_transform, mut velocity) in &mut doors {
            let offset = door_transform.translation - player_transform.translation;
            let horizontal_offset = Vec2::new(offset.x, offset.z);
            let push_distance = DOOR_HALF_WIDTH + crate::player::PLAYER_RADIUS + 0.08;
            let vertical_overlap = offset.y.abs()
                < crate::player::PLAYER_CAPSULE_LENGTH * 0.5
                    + crate::player::PLAYER_RADIUS
                    + DOOR_SIZE.y * 0.5;
            if !vertical_overlap || horizontal_offset.length() > push_distance {
                continue;
            }

            let target_speed = settings.speed * 0.8;
            let current_speed = velocity.0.dot(push_direction);
            if current_speed < target_speed {
                velocity.0 += push_direction * (target_speed - current_speed);
            }
        }
    }
}

fn draw_revolute_joint(
    mut gizmos: Gizmos,
    joints: Query<&RevoluteJoint, With<RevoluteJointDemo>>,
    bodies: Query<&Transform>,
) {
    let Some(joint) = joints.iter().next() else {
        return;
    };
    let Ok([body1, body2]) = bodies.get_many([joint.body1, joint.body2]) else {
        return;
    };
    let (JointAnchor::Local(local_anchor1), JointAnchor::Local(local_anchor2)) =
        (joint.frame1.anchor, joint.frame2.anchor)
    else {
        return;
    };
    let anchor1 = body1.translation + body1.rotation * local_anchor1;
    let anchor2 = body2.translation + body2.rotation * local_anchor2;
    let axis = body1.rotation
        * joint
            .local_hinge_axis1()
            .unwrap_or(joint.hinge_axis)
            .normalize_or_zero();
    let anchor = (anchor1 + anchor2) * 0.5;
    let axis_start = anchor - axis * 1.4;
    let axis_end = anchor + axis * 1.4;

    gizmos.line(anchor1, anchor2, HINGE_ANCHOR_COLOR);
    gizmos.sphere(anchor, 0.13, HINGE_ANCHOR_COLOR);
    gizmos.line(axis_start, axis_end, HINGE_AXIS_COLOR);
    gizmos.sphere(axis_start, 0.08, HINGE_AXIS_COLOR);
    gizmos.sphere(axis_end, 0.08, HINGE_AXIS_COLOR);
    gizmos.text(
        Isometry3d::new(anchor + axis * 1.6, Quat::IDENTITY),
        "Y HINGE AXIS",
        0.2,
        Vec2::ZERO,
        HINGE_AXIS_COLOR,
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn revolute_joint_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            RevoluteJointStationPlugin,
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

    fn door(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut doors = world.query_filtered::<Entity, With<RevoluteJointDoor>>();
        doors.iter(world).next().expect("revolute-joint door")
    }

    fn hinge_post(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut posts = world.query_filtered::<Entity, With<RevoluteJointHingePost>>();
        posts.iter(world).next().expect("revolute-joint hinge post")
    }

    fn joint(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<RevoluteJointDemo>>();
        joints.iter(world).next().expect("revolute-joint entity")
    }

    fn hinge_anchor(app: &mut App) -> Vec3 {
        let joint_entity = joint_entity(app);
        let joint = app.world().entity(joint_entity);
        let joint = joint.get::<RevoluteJoint>().expect("revolute joint");
        let door = app.world().entity(joint.body2);
        let transform = door.get::<Transform>().expect("door transform");
        let local_anchor = joint.local_anchor2().expect("door local anchor");
        transform.translation + transform.rotation * local_anchor
    }

    fn joint_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<RevoluteJointDemo>>();
        joints.iter(world).next().expect("revolute-joint entity")
    }

    #[test]
    fn station_spawns_a_static_hinge_dynamic_door_and_vertical_revolute_joint() {
        let mut app = revolute_joint_app();
        app.update();

        let door = door(&mut app);
        let post = hinge_post(&mut app);
        let joint_entity = joint(&mut app);
        let joint = app
            .world()
            .entity(joint_entity)
            .get::<RevoluteJoint>()
            .expect("revolute joint");

        assert_eq!([joint.body1, joint.body2], [post, door]);
        assert_eq!(joint.hinge_axis, HINGE_AXIS);
        assert_eq!(joint.local_anchor1(), Some(Vec3::ZERO));
        assert_eq!(joint.local_anchor2(), Some(Vec3::NEG_X * DOOR_HALF_WIDTH));
        assert!(app.world().entity(joint_entity).contains::<JointForces>());
        assert!(
            app.world()
                .entity(joint_entity)
                .contains::<JointCollisionDisabled>()
        );
        assert_eq!(
            app.world().entity(post).get::<RigidBody>(),
            Some(&RigidBody::Static)
        );
        assert_eq!(
            app.world().entity(door).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
        assert!(app.world().entity(door).contains::<StationObject>());
        assert!((hinge_anchor(&mut app) - HINGE_ANCHOR).length() < 0.01);
    }

    #[test]
    fn off_axis_angular_impacts_leave_only_the_vertical_hinge_rotation() {
        let mut app = revolute_joint_app();
        app.update();
        let door = door(&mut app);
        app.world_mut()
            .entity_mut(door)
            .insert(AngularVelocity(Vec3::new(5.0, 4.0, 3.0)));

        run_steps(&mut app, 120);

        let rotation = app.world().entity(door).get::<Rotation>().unwrap().0;
        let vertical_axis = rotation * Vec3::Y;
        assert!(
            vertical_axis.distance(Vec3::Y) < 0.08,
            "door tilted away from Y hinge axis: {vertical_axis:?}"
        );
        assert!(
            (hinge_anchor(&mut app) - HINGE_ANCHOR).length() < 0.08,
            "door separated from the hinge: {:?}",
            hinge_anchor(&mut app)
        );
    }

    #[test]
    fn impacts_from_both_sides_keep_the_door_attached_to_the_hinge() {
        let mut app = revolute_joint_app();
        app.add_systems(Startup, spawn_impact_course);
        app.update();
        let door = door(&mut app);

        for velocity in [Vec3::new(0.0, 0.0, 14.0), Vec3::new(0.0, 0.0, -14.0)] {
            app.world_mut()
                .entity_mut(door)
                .insert(LinearVelocity(velocity));
            run_steps(&mut app, 120);
            assert!(
                (hinge_anchor(&mut app) - HINGE_ANCHOR).length() < 0.12,
                "door separated after impact from {velocity:?}: {:?}",
                hinge_anchor(&mut app)
            );
            let rotation = app.world().entity(door).get::<Rotation>().unwrap().0;
            assert!(
                (rotation * Vec3::Y).distance(Vec3::Y) < 0.12,
                "impact from {velocity:?} tilted the door"
            );
        }
    }

    fn spawn_impact_course(mut commands: Commands) {
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 4.0, 0.4),
            Transform::from_xyz(0.0, 1.6, 5.0),
            Name::new("Revolute Joint Impact Wall"),
        ));
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 4.0, 0.4),
            Transform::from_xyz(0.0, 1.6, -1.8),
            Name::new("Revolute Joint Return Wall"),
        ));
    }

    #[test]
    fn player_pushes_the_door_from_both_horizontal_directions() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            RevoluteJointStationPlugin,
            crate::player::PlayerPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app.update();

        let door = door(&mut app);
        let player = {
            let world = app.world_mut();
            let mut players = world.query_filtered::<Entity, With<Player>>();
            players.iter(world).next().expect("push test player")
        };

        let initial_anchor = hinge_anchor(&mut app);
        for (player_position, key) in [
            (Vec3::new(0.0, 1.0, 0.1), KeyCode::KeyS),
            (Vec3::new(0.0, 1.0, 3.1), KeyCode::KeyW),
        ] {
            app.world_mut().entity_mut(player).insert((
                Position(player_position),
                Transform::from_translation(player_position),
            ));
            run_steps(&mut app, 2);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            run_steps(&mut app, 90);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
            run_steps(&mut app, 2);
        }

        assert!(
            (hinge_anchor(&mut app) - initial_anchor).length() < 0.12,
            "player pushes separated the door from its hinge"
        );
        let rotation = app.world().entity(door).get::<Rotation>().unwrap().0;
        assert!(
            (rotation * Vec3::Y).distance(Vec3::Y) < 0.12,
            "player pushes tilted the door away from its hinge axis"
        );
    }
}
