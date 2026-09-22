use avian3d::prelude::*;
use bevy::prelude::*;

use crate::cursor_hover::{HoverState, update_hover_reachability};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// The phase of the J-key runtime joint creation workflow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JointCreationPhase {
    #[default]
    Inactive,
    AwaitingFirst,
    AwaitingSecond {
        first: Entity,
    },
    ChoosingType {
        first: Entity,
        second: Entity,
    },
}

/// Runtime state for selecting two bodies and creating a joint between them.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JointCreationState {
    pub phase: JointCreationPhase,
}

/// The five joint types supported by the runtime creation tool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeJointType {
    Fixed,
    Distance,
    Revolute,
    Prismatic,
    Spherical,
}

impl RuntimeJointType {
    const ALL: [Self; 5] = [
        Self::Fixed,
        Self::Distance,
        Self::Revolute,
        Self::Prismatic,
        Self::Spherical,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Distance => "Distance",
            Self::Revolute => "Revolute",
            Self::Prismatic => "Prismatic",
            Self::Spherical => "Spherical",
        }
    }
}

/// Headless-friendly commands for the runtime joint creation workflow.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum JointCreationAction {
    Choose(RuntimeJointType),
    Cancel,
}

/// Marks joints made by the runtime creation tool.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeCreatedJoint;

/// Owns the J-key runtime joint creation workflow and its small instruction panel.
pub struct JointCreationPlugin;

impl Plugin for JointCreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JointCreationState>()
            .add_message::<JointCreationAction>()
            .add_systems(Startup, spawn_joint_creation_ui)
            .add_systems(
                Update,
                (
                    handle_joint_creation_keyboard,
                    select_joint_creation_body.after(update_hover_reachability),
                    queue_joint_creation_actions,
                    apply_joint_creation_actions,
                    update_joint_creation_ui,
                )
                    .chain(),
            );
    }
}

fn handle_joint_creation_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<JointCreationState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        state.phase = JointCreationPhase::Inactive;
    } else if keyboard.just_pressed(KeyCode::KeyJ) {
        state.phase = match state.phase {
            JointCreationPhase::Inactive => JointCreationPhase::AwaitingFirst,
            _ => JointCreationPhase::Inactive,
        };
    }
}

fn select_joint_creation_body(
    mouse: Res<ButtonInput<MouseButton>>,
    hover: Res<HoverState>,
    controls: Query<&Interaction, With<JointCreationControl>>,
    bodies: Query<(), (With<Collider>, With<RigidBody>)>,
    mut state: ResMut<JointCreationState>,
) {
    if !mouse.just_pressed(MouseButton::Left)
        || controls
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    let Some(entity) = hover
        .object
        .as_ref()
        .map(|object| object.entity)
        .filter(|entity| bodies.contains(*entity))
    else {
        return;
    };

    state.phase = match state.phase {
        JointCreationPhase::AwaitingFirst => JointCreationPhase::AwaitingSecond { first: entity },
        JointCreationPhase::AwaitingSecond { first } if first != entity => {
            JointCreationPhase::ChoosingType {
                first,
                second: entity,
            }
        }
        phase => phase,
    };
}

fn queue_joint_creation_actions(
    controls: Query<(&Interaction, &JointCreationControl), Changed<Interaction>>,
    mut actions: MessageWriter<JointCreationAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match *control {
            JointCreationControl::Choose(joint_type) => JointCreationAction::Choose(joint_type),
            JointCreationControl::Cancel => JointCreationAction::Cancel,
        });
    }
}

#[allow(clippy::type_complexity)]
fn apply_joint_creation_actions(
    mut commands: Commands,
    mut actions: MessageReader<JointCreationAction>,
    mut state: ResMut<JointCreationState>,
    bodies: Query<&Transform, (With<Collider>, With<RigidBody>)>,
) {
    for action in actions.read() {
        match *action {
            JointCreationAction::Cancel => state.phase = JointCreationPhase::Inactive,
            JointCreationAction::Choose(joint_type) => {
                let JointCreationPhase::ChoosingType { first, second } = state.phase else {
                    continue;
                };
                if first == second {
                    state.phase = JointCreationPhase::AwaitingFirst;
                    continue;
                }
                let Ok([transform1, transform2]) = bodies.get_many([first, second]) else {
                    state.phase = JointCreationPhase::Inactive;
                    continue;
                };
                let anchor = (transform1.translation + transform2.translation) * 0.5;
                let local_anchor1 =
                    transform1.rotation.inverse() * (anchor - transform1.translation);
                let local_anchor2 =
                    transform2.rotation.inverse() * (anchor - transform2.translation);
                spawn_runtime_joint(
                    &mut commands,
                    joint_type,
                    first,
                    second,
                    local_anchor1,
                    local_anchor2,
                    transform1.translation.distance(transform2.translation),
                );
                state.phase = JointCreationPhase::Inactive;
            }
        }
    }
}

