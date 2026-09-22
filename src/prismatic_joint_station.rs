use avian3d::prelude::*;
use bevy::prelude::*;

use crate::camera::FixedFollowCamera;
use crate::player::{Player, PlayerInput, PlayerMovementSet, PlayerMovementSettings};
use crate::stations::{ResetStation, StationObject};

const STATION_CODE: char = 'E';
const GUIDE_CENTER: Vec3 = Vec3::new(-1.6, 0.8, -0.1);
const GUIDE_SIZE: Vec3 = Vec3::new(0.8, 1.2, 1.2);
const SLIDER_CENTER: Vec3 = Vec3::new(-0.4, 0.8, -0.1);
const SLIDER_SIZE: f32 = 0.8;
const GUIDE_ANCHOR: Vec3 = Vec3::new(GUIDE_SIZE.x * 0.5, 0.0, 0.0);
const SLIDER_AXIS: Vec3 = Vec3::X;
const MIN_SLIDER_DISTANCE: f32 = 0.5;
const MAX_SLIDER_DISTANCE: f32 = 1.8;
const OBJECT_DENSITY: f32 = 0.75;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const SLIDER_AXIS_COLOR: Color = Color::srgb(0.25, 1.0, 0.65);
const SLIDER_LIMIT_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);

/// Identifies the static guide in the prismatic-joint demonstration.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrismaticJointGuide;

/// Identifies the dynamic block that slides through the guide.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrismaticJointSlider;

/// Marks the Avian joint that constrains the slider to one axis.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrismaticJointDemo;

/// A headless and UI-friendly command for resetting the sliding-block demo.
#[derive(Message, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrismaticJointStationAction {
    pub action: PrismaticJointAction,
}

/// The controls exposed by the prismatic-joint station.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PrismaticJointAction {
    #[default]
    Reset,
}

/// Owns the guide-and-slider prismatic-joint demonstration in station E.
pub struct PrismaticJointStationPlugin;

