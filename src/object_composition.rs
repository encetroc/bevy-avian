use avian3d::prelude::*;
use bevy::prelude::*;

use crate::cursor_hover::{HoverState, update_hover_reachability};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// The phase of the C-key object composition workflow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompositionPhase {
    #[default]
    Inactive,
    AwaitingFirst,
    AwaitingSecond {
        first: Entity,
    },
}

/// Runtime state for selecting two physical objects to compose.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompositionState {
    pub phase: CompositionPhase,
}

/// Marks a body that belongs to at least one composed structure.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComposedObject;

/// Marks the fixed joint created by object composition mode.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompositionJoint;

/// Headless-friendly commands for the composition workflow.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionAction {
    Cancel,
}

/// Owns the C-key workflow that connects two selected physics objects with a
/// fixed joint while leaving both bodies in Avian's normal simulation.
pub struct ObjectCompositionPlugin;

impl Plugin for ObjectCompositionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompositionState>()
            .add_message::<CompositionAction>()
            .add_systems(Startup, spawn_composition_ui)
            .add_systems(
                Update,
                (
                    handle_composition_keyboard,
                    select_composition_bodies.after(update_hover_reachability),
                    queue_composition_actions,
                    apply_composition_actions,
                    update_composition_ui,
                )
                    .chain(),
            );
    }
}

fn handle_composition_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CompositionState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        state.phase = CompositionPhase::Inactive;
    } else if keyboard.just_pressed(KeyCode::KeyC) {
        state.phase = match state.phase {
            CompositionPhase::Inactive => CompositionPhase::AwaitingFirst,
            _ => CompositionPhase::Inactive,
        };
    }
}

fn select_composition_bodies(
    mouse: Res<ButtonInput<MouseButton>>,
    hover: Res<HoverState>,
    controls: Query<&Interaction, With<CompositionControl>>,
    bodies: Query<&Transform, (With<Collider>, With<RigidBody>)>,
    mut commands: Commands,
    mut state: ResMut<CompositionState>,
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

    match state.phase {
        CompositionPhase::AwaitingFirst => {
            state.phase = CompositionPhase::AwaitingSecond { first: entity };
        }
        CompositionPhase::AwaitingSecond { first } if first != entity => {
            let Ok([transform1, transform2]) = bodies.get_many([first, entity]) else {
                state.phase = CompositionPhase::Inactive;
                return;
            };
            spawn_composition_joint(&mut commands, first, entity, transform1, transform2);
            state.phase = CompositionPhase::Inactive;
        }
        _ => {}
    }
}

fn queue_composition_actions(
    controls: Query<(&Interaction, &CompositionControl), Changed<Interaction>>,
    mut actions: MessageWriter<CompositionAction>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            match control {
                CompositionControl::Cancel => {
                    actions.write(CompositionAction::Cancel);
                }
            }
        }
    }
}

fn apply_composition_actions(
    mut actions: MessageReader<CompositionAction>,
    mut state: ResMut<CompositionState>,
) {
    if actions
        .read()
        .any(|action| *action == CompositionAction::Cancel)
    {
        state.phase = CompositionPhase::Inactive;
    }
}

fn spawn_composition_joint(
    commands: &mut Commands,
    body1: Entity,
    body2: Entity,
    transform1: &Transform,
    transform2: &Transform,
) {
    let anchor = (transform1.translation + transform2.translation) * 0.5;
    let local_anchor1 = transform1.rotation.inverse() * (anchor - transform1.translation);
    let local_anchor2 = transform2.rotation.inverse() * (anchor - transform2.translation);

    commands.entity(body1).insert(ComposedObject);
    commands.entity(body2).insert(ComposedObject);
    commands.spawn((
        FixedJoint::new(body1, body2)
            .with_local_anchor1(local_anchor1)
            .with_local_anchor2(local_anchor2),
        JointForces::new(),
        JointCollisionDisabled,
        CompositionJoint,
        Name::new("Composition Fixed Joint"),
    ));
}

#[derive(Component)]
struct CompositionPanel;

#[derive(Component)]
struct CompositionStatusText;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum CompositionControl {
    Cancel,
}

fn spawn_composition_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(230.0),
                width: px(360.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            CompositionPanel,
            Name::new("Object Composition Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("C — COMPOSE OBJECTS"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Press C, then click two physics objects to fix them together."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Press C to begin."),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                CompositionStatusText,
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            spawn_composition_button(parent, "cancel", CompositionControl::Cancel);
        });
}

fn spawn_composition_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: CompositionControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(72.0),
                height: px(22.0),
                margin: UiRect::top(px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Composition control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(10.0),
            TextColor(PANEL_TEXT),
        ));
}

