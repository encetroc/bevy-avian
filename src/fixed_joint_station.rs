use avian3d::prelude::*;
use bevy::prelude::*;

use crate::camera::FixedFollowCamera;
use crate::player::{Player, PlayerInput, PlayerMovementSet, PlayerMovementSettings};
use crate::stations::{ResetStation, StationObject};

const STATION_CODE: char = 'E';
const OBJECT_CENTER: Vec3 = Vec3::new(0.0, 0.5, -0.9);
const OBJECT_SEPARATION: f32 = 1.5;
const OBJECT_SIZE: f32 = 0.8;
const OBJECT_HALF_SIZE: f32 = OBJECT_SIZE * 0.5;
const OBJECT_DENSITY: f32 = 0.75;
const OUTWARD_FORCE: f32 = 12.0;
const OUTWARD_IMPULSE: f32 = 6.0;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// Identifies one of the two dynamic bodies in the fixed-joint demonstration.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FixedJointBody {
    Left,
    Right,
}

impl FixedJointBody {
    const ALL: [Self; 2] = [Self::Left, Self::Right];

    const fn label(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }

    const fn outward_direction(self) -> Vec3 {
        match self {
            Self::Left => Vec3::NEG_X,
            Self::Right => Vec3::X,
        }
    }

    fn initial_transform(self) -> Transform {
        let x = match self {
            Self::Left => -OBJECT_SEPARATION * 0.5,
            Self::Right => OBJECT_SEPARATION * 0.5,
        };
        Transform::from_translation(OBJECT_CENTER + Vec3::X * x)
    }
}

/// Marks one of the bodies that can be pushed, grabbed, and thrown in station E.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedJointDemoBody {
    pub side: FixedJointBody,
}

/// Marks the Avian joint that connects the two demonstration bodies.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FixedJointDemo;

/// A headless and UI-friendly command for stressing one side of the structure.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedJointStationAction {
    pub body: FixedJointBody,
    pub action: FixedJointAction,
}

/// The two force controls exposed by the fixed-joint station.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedJointAction {
    Force,
    Impulse,
}

/// Owns the two-body fixed-joint demonstration in station E.
pub struct FixedJointStationPlugin;

impl Plugin for FixedJointStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_message::<FixedJointStationAction>()
            .add_systems(
                Startup,
                (spawn_fixed_joint_demo, spawn_fixed_joint_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_fixed_joint_station_actions,
                    update_fixed_joint_button_colors,
                    reset_fixed_joint_station,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (apply_fixed_joint_station_actions, push_fixed_joint_demo).after(PlayerMovementSet),
            )
            .add_systems(
                Update,
                draw_fixed_joint_labels.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_fixed_joint_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(OBJECT_SIZE))));

    let mut bodies = [Entity::PLACEHOLDER; 2];
    for (index, side) in FixedJointBody::ALL.into_iter().enumerate() {
        let material = materials.as_mut().map(|materials| {
            let color = match side {
                FixedJointBody::Left => Color::srgb(0.25, 0.55, 0.95),
                FixedJointBody::Right => Color::srgb(0.95, 0.45, 0.2),
            };
            materials.add(color)
        });
        let transform = side.initial_transform();
        let mut body = commands.spawn((
            FixedJointDemoBody { side },
            StationObject::new(STATION_CODE, transform),
            RigidBody::Dynamic,
            Collider::cuboid(OBJECT_SIZE, OBJECT_SIZE, OBJECT_SIZE),
            ColliderDensity(OBJECT_DENSITY),
            Friction::new(0.55),
            Restitution::new(0.05),
            LinearDamping(0.12),
            AngularDamping(0.2),
            transform,
            Name::new(format!("Fixed Joint Demo - {}", side.label())),
        ));
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), material) {
            body.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material)));
        }
        bodies[index] = body.id();
    }

    // The local anchors meet at the midpoint between the two bodies. Keeping
    // the frames local preserves the initial relative transform after forces,
    // throws, and collisions move the structure through the arena.
    commands.spawn((
        FixedJoint::new(bodies[0], bodies[1])
            .with_local_anchor1(Vec3::X * (OBJECT_SEPARATION * 0.5))
            .with_local_anchor2(Vec3::NEG_X * (OBJECT_SEPARATION * 0.5)),
        JointForces::new(),
        FixedJointDemo,
        Name::new("Fixed Joint Demo Connection"),
    ));
}

#[derive(Component)]
struct FixedJointStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum FixedJointStationControl {
    Apply(FixedJointBody, FixedJointAction),
    Reset,
}

