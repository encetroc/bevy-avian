use avian3d::prelude::*;
use bevy::prelude::*;

use crate::camera::FixedFollowCamera;
use crate::player::{Player, PlayerInput, PlayerMovementSet, PlayerMovementSettings};
use crate::stations::{ResetStation, StationObject};

const STATION_CODE: char = 'E';
const OBJECT_CENTER: Vec3 = Vec3::new(0.0, 0.5, 0.7);
const OBJECT_SEPARATION: f32 = 1.6;
const OBJECT_SIZE: f32 = 0.65;
const OBJECT_HALF_SIZE: f32 = OBJECT_SIZE * 0.5;
const INITIAL_DISTANCE: f32 = OBJECT_SEPARATION - OBJECT_SIZE;
const DISTANCE_STEP: f32 = 0.25;
const MIN_DISTANCE: f32 = 0.5;
const MAX_DISTANCE: f32 = 3.0;
const OBJECT_DENSITY: f32 = 0.75;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const ENDPOINT_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);
const DISTANCE_LINE_COLOR: Color = Color::srgb(0.95, 0.35, 0.85);

/// Identifies one of the two dynamic bodies in the distance-joint demonstration.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DistanceJointBody {
    Left,
    Right,
}

impl DistanceJointBody {
    const ALL: [Self; 2] = [Self::Left, Self::Right];

    const fn label(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }

    fn initial_transform(self) -> Transform {
        let x = match self {
            Self::Left => -OBJECT_SEPARATION * 0.5,
            Self::Right => OBJECT_SEPARATION * 0.5,
        };
        Transform::from_translation(OBJECT_CENTER + Vec3::X * x)
    }

    fn local_anchor(self) -> Vec3 {
        match self {
            Self::Left => Vec3::X * OBJECT_HALF_SIZE,
            Self::Right => Vec3::NEG_X * OBJECT_HALF_SIZE,
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Left => Color::srgb(0.3, 0.7, 1.0),
            Self::Right => Color::srgb(1.0, 0.45, 0.25),
        }
    }
}

/// Marks one of the bodies that can be pushed, grabbed, and thrown in station E.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistanceJointDemoBody {
    pub side: DistanceJointBody,
}

/// Marks the Avian joint that constrains the distance between the two endpoints.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DistanceJointDemo;

/// A headless and UI-friendly command for changing the configured distance.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistanceJointStationAction {
    pub action: DistanceJointAction,
}

/// The controls exposed by the distance-joint station.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistanceJointAction {
    Increase,
    Decrease,
}

/// Owns the two-body distance-joint demonstration in station E.
pub struct DistanceJointStationPlugin;

impl Plugin for DistanceJointStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_message::<DistanceJointStationAction>()
            .add_systems(
                Startup,
                (spawn_distance_joint_demo, spawn_distance_joint_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_distance_joint_station_actions,
                    update_distance_joint_button_colors,
                    reset_distance_joint_station,
                    update_distance_joint_distance_text,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    apply_distance_joint_station_actions,
                    push_distance_joint_demo,
                )
                    .after(PlayerMovementSet),
            )
            .add_systems(
                Update,
                draw_distance_joint.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_distance_joint_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(OBJECT_SIZE))));

    let mut bodies = [Entity::PLACEHOLDER; 2];
    for (index, side) in DistanceJointBody::ALL.into_iter().enumerate() {
        let material = materials
            .as_mut()
            .map(|materials| materials.add(side.color()));
        let transform = side.initial_transform();
        let mut body = commands.spawn((
            DistanceJointDemoBody { side },
            StationObject::new(STATION_CODE, transform),
            RigidBody::Dynamic,
            Collider::cuboid(OBJECT_SIZE, OBJECT_SIZE, OBJECT_SIZE),
            ColliderDensity(OBJECT_DENSITY),
            Friction::new(0.55),
            Restitution::new(0.05),
            LinearDamping(0.12),
            AngularDamping(0.2),
            transform,
            Name::new(format!("Distance Joint Demo - {}", side.label())),
        ));
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), material) {
            body.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material)));
        }
        bodies[index] = body.id();
    }

    // The anchors are on the facing sides of the bodies. This makes the
    // configured distance visible as the gap between two endpoint markers.
    commands.spawn((
        DistanceJoint::new(bodies[0], bodies[1])
            .with_local_anchor1(DistanceJointBody::Left.local_anchor())
            .with_local_anchor2(DistanceJointBody::Right.local_anchor())
            .with_limits(INITIAL_DISTANCE, INITIAL_DISTANCE),
        JointForces::new(),
        DistanceJointDemo,
        Name::new("Distance Joint Demo Connection"),
    ));
}