fn update_composition_ui(
    state: Res<CompositionState>,
    mut status: Query<&mut Text, With<CompositionStatusText>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<CompositionControl>>,
) {
    let Ok(mut status) = status.single_mut() else {
        return;
    };
    status.0 = match state.phase {
        CompositionPhase::Inactive => "Press C to begin.".to_owned(),
        CompositionPhase::AwaitingFirst => "Select the plank or first support.".to_owned(),
        CompositionPhase::AwaitingSecond { .. } => "Select the second object.".to_owned(),
    };

    for (interaction, mut color) in &mut buttons {
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

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn composition_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CursorHoverPlugin,
            ObjectCompositionPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn spawn_body(app: &mut App, name: &str, position: Vec3, size: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(size.x, size.y, size.z),
                Position(position),
                Transform::from_translation(position),
                Name::new(name.to_owned()),
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

    fn joint_entities(app: &mut App) -> Vec<Entity> {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<CompositionJoint>>();
        joints.iter(world).collect()
    }

    fn relative_pose(app: &App, first: Entity, second: Entity) -> (Vec3, Quat) {
        let first_position = app.world().entity(first).get::<Position>().unwrap().0;
        let second_position = app.world().entity(second).get::<Position>().unwrap().0;
        let first_rotation = app.world().entity(first).get::<Rotation>().unwrap().0;
        let second_rotation = app.world().entity(second).get::<Rotation>().unwrap().0;
        (
            first_rotation.inverse() * (second_position - first_position),
            first_rotation.inverse() * second_rotation,
        )
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    #[test]
    fn c_key_selects_two_objects_and_creates_a_fixed_composition_joint() {
        let mut app = composition_app();
        let first = spawn_body(
            &mut app,
            "Composition First",
            Vec3::new(-2.0, 0.0, -5.0),
            Vec3::splat(0.5),
        );
        let second = spawn_body(
            &mut app,
            "Composition Second",
            Vec3::new(2.0, 0.0, -5.0),
            Vec3::splat(0.5),
        );
        app.update();

        press_key(&mut app, KeyCode::KeyC);
        assert_eq!(
            app.world().resource::<CompositionState>().phase,
            CompositionPhase::AwaitingFirst
        );
        click_at(&mut app, Vec3::new(-2.0, 0.0, -5.0));
        assert_eq!(
            app.world().resource::<CompositionState>().phase,
            CompositionPhase::AwaitingSecond { first }
        );
        click_at(&mut app, Vec3::new(2.0, 0.0, -5.0));

        let joints = joint_entities(&mut app);
        assert_eq!(joints.len(), 1);
        let joint = app.world().entity(joints[0]);
        let fixed = joint.get::<FixedJoint>().expect("fixed composition joint");
        assert_eq!([fixed.body1, fixed.body2], [first, second]);
        assert!(app.world().entity(first).contains::<ComposedObject>());
        assert!(app.world().entity(second).contains::<ComposedObject>());
        assert_eq!(
            app.world().resource::<CompositionState>().phase,
            CompositionPhase::Inactive
        );
    }

    #[test]
    fn plank_and_two_supports_remain_connected_after_a_collision() {
        let mut app = composition_app();
        let plank = spawn_body(
            &mut app,
            "Plank",
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::new(1.8, 0.5, 0.5),
        );
        let left_support = spawn_body(
            &mut app,
            "Left Support",
            Vec3::new(-1.2, 0.0, -5.0),
            Vec3::splat(0.5),
        );
        let right_support = spawn_body(
            &mut app,
            "Right Support",
            Vec3::new(1.2, 0.0, -5.0),
            Vec3::splat(0.5),
        );
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(0.35, 3.0, 3.0),
            Transform::from_xyz(2.25, 0.0, -5.0),
            Name::new("Composition Collision Wall"),
        ));
        app.update();

        for (first, second) in [(plank, left_support), (plank, right_support)] {
            press_key(&mut app, KeyCode::KeyC);
            let first_position = app.world().entity(first).get::<Position>().unwrap().0;
            let second_position = app.world().entity(second).get::<Position>().unwrap().0;
            click_at(&mut app, first_position);
            click_at(&mut app, second_position);
        }

        let initial_left = relative_pose(&app, plank, left_support);
        let initial_right = relative_pose(&app, plank, right_support);
        for body in [plank, left_support, right_support] {
            app.world_mut()
                .entity_mut(body)
                .insert(LinearVelocity(Vec3::new(15.0, 0.0, 0.0)));
        }
        run_steps(&mut app, 120);

        let final_left = relative_pose(&app, plank, left_support);
        let final_right = relative_pose(&app, plank, right_support);
        assert!(final_left.0.distance(initial_left.0) < 0.12);
        assert!(final_right.0.distance(initial_right.0) < 0.12);
        assert!(final_left.1.angle_between(initial_left.1) < 0.12);
        assert!(final_right.1.angle_between(initial_right.1) < 0.12);
        assert_eq!(joint_entities(&mut app).len(), 2);
    }

    #[test]
    fn escape_cancels_a_partial_composition_without_creating_a_joint() {
        let mut app = composition_app();
        spawn_body(
            &mut app,
            "Composition First",
            Vec3::new(-2.0, 0.0, -5.0),
            Vec3::splat(0.5),
        );
        app.update();
        press_key(&mut app, KeyCode::KeyC);
        press_key(&mut app, KeyCode::Escape);

        assert_eq!(
            app.world().resource::<CompositionState>().phase,
            CompositionPhase::Inactive
        );
        assert!(joint_entities(&mut app).is_empty());
    }
}