fn spawn_runtime_joint(
    commands: &mut Commands,
    joint_type: RuntimeJointType,
    body1: Entity,
    body2: Entity,
    local_anchor1: Vec3,
    local_anchor2: Vec3,
    body_distance: f32,
) {
    let name = format!("Runtime {} Joint", joint_type.label());
    match joint_type {
        RuntimeJointType::Fixed => {
            commands.spawn((
                FixedJoint::new(body1, body2)
                    .with_local_anchor1(local_anchor1)
                    .with_local_anchor2(local_anchor2),
                JointForces::new(),
                JointCollisionDisabled,
                RuntimeCreatedJoint,
                Name::new(name),
            ));
        }
        RuntimeJointType::Distance => {
            commands.spawn((
                DistanceJoint::new(body1, body2)
                    .with_local_anchor1(Vec3::ZERO)
                    .with_local_anchor2(Vec3::ZERO)
                    .with_limits(body_distance, body_distance),
                JointForces::new(),
                JointCollisionDisabled,
                RuntimeCreatedJoint,
                Name::new(name),
            ));
        }
        RuntimeJointType::Revolute => {
            commands.spawn((
                RevoluteJoint::new(body1, body2)
                    .with_local_anchor1(local_anchor1)
                    .with_local_anchor2(local_anchor2)
                    .with_hinge_axis(Vec3::Y),
                JointForces::new(),
                JointCollisionDisabled,
                RuntimeCreatedJoint,
                Name::new(name),
            ));
        }
        RuntimeJointType::Prismatic => {
            commands.spawn((
                PrismaticJoint::new(body1, body2)
                    .with_local_anchor1(local_anchor1)
                    .with_local_anchor2(local_anchor2)
                    .with_slider_axis(Vec3::X),
                JointForces::new(),
                JointCollisionDisabled,
                RuntimeCreatedJoint,
                Name::new(name),
            ));
        }
        RuntimeJointType::Spherical => {
            commands.spawn((
                SphericalJoint::new(body1, body2)
                    .with_local_anchor1(local_anchor1)
                    .with_local_anchor2(local_anchor2),
                JointForces::new(),
                JointCollisionDisabled,
                RuntimeCreatedJoint,
                Name::new(name),
            ));
        }
    }
}

#[derive(Component)]
struct JointCreationPanel;

#[derive(Component)]
struct JointCreationStatusText;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum JointCreationControl {
    Choose(RuntimeJointType),
    Cancel,
}

fn spawn_joint_creation_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(16.0),
                width: px(360.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            JointCreationPanel,
            Name::new("Runtime Joint Creation Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("J — CREATE JOINT"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Press J, then click two physics bodies. Escape cancels."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Press J to begin."),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                JointCreationStatusText,
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            parent
                .spawn(Node {
                    width: percent(100.0),
                    height: px(26.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for joint_type in RuntimeJointType::ALL {
                        spawn_joint_creation_button(
                            row,
                            joint_type.label(),
                            JointCreationControl::Choose(joint_type),
                        );
                    }
                });
            spawn_joint_creation_button(parent, "cancel", JointCreationControl::Cancel);
        });
}

fn spawn_joint_creation_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: JointCreationControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(62.0),
                height: px(22.0),
                margin: UiRect::right(px(3.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Joint creation control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(10.0),
            TextColor(PANEL_TEXT),
        ));
}