fn spawn_fixed_joint_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(420.0),
                top: px(16.0),
                width: px(380.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            FixedJointStationPanel,
            Name::new("Fixed Joint Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("E — FIXED JOINT"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Two dynamic bodies share translation and rotation. Grab or throw either side; the fixed joint keeps the structure together.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            for side in FixedJointBody::ALL {
                parent.spawn((
                    Text::new(format!("{} body", side.label())),
                    TextFont::from_font_size(13.0),
                    TextColor(match side {
                        FixedJointBody::Left => Color::srgb(0.25, 0.55, 0.95),
                        FixedJointBody::Right => Color::srgb(0.95, 0.45, 0.2),
                    }),
                ));
                let mut row = parent.spawn(fixed_joint_control_row());
                row.with_children(|row| {
                    spawn_fixed_joint_control_button(
                        row,
                        "force outward",
                        FixedJointStationControl::Apply(side, FixedJointAction::Force),
                    );
                    spawn_fixed_joint_control_button(
                        row,
                        "impulse outward",
                        FixedJointStationControl::Apply(side, FixedJointAction::Impulse),
                    );
                });
            }
            parent.spawn((
                Text::new("F1 enables Avian collider and fixed-joint debug lines."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            spawn_fixed_joint_control_button(
                parent,
                "reset station E",
                FixedJointStationControl::Reset,
            );
        });
}

fn fixed_joint_control_row() -> Node {
    Node {
        width: percent(100.0),
        height: px(26.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_fixed_joint_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: FixedJointStationControl,
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
            Name::new(format!("Fixed joint control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_fixed_joint_station_actions(
    controls: Query<(&Interaction, &FixedJointStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<FixedJointStationAction>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *control {
            FixedJointStationControl::Apply(body, action) => {
                actions.write(FixedJointStationAction { body, action });
            }
            FixedJointStationControl::Reset => {
                resets.write(ResetStation { code: STATION_CODE });
            }
        }
    }
}

fn update_fixed_joint_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<FixedJointStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn apply_fixed_joint_station_actions(
    mut actions: MessageReader<FixedJointStationAction>,
    mut bodies: Query<(Entity, &FixedJointDemoBody, Forces)>,
    mut commands: Commands,
) {
    for action in actions.read().copied() {
        for (entity, body, mut forces) in &mut bodies {
            if body.side != action.body {
                continue;
            }
            let direction = body.side.outward_direction();
            match action.action {
                FixedJointAction::Force => {
                    commands
                        .entity(entity)
                        .insert(ConstantForce(direction * OUTWARD_FORCE));
                }
                FixedJointAction::Impulse => {
                    forces.apply_linear_impulse(direction * OUTWARD_IMPULSE);
                }
            }
        }
    }
}

fn reset_fixed_joint_station(
    mut requests: MessageReader<ResetStation>,
    mut commands: Commands,
    bodies: Query<(Entity, &FixedJointDemoBody)>,
) {
    if !requests.read().any(|request| request.code == STATION_CODE) {
        return;
    }
    for (entity, _) in &bodies {
        commands.entity(entity).remove::<ConstantForce>();
    }
}

fn push_fixed_joint_demo(
    settings: Option<Res<PlayerMovementSettings>>,
    camera: Option<Single<&Transform, With<FixedFollowCamera>>>,
    players: Query<(&Transform, &PlayerInput), With<Player>>,
    mut bodies: Query<(&Transform, &mut LinearVelocity), With<FixedJointDemoBody>>,
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

fn draw_fixed_joint_labels(mut gizmos: Gizmos, bodies: Query<(&FixedJointDemoBody, &Transform)>) {
    for (body, transform) in &bodies {
        gizmos.text(
            Isometry3d::new(
                transform.translation + Vec3::Y * (OBJECT_HALF_SIZE + 0.2),
                Quat::IDENTITY,
            ),
            body.side.label(),
            0.22,
            Vec2::ZERO,
            match body.side {
                FixedJointBody::Left => Color::srgb(0.25, 0.55, 0.95),
                FixedJointBody::Right => Color::srgb(0.95, 0.45, 0.2),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn fixed_joint_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            FixedJointStationPlugin,
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

    fn body(app: &mut App, side: FixedJointBody) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query::<(Entity, &FixedJointDemoBody)>();
        bodies
            .iter(world)
            .find_map(|(entity, body)| (body.side == side).then_some(entity))
            .expect("fixed-joint body")
    }

    fn joint(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<FixedJointDemo>>();
        joints.iter(world).next().expect("fixed-joint entity")
    }

    fn relative_pose(app: &App, left: Entity, right: Entity) -> (Vec3, Quat) {
        let left_position = app.world().entity(left).get::<Position>().unwrap().0;
        let right_position = app.world().entity(right).get::<Position>().unwrap().0;
        let left_rotation = app.world().entity(left).get::<Rotation>().unwrap().0;
        let right_rotation = app.world().entity(right).get::<Rotation>().unwrap().0;
        (
            left_rotation.inverse() * (right_position - left_position),
            left_rotation.inverse() * right_rotation,
        )
    }

    #[test]
    fn station_spawns_two_dynamic_bodies_and_one_force_reporting_fixed_joint() {
        let mut app = fixed_joint_app();
        app.update();

        let left = body(&mut app, FixedJointBody::Left);
        let right = body(&mut app, FixedJointBody::Right);
        let joint_entity = joint(&mut app);
        let entity = app.world().entity(joint_entity);
        let fixed_joint = entity.get::<FixedJoint>().unwrap();
        assert_eq!([fixed_joint.body1, fixed_joint.body2], [left, right]);
        assert!(entity.contains::<JointForces>());
        for body in [left, right] {
            let entity = app.world().entity(body);
            assert_eq!(entity.get::<RigidBody>(), Some(&RigidBody::Dynamic));
            assert!(entity.contains::<Collider>());
            assert!(entity.contains::<StationObject>());
            assert!(entity.contains::<Name>());
        }
    }

    #[test]
    fn outward_forces_on_each_body_preserve_their_relative_transform() {
        let mut app = fixed_joint_app();
        app.update();
        let left = body(&mut app, FixedJointBody::Left);
        let right = body(&mut app, FixedJointBody::Right);
        let initial = relative_pose(&app, left, right);

        for body in FixedJointBody::ALL {
            app.world_mut()
                .resource_mut::<Messages<FixedJointStationAction>>()
                .write(FixedJointStationAction {
                    body,
                    action: FixedJointAction::Force,
                });
            run_steps(&mut app, 90);
        }

        let final_pose = relative_pose(&app, left, right);
        assert!(
            final_pose.0.distance(initial.0) < 0.08,
            "fixed joint allowed translation drift: {:?} -> {:?}",
            initial.0,
            final_pose.0
        );
        assert!(
            final_pose.1.angle_between(initial.1) < 0.08,
            "fixed joint allowed rotation drift: {:?} -> {:?}",
            initial.1,
            final_pose.1
        );
        let joint_entity = joint(&mut app);
        assert!(app.world().entity(joint_entity).contains::<JointForces>());
    }

    #[test]
    fn thrown_structure_survives_an_ordinary_collision_without_breaking_the_joint() {
        let mut app = fixed_joint_app();
        app.add_systems(Startup, spawn_collision_course);
        app.update();
        let left = body(&mut app, FixedJointBody::Left);
        let right = body(&mut app, FixedJointBody::Right);
        let initial = relative_pose(&app, left, right);

        for body in [left, right] {
            app.world_mut()
                .entity_mut(body)
                .insert(LinearVelocity(Vec3::new(15.0, 0.0, 0.0)));
        }
        run_steps(&mut app, 90);

        let final_pose = relative_pose(&app, left, right);
        assert!(final_pose.0.distance(initial.0) < 0.12);
        assert!(final_pose.1.angle_between(initial.1) < 0.12);
        let joint_entity = joint(&mut app);
        assert!(app.world().entity(joint_entity).contains::<FixedJoint>());
    }

    fn spawn_collision_course(mut commands: Commands) {
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 0.5, 20.0),
            Transform::from_xyz(0.0, -0.25, -0.9),
            Name::new("Fixed Joint Test Floor"),
        ));
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(0.4, 3.0, 4.0),
            Transform::from_xyz(2.0, 1.5, -0.9),
            Name::new("Fixed Joint Test Wall"),
        ));
    }

    #[test]
    fn player_push_system_moves_the_connected_pair_together() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::input::InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            FixedJointStationPlugin,
            crate::player::PlayerPlugin,
            StationLayoutPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )))
        .add_systems(Startup, spawn_push_test_floor);
        app.finish();
        app.update();

        let left = body(&mut app, FixedJointBody::Left);
        let right = body(&mut app, FixedJointBody::Right);
        let player = {
            let world = app.world_mut();
            let mut players = world.query_filtered::<Entity, With<Player>>();
            players.iter(world).next().expect("push test player")
        };
        let player_position = Vec3::new(-2.0, 1.0, -0.9);
        app.world_mut().entity_mut(player).insert((
            Position(player_position),
            Transform::from_translation(player_position),
        ));
        run_steps(&mut app, 2);
        let initial = relative_pose(&app, left, right);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);
        run_steps(&mut app, 90);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyD);

        let final_pose = relative_pose(&app, left, right);
        let left_delta = app.world().entity(left).get::<Position>().unwrap().0.x
            - (OBJECT_CENTER.x - OBJECT_SEPARATION * 0.5);
        let right_delta = app.world().entity(right).get::<Position>().unwrap().0.x
            - (OBJECT_CENTER.x + OBJECT_SEPARATION * 0.5);
        assert!(left_delta > 0.1 || right_delta > 0.1);
        assert!(final_pose.0.distance(initial.0) < 0.12);
    }

    fn spawn_push_test_floor(mut commands: Commands) {
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 0.5, 20.0),
            Transform::from_xyz(0.0, -0.25, -0.9),
        ));
    }
}