#[derive(Component)]
struct DistanceJointStationPanel;

#[derive(Component)]
struct DistanceJointDistanceText;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum DistanceJointStationControl {
    Adjust(DistanceJointAction),
    Reset,
}

fn spawn_distance_joint_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(420.0),
                top: px(220.0),
                width: px(380.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            DistanceJointStationPanel,
            Name::new("Distance Joint Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("E — DISTANCE JOINT"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "The yellow endpoints stay within a configured gap. Push or grab either body to move the pair.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new(format!("Configured distance: {INITIAL_DISTANCE:.2} m")),
                TextFont::from_font_size(14.0),
                TextColor(ENDPOINT_COLOR),
                DistanceJointDistanceText,
            ));
            let mut row = parent.spawn(distance_joint_control_row());
            row.with_children(|row| {
                spawn_distance_joint_control_button(
                    row,
                    "distance -",
                    DistanceJointStationControl::Adjust(DistanceJointAction::Decrease),
                );
                spawn_distance_joint_control_button(
                    row,
                    "distance +",
                    DistanceJointStationControl::Adjust(DistanceJointAction::Increase),
                );
            });
            parent.spawn((
                Text::new("F1 adds Avian joint debug lines."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            spawn_distance_joint_control_button(
                parent,
                "reset station E",
                DistanceJointStationControl::Reset,
            );
        });
}