fn update_joint_creation_ui(
    state: Res<JointCreationState>,
    mut status: Query<&mut Text, With<JointCreationStatusText>>,
    mut buttons: Query<(
        &JointCreationControl,
        &mut Node,
        &Interaction,
        &mut BackgroundColor,
    )>,
) {
    let Ok(mut status) = status.single_mut() else {
        return;
    };
    status.0 = match state.phase {
        JointCreationPhase::Inactive => "Press J to begin.".to_owned(),
        JointCreationPhase::AwaitingFirst => "Select object A.".to_owned(),
        JointCreationPhase::AwaitingSecond { .. } => "Select object B.".to_owned(),
        JointCreationPhase::ChoosingType { .. } => "Choose a joint type.".to_owned(),
    };

    let choosing = matches!(state.phase, JointCreationPhase::ChoosingType { .. });
    for (control, mut node, interaction, mut color) in &mut buttons {
        node.display = if matches!(control, JointCreationControl::Choose(_)) && !choosing {
            Display::None
        } else {
            Display::Flex
        };
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::{ButtonState, InputPlugin, keyboard::KeyboardInput, mouse::MouseButtonInput},
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::cursor_hover::{CursorHoverPlugin, CursorRay};

    fn creation_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CursorHoverPlugin,
            JointCreationPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn spawn_body(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                Transform::from_translation(position),
            ))
            .id()
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
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
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn choose(app: &mut App, joint_type: RuntimeJointType) {
        app.world_mut()
            .resource_mut::<Messages<JointCreationAction>>()
            .write(JointCreationAction::Choose(joint_type));
        app.update();
    }

    fn joint_count<T: Component>(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<T>>();
        joints.iter(world).count()
    }

    #[test]
    fn j_key_guides_two_body_selection_before_type_choice() {
        let mut app = creation_app();
        let first = spawn_body(&mut app, Vec3::new(-2.0, 0.0, -5.0));
        let second = spawn_body(&mut app, Vec3::new(2.0, 0.0, -5.0));
        app.update();

        press_key(&mut app, KeyCode::KeyJ);
        assert_eq!(
            app.world().resource::<JointCreationState>().phase,
            JointCreationPhase::AwaitingFirst
        );
        click_at(&mut app, Vec3::new(-2.0, 0.0, -5.0));
        assert_eq!(
            app.world().resource::<JointCreationState>().phase,
            JointCreationPhase::AwaitingSecond { first }
        );
        click_at(&mut app, Vec3::new(2.0, 0.0, -5.0));
        assert_eq!(
            app.world().resource::<JointCreationState>().phase,
            JointCreationPhase::ChoosingType { first, second }
        );
    }

    #[test]
    fn every_supported_joint_type_can_be_created() {
        for (index, joint_type) in RuntimeJointType::ALL.into_iter().enumerate() {
            let mut app = creation_app();
            let first = spawn_body(&mut app, Vec3::new(-2.0, 0.0, -5.0));
            let second = spawn_body(&mut app, Vec3::new(2.0, 0.0, -5.0));
            app.update();
            press_key(&mut app, KeyCode::KeyJ);
            click_at(&mut app, Vec3::new(-2.0, 0.0, -5.0));
            click_at(&mut app, Vec3::new(2.0, 0.0, -5.0));
            choose(&mut app, joint_type);

            assert_eq!(
                joint_count::<RuntimeCreatedJoint>(&mut app),
                1,
                "case {index}"
            );
            let entity = {
                let world = app.world_mut();
                let mut joints = world.query_filtered::<Entity, With<RuntimeCreatedJoint>>();
                joints.iter(world).next().expect("runtime joint")
            };
            let joint = app.world().entity(entity);
            let has_expected_type = match joint_type {
                RuntimeJointType::Fixed => joint.contains::<FixedJoint>(),
                RuntimeJointType::Distance => joint.contains::<DistanceJoint>(),
                RuntimeJointType::Revolute => joint.contains::<RevoluteJoint>(),
                RuntimeJointType::Prismatic => joint.contains::<PrismaticJoint>(),
                RuntimeJointType::Spherical => joint.contains::<SphericalJoint>(),
            };
            assert!(
                has_expected_type,
                "case {index} added the wrong Avian joint type"
            );
            assert_eq!(
                app.world().resource::<JointCreationState>().phase,
                JointCreationPhase::Inactive
            );
            assert!(app.world().entity(first).contains::<RigidBody>());
            assert!(app.world().entity(second).contains::<RigidBody>());
        }
    }

    #[test]
    fn escape_cancels_partial_selection_without_spawning_a_joint() {
        let mut app = creation_app();
        spawn_body(&mut app, Vec3::new(-2.0, 0.0, -5.0));
        app.update();
        press_key(&mut app, KeyCode::KeyJ);
        click_at(&mut app, Vec3::new(-2.0, 0.0, -5.0));
        assert!(matches!(
            app.world().resource::<JointCreationState>().phase,
            JointCreationPhase::AwaitingSecond { .. }
        ));
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(
            app.world().resource::<JointCreationState>().phase,
            JointCreationPhase::Inactive
        );
        assert_eq!(joint_count::<RuntimeCreatedJoint>(&mut app), 0);
    }
}