impl Plugin for PrismaticJointStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_message::<PrismaticJointStationAction>()
            .add_systems(
                Startup,
                (spawn_prismatic_joint_demo, spawn_prismatic_joint_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_prismatic_joint_station_actions,
                    update_prismatic_joint_button_colors,
                    reset_prismatic_joint_station,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                push_prismatic_joint_demo.after(PlayerMovementSet),
            )
            .add_systems(
                Update,
                draw_prismatic_joint.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_prismatic_joint_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let guide_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(GUIDE_SIZE)));
    let slider_mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(SLIDER_SIZE))));
    let guide_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.24, 0.65, 0.78)));
    let slider_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.92, 0.38, 0.18)));

    let guide = commands
        .spawn((
            PrismaticJointGuide,
            RigidBody::Static,
            Collider::cuboid(GUIDE_SIZE.x, GUIDE_SIZE.y, GUIDE_SIZE.z),
            Transform::from_translation(GUIDE_CENTER),
            Name::new("Prismatic Joint Guide"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (guide_mesh, guide_material) {
        commands
            .entity(guide)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    let slider_transform = Transform::from_translation(SLIDER_CENTER);
    let slider = commands
        .spawn((
            PrismaticJointSlider,
            StationObject::new(STATION_CODE, slider_transform),
            RigidBody::Dynamic,
            Collider::cuboid(SLIDER_SIZE, SLIDER_SIZE, SLIDER_SIZE),
            ColliderDensity(OBJECT_DENSITY),
            Friction::new(0.55),
            Restitution::new(0.05),
            LinearDamping(0.12),
            AngularDamping(0.2),
            slider_transform,
            Name::new("Prismatic Joint Slider"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (slider_mesh, slider_material) {
        commands
            .entity(slider)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    // The guide's right face is the reference anchor. The slider starts 0.8 m
    // along +X from it and can travel between the configured limits. Disabling
    // guide/slider collisions lets the joint limit, rather than collider
    // overlap at the end stops, define the full visible travel.
    commands.spawn((
        PrismaticJoint::new(guide, slider)
            .with_local_anchor1(GUIDE_ANCHOR)
            .with_local_anchor2(Vec3::ZERO)
            .with_slider_axis(SLIDER_AXIS)
            .with_limits(MIN_SLIDER_DISTANCE, MAX_SLIDER_DISTANCE),
        JointForces::new(),
        JointCollisionDisabled,
        PrismaticJointDemo,
        Name::new("Prismatic Joint Slider Constraint"),
    ));
}

#[derive(Component)]
struct PrismaticJointStationPanel;

#[derive(Component)]
struct PrismaticJointStationControl(PrismaticJointAction);

fn spawn_prismatic_joint_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(420.0),
                top: px(660.0),
                width: px(380.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            PrismaticJointStationPanel,
            Name::new("Prismatic Joint Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("E — PRISMATIC JOINT"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Push, grab, or throw the block. The guide permits translation along X while preventing sideways motion and rotation.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new(format!(
                    "X travel limits: {MIN_SLIDER_DISTANCE:.2} m to {MAX_SLIDER_DISTANCE:.2} m"
                )),
                TextFont::from_font_size(12.0),
                TextColor(SLIDER_AXIS_COLOR),
            ));
            parent.spawn((
                Text::new("Green: permitted X axis   F1: Avian joint debug lines"),
                TextFont::from_font_size(12.0),
                TextColor(SLIDER_LIMIT_COLOR),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            spawn_prismatic_joint_control_button(
                parent,
                "reset station E",
                PrismaticJointAction::Reset,
            );
        });
}

fn spawn_prismatic_joint_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    action: PrismaticJointAction,
) {
    parent
        .spawn((
            Button,
            PrismaticJointStationControl(action),
            Node {
                width: px(132.0),
                height: px(22.0),
                margin: UiRect::right(px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Prismatic joint control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_prismatic_joint_station_actions(
    controls: Query<(&Interaction, &PrismaticJointStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<PrismaticJointStationAction>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            actions.write(PrismaticJointStationAction { action: control.0 });
        }
    }
}

fn update_prismatic_joint_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<PrismaticJointStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn reset_prismatic_joint_station(
    mut actions: MessageReader<PrismaticJointStationAction>,
    mut resets: MessageWriter<ResetStation>,
) {
    if actions
        .read()
        .any(|action| action.action == PrismaticJointAction::Reset)
    {
        resets.write(ResetStation { code: STATION_CODE });
    }
}

fn push_prismatic_joint_demo(
    settings: Option<Res<PlayerMovementSettings>>,
    camera: Option<Single<&Transform, With<FixedFollowCamera>>>,
    players: Query<(&Transform, &PlayerInput), With<Player>>,
    mut sliders: Query<(&Transform, &mut LinearVelocity), With<PrismaticJointSlider>>,
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

        for (slider_transform, mut velocity) in &mut sliders {
            let offset = slider_transform.translation - player_transform.translation;
            let horizontal_offset = Vec2::new(offset.x, offset.z);
            let push_distance = SLIDER_SIZE * 0.5 + crate::player::PLAYER_RADIUS + 0.08;
            let vertical_overlap = offset.y.abs()
                < crate::player::PLAYER_CAPSULE_LENGTH * 0.5
                    + crate::player::PLAYER_RADIUS
                    + SLIDER_SIZE * 0.5;
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

fn draw_prismatic_joint(
    mut gizmos: Gizmos,
    joints: Query<&PrismaticJoint, With<PrismaticJointDemo>>,
    bodies: Query<&Transform>,
) {
    let Some(joint) = joints.iter().next() else {
        return;
    };
    let Ok([guide, slider]) = bodies.get_many([joint.body1, joint.body2]) else {
        return;
    };
    let (Some(anchor1), Some(anchor2), Some(axis)) = (
        joint.local_anchor1(),
        joint.local_anchor2(),
        joint.local_slider_axis1(),
    ) else {
        return;
    };
    let anchor1 = guide.translation + guide.rotation * anchor1;
    let anchor2 = slider.translation + slider.rotation * anchor2;
    let axis = (guide.rotation * axis).normalize_or_zero();
    if axis == Vec3::ZERO {
        return;
    }

    let limits = joint.limits.unwrap_or(DistanceLimit::new(-2.0, 2.0));
    let min_point = anchor1 + axis * limits.min;
    let max_point = anchor1 + axis * limits.max;
    gizmos.line(anchor1, anchor2, SLIDER_LIMIT_COLOR);
    gizmos.line(min_point, max_point, SLIDER_AXIS_COLOR);
    gizmos.sphere(min_point, 0.12, SLIDER_LIMIT_COLOR);
    gizmos.sphere(max_point, 0.12, SLIDER_LIMIT_COLOR);
    gizmos.sphere(anchor2, 0.13, SLIDER_AXIS_COLOR);
    gizmos.text(
        Isometry3d::new(max_point + Vec3::Y * 0.2, Quat::IDENTITY),
        "X SLIDER AXIS",
        0.2,
        Vec2::ZERO,
        SLIDER_AXIS_COLOR,
    );
    gizmos.text(
        Isometry3d::new(
            slider.translation + Vec3::Y * (SLIDER_SIZE * 0.5 + 0.2),
            Quat::IDENTITY,
        ),
        "SLIDER",
        0.22,
        Vec2::ZERO,
        SLIDER_AXIS_COLOR,
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn prismatic_joint_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            PrismaticJointStationPlugin,
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

    fn guide(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut guides = world.query_filtered::<Entity, With<PrismaticJointGuide>>();
        guides.iter(world).next().expect("prismatic-joint guide")
    }

    fn slider(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut sliders = world.query_filtered::<Entity, With<PrismaticJointSlider>>();
        sliders.iter(world).next().expect("prismatic-joint slider")
    }

    fn joint(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<PrismaticJointDemo>>();
        joints.iter(world).next().expect("prismatic-joint entity")
    }

    fn slider_distance(app: &mut App) -> f32 {
        let joint_entity = joint(app);
        let joint = app.world().entity(joint_entity);
        let joint = joint.get::<PrismaticJoint>().expect("prismatic joint");
        let guide = app.world().entity(joint.body1);
        let slider = app.world().entity(joint.body2);
        let guide_transform = guide.get::<Transform>().expect("guide transform");
        let slider_transform = slider.get::<Transform>().expect("slider transform");
        let anchor1 = guide_transform.translation
            + guide_transform.rotation * joint.local_anchor1().expect("guide anchor");
        let anchor2 = slider_transform.translation
            + slider_transform.rotation * joint.local_anchor2().expect("slider anchor");
        let axis = guide_transform.rotation * joint.local_slider_axis1().expect("slider axis");
        (anchor2 - anchor1).dot(axis.normalize_or_zero())
    }

    #[test]
    fn station_spawns_static_guide_dynamic_slider_and_configured_prismatic_joint() {
        let mut app = prismatic_joint_app();
        app.update();

        let guide = guide(&mut app);
        let slider = slider(&mut app);
        let joint_entity = joint(&mut app);
        let entity = app.world().entity(joint_entity);
        let prismatic_joint = entity.get::<PrismaticJoint>().unwrap();
        assert_eq!(
            [prismatic_joint.body1, prismatic_joint.body2],
            [guide, slider]
        );
        assert_eq!(prismatic_joint.slider_axis, SLIDER_AXIS);
        assert_eq!(
            prismatic_joint.limits,
            Some(DistanceLimit::new(MIN_SLIDER_DISTANCE, MAX_SLIDER_DISTANCE))
        );
        assert_eq!(prismatic_joint.local_anchor1(), Some(GUIDE_ANCHOR));
        assert_eq!(prismatic_joint.local_anchor2(), Some(Vec3::ZERO));
        assert!(entity.contains::<JointForces>());
        assert!(entity.contains::<JointCollisionDisabled>());
        assert_eq!(
            app.world().entity(guide).get::<RigidBody>(),
            Some(&RigidBody::Static)
        );
        assert_eq!(
            app.world().entity(slider).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
        assert!(app.world().entity(slider).contains::<StationObject>());
        assert!((slider_distance(&mut app) - 0.8).abs() < 0.01);
    }

    #[test]
    fn velocity_along_the_slider_axis_moves_the_block_within_limits() {
        let mut app = prismatic_joint_app();
        app.update();
        let slider = slider(&mut app);
        let initial_position = app.world().entity(slider).get::<Position>().unwrap().0;
        app.world_mut()
            .entity_mut(slider)
            .insert(LinearVelocity(Vec3::X * 12.0));

        run_steps(&mut app, 120);

        let final_position = app.world().entity(slider).get::<Position>().unwrap().0;
        assert!(final_position.x > initial_position.x + 0.1);
        assert!((final_position.y - initial_position.y).abs() < 0.08);
        assert!((final_position.z - initial_position.z).abs() < 0.08);
        assert!(
            (MIN_SLIDER_DISTANCE - 0.08..=MAX_SLIDER_DISTANCE + 0.08)
                .contains(&slider_distance(&mut app)),
            "slider escaped its travel limits: {}",
            slider_distance(&mut app)
        );
    }

    #[test]
    fn off_axis_linear_and_angular_impacts_do_not_tilt_or_displace_the_block() {
        let mut app = prismatic_joint_app();
        app.update();
        let slider = slider(&mut app);
        app.world_mut().entity_mut(slider).insert((
            LinearVelocity(Vec3::new(0.0, 10.0, 12.0)),
            AngularVelocity(Vec3::new(5.0, 4.0, 3.0)),
        ));

        run_steps(&mut app, 120);

        let entity = app.world().entity(slider);
        let position = entity.get::<Position>().unwrap().0;
        let rotation = entity.get::<Rotation>().unwrap().0;
        assert!((position.y - SLIDER_CENTER.y).abs() < 0.1);
        assert!((position.z - SLIDER_CENTER.z).abs() < 0.1);
        assert!(
            rotation.angle_between(Quat::IDENTITY) < 0.12,
            "prismatic joint allowed rotation: {rotation:?}"
        );
    }

    #[test]
    fn cross_axis_collision_keeps_the_slider_on_its_axis() {
        let mut app = prismatic_joint_app();
        app.add_systems(Startup, spawn_cross_axis_impact_wall);
        app.update();
        let slider = slider(&mut app);
        app.world_mut()
            .entity_mut(slider)
            .insert(LinearVelocity(Vec3::new(0.0, 0.0, 18.0)));

        run_steps(&mut app, 120);

        let entity = app.world().entity(slider);
        let position = entity.get::<Position>().unwrap().0;
        assert!((position.y - SLIDER_CENTER.y).abs() < 0.1);
        assert!((position.z - SLIDER_CENTER.z).abs() < 0.1);
        assert!(
            (MIN_SLIDER_DISTANCE - 0.08..=MAX_SLIDER_DISTANCE + 0.08)
                .contains(&slider_distance(&mut app)),
            "cross-axis impact broke the prismatic constraint"
        );
    }

    fn spawn_cross_axis_impact_wall(mut commands: Commands) {
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(3.0, 3.0, 0.3),
            Transform::from_xyz(SLIDER_CENTER.x, SLIDER_CENTER.y, 0.7),
            Name::new("Prismatic Joint Cross Axis Impact Wall"),
        ));
    }
}