fn distance_joint_control_row() -> Node {
    Node {
        width: percent(100.0),
        height: px(26.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_distance_joint_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: DistanceJointStationControl,
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
            Name::new(format!("Distance joint control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_distance_joint_station_actions(
    controls: Query<(&Interaction, &DistanceJointStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<DistanceJointStationAction>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *control {
            DistanceJointStationControl::Adjust(action) => {
                actions.write(DistanceJointStationAction { action });
            }
            DistanceJointStationControl::Reset => {
                resets.write(ResetStation { code: STATION_CODE });
            }
        }
    }
}

fn update_distance_joint_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<DistanceJointStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn apply_distance_joint_station_actions(
    mut actions: MessageReader<DistanceJointStationAction>,
    mut joints: Query<&mut DistanceJoint, With<DistanceJointDemo>>,
) {
    let mut adjustment = 0.0;
    for action in actions.read().copied() {
        adjustment += match action.action {
            DistanceJointAction::Increase => DISTANCE_STEP,
            DistanceJointAction::Decrease => -DISTANCE_STEP,
        };
    }
    if adjustment == 0.0 {
        return;
    }

    for mut joint in &mut joints {
        let distance = (joint.limits.min + adjustment).clamp(MIN_DISTANCE, MAX_DISTANCE);
        joint.limits = DistanceLimit::new(distance, distance);
    }
}

fn reset_distance_joint_station(
    mut requests: MessageReader<ResetStation>,
    mut joints: Query<&mut DistanceJoint, With<DistanceJointDemo>>,
) {
    if !requests.read().any(|request| request.code == STATION_CODE) {
        return;
    }
    for mut joint in &mut joints {
        joint.limits = DistanceLimit::new(INITIAL_DISTANCE, INITIAL_DISTANCE);
    }
}

fn update_distance_joint_distance_text(
    joints: Query<&DistanceJoint, With<DistanceJointDemo>>,
    mut labels: Query<&mut Text, With<DistanceJointDistanceText>>,
) {
    let Some(joint) = joints.iter().next() else {
        return;
    };
    for mut label in &mut labels {
        *label = Text::new(format!("Configured distance: {:.2} m", joint.limits.min));
    }
}

fn push_distance_joint_demo(
    settings: Option<Res<PlayerMovementSettings>>,
    camera: Option<Single<&Transform, With<FixedFollowCamera>>>,
    players: Query<(&Transform, &PlayerInput), With<Player>>,
    mut bodies: Query<(&Transform, &mut LinearVelocity), With<DistanceJointDemoBody>>,
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

        for (body_transform, mut velocity) in &mut bodies {
            let offset = body_transform.translation - player_transform.translation;
            let horizontal_offset = Vec2::new(offset.x, offset.z);
            let push_distance = OBJECT_HALF_SIZE + crate::player::PLAYER_RADIUS + 0.08;
            let vertical_overlap = offset.y.abs()
                < crate::player::PLAYER_CAPSULE_LENGTH * 0.5
                    + crate::player::PLAYER_RADIUS
                    + OBJECT_HALF_SIZE;
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

fn draw_distance_joint(
    mut gizmos: Gizmos,
    joints: Query<&DistanceJoint, With<DistanceJointDemo>>,
    bodies: Query<&Transform, With<DistanceJointDemoBody>>,
) {
    let Some(joint) = joints.iter().next() else {
        return;
    };
    let Ok([body1, body2]) = bodies.get_many([joint.body1, joint.body2]) else {
        return;
    };
    let (JointAnchor::Local(local_anchor1), JointAnchor::Local(local_anchor2)) =
        (joint.anchor1, joint.anchor2)
    else {
        return;
    };
    let endpoint1 = body1.translation + body1.rotation * local_anchor1;
    let endpoint2 = body2.translation + body2.rotation * local_anchor2;
    gizmos.line(endpoint1, endpoint2, DISTANCE_LINE_COLOR);
    gizmos.sphere(endpoint1, 0.12, ENDPOINT_COLOR);
    gizmos.sphere(endpoint2, 0.12, ENDPOINT_COLOR);
    gizmos.text(
        Isometry3d::new(
            (endpoint1 + endpoint2) * 0.5 + Vec3::Y * 0.2,
            Quat::IDENTITY,
        ),
        &format!("{:.2} m", joint.limits.min),
        0.2,
        Vec2::ZERO,
        ENDPOINT_COLOR,
    );
    gizmos.text(
        Isometry3d::new(
            body1.translation + Vec3::Y * (OBJECT_HALF_SIZE + 0.2),
            Quat::IDENTITY,
        ),
        DistanceJointBody::Left.label(),
        0.22,
        Vec2::ZERO,
        DistanceJointBody::Left.color(),
    );
    gizmos.text(
        Isometry3d::new(
            body2.translation + Vec3::Y * (OBJECT_HALF_SIZE + 0.2),
            Quat::IDENTITY,
        ),
        DistanceJointBody::Right.label(),
        0.22,
        Vec2::ZERO,
        DistanceJointBody::Right.color(),
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn distance_joint_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            DistanceJointStationPlugin,
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

    fn body(app: &mut App, side: DistanceJointBody) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query::<(Entity, &DistanceJointDemoBody)>();
        bodies
            .iter(world)
            .find_map(|(entity, body)| (body.side == side).then_some(entity))
            .expect("distance-joint body")
    }

    fn joint(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<DistanceJointDemo>>();
        joints.iter(world).next().expect("distance-joint entity")
    }

    fn endpoint_positions(app: &mut App) -> (Vec3, Vec3) {
        let joint_entity = joint(app);
        let joint = app.world().entity(joint_entity);
        let joint = joint.get::<DistanceJoint>().expect("distance joint");
        let [body1, body2] = [joint.body1, joint.body2];
        let position1 = app.world().entity(body1).get::<Position>().unwrap().0;
        let position2 = app.world().entity(body2).get::<Position>().unwrap().0;
        let rotation1 = app.world().entity(body1).get::<Rotation>().unwrap().0;
        let rotation2 = app.world().entity(body2).get::<Rotation>().unwrap().0;
        let anchor1 = match joint.anchor1 {
            JointAnchor::Local(anchor) => anchor,
            _ => panic!("expected local anchor 1"),
        };
        let anchor2 = match joint.anchor2 {
            JointAnchor::Local(anchor) => anchor,
            _ => panic!("expected local anchor 2"),
        };
        (
            position1 + rotation1 * anchor1,
            position2 + rotation2 * anchor2,
        )
    }

    #[test]
    fn station_spawns_two_visible_endpoint_bodies_and_a_configured_distance_joint() {
        let mut app = distance_joint_app();
        app.update();

        let left = body(&mut app, DistanceJointBody::Left);
        let right = body(&mut app, DistanceJointBody::Right);
        let joint_entity = joint(&mut app);
        let entity = app.world().entity(joint_entity);
        let distance_joint = entity.get::<DistanceJoint>().unwrap();
        assert_eq!([distance_joint.body1, distance_joint.body2], [left, right]);
        assert_eq!(
            distance_joint.limits,
            DistanceLimit::new(INITIAL_DISTANCE, INITIAL_DISTANCE)
        );
        assert_eq!(
            distance_joint.local_anchor1(),
            Some(DistanceJointBody::Left.local_anchor())
        );
        assert_eq!(
            distance_joint.local_anchor2(),
            Some(DistanceJointBody::Right.local_anchor())
        );
        assert!(entity.contains::<JointForces>());
        for body in [left, right] {
            let entity = app.world().entity(body);
            assert_eq!(entity.get::<RigidBody>(), Some(&RigidBody::Dynamic));
            assert!(entity.contains::<Collider>());
            assert!(entity.contains::<StationObject>());
            assert!(entity.contains::<Name>());
        }
        let (endpoint1, endpoint2) = endpoint_positions(&mut app);
        assert!((endpoint1.distance(endpoint2) - INITIAL_DISTANCE).abs() < 0.01);
    }

    #[test]
    fn changing_distance_updates_both_limits_and_moves_the_endpoints() {
        let mut app = distance_joint_app();
        app.update();
        let initial_endpoints = endpoint_positions(&mut app);

        app.world_mut()
            .resource_mut::<Messages<DistanceJointStationAction>>()
            .write(DistanceJointStationAction {
                action: DistanceJointAction::Increase,
            });
        run_steps(&mut app, 90);

        let joint_entity = joint(&mut app);
        let distance_joint = app
            .world()
            .entity(joint_entity)
            .get::<DistanceJoint>()
            .unwrap()
            .clone();
        assert_eq!(
            distance_joint.limits,
            DistanceLimit::new(
                INITIAL_DISTANCE + DISTANCE_STEP,
                INITIAL_DISTANCE + DISTANCE_STEP
            )
        );
        let endpoints = endpoint_positions(&mut app);
        assert!(
            (endpoints.0.distance(endpoints.1) - (INITIAL_DISTANCE + DISTANCE_STEP)).abs() < 0.08,
            "distance did not update: {:?}",
            endpoints.0.distance(endpoints.1)
        );
        assert!(endpoints.0.distance(initial_endpoints.0) > 0.05);
        assert!(endpoints.1.distance(initial_endpoints.1) > 0.05);
    }

    #[test]
    fn player_pushes_the_pair_while_the_distance_stays_constrained() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            DistanceJointStationPlugin,
            crate::player::PlayerPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app.update();

        let left = body(&mut app, DistanceJointBody::Left);
        let right = body(&mut app, DistanceJointBody::Right);
        let player = {
            let world = app.world_mut();
            let mut players = world.query_filtered::<Entity, With<Player>>();
            players.iter(world).next().expect("push test player")
        };
        let player_position = Vec3::new(-2.0, 1.0, OBJECT_CENTER.z);
        app.world_mut().entity_mut(player).insert((
            Position(player_position),
            Transform::from_translation(player_position),
        ));
        run_steps(&mut app, 2);
        let initial = endpoint_positions(&mut app);
        let initial_left_position = app.world().entity(left).get::<Position>().unwrap().0;
        let initial_right_position = app.world().entity(right).get::<Position>().unwrap().0;

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);
        run_steps(&mut app, 90);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyD);

        let final_endpoints = endpoint_positions(&mut app);
        let final_left_position = app.world().entity(left).get::<Position>().unwrap().0;
        let final_right_position = app.world().entity(right).get::<Position>().unwrap().0;
        assert!(final_left_position.x > initial_left_position.x + 0.1);
        assert!(final_right_position.x > initial_right_position.x + 0.1);
        assert!(
            (final_endpoints.0.distance(final_endpoints.1) - initial.0.distance(initial.1)).abs()
                < 0.08
        );
    }
}
