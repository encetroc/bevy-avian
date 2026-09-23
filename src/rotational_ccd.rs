use avian3d::prelude::*;
use bevy::prelude::*;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const ROTOR_POSITION: Vec3 = Vec3::new(1.55, 1.1, 7.0);
const ROTOR_SIZE: Vec3 = Vec3::new(2.0, 0.12, 0.12);
const TARGET_POSITION: Vec3 = Vec3::new(0.87, 1.1, 6.52);
const TARGET_RADIUS: f32 = 0.16;
const SPIN_SPEED: f32 = 240.0;
const OBSERVATION_FRAMES: u32 = 8;

/// The swept CCD algorithm used by the rotational experiment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RotationalCcdMode {
    SweptLinear,
    SweptNonLinear,
}

impl RotationalCcdMode {
    pub const ALL: [Self; 2] = [Self::SweptLinear, Self::SweptNonLinear];

    const fn label(self) -> &'static str {
        match self {
            Self::SweptLinear => "SWEPT LINEAR",
            Self::SweptNonLinear => "SWEPT NONLINEAR",
        }
    }

    const fn sweep_mode(self) -> SweepMode {
        match self {
            Self::SweptLinear => SweepMode::Linear,
            Self::SweptNonLinear => SweepMode::NonLinear,
        }
    }
}

/// What happened during the most recent rotational CCD run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RotationalCcdOutcome {
    Ready,
    Spinning,
    Hit,
    NoHit,
}

impl RotationalCcdOutcome {
    const fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Spinning => "SPINNING",
            Self::Hit => "HIT",
            Self::NoHit => "NO HIT",
        }
    }
}

/// Current mode and result for the repeatable rotating-body experiment.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RotationalCcdStatus {
    pub mode: RotationalCcdMode,
    pub outcome: RotationalCcdOutcome,
    pub frames: u32,
}

impl Default for RotationalCcdStatus {
    fn default() -> Self {
        Self {
            mode: RotationalCcdMode::SweptLinear,
            outcome: RotationalCcdOutcome::Ready,
            frames: 0,
        }
    }
}

/// Headless-friendly commands for selecting or repeating a rotational CCD run.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum RotationalCcdAction {
    SetMode(RotationalCcdMode),
    Run,
    Reset,
}

#[derive(Component)]
struct RotationalCcdRotor;

#[derive(Component)]
struct RotationalCcdTarget;

#[derive(Component)]
struct RotationalCcdStatusText;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum RotationalCcdControl {
    Mode(RotationalCcdMode),
    Run,
    Reset,
}

/// Owns station H's angular-motion CCD comparison.
pub struct RotationalCcdPlugin;

impl Plugin for RotationalCcdPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RotationalCcdStatus>()
            .add_message::<RotationalCcdAction>()
            .add_systems(
                Startup,
                (spawn_rotational_ccd_test, spawn_rotational_ccd_ui),
            )
            .add_systems(
                Update,
                (
                    queue_rotational_ccd_actions,
                    apply_rotational_ccd_actions,
                    update_rotational_ccd_button_colors,
                    finish_rotational_ccd_run,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    record_rotational_ccd_hit,
                    update_rotational_ccd_status_text,
                    draw_rotational_ccd_labels.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

fn spawn_rotational_ccd_test(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_length(1.0)));
    let rotor_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(1.0, 0.48, 0.12)));
    let target_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.25, 0.85, 1.0)));

    let mut rotor = commands.spawn((
        RotationalCcdRotor,
        RigidBody::Dynamic,
        Collider::cuboid(ROTOR_SIZE.x, ROTOR_SIZE.y, ROTOR_SIZE.z),
        ColliderDensity(1.0),
        SleepingDisabled,
        SpeculativeMargin::ZERO,
        Transform::from_translation(ROTOR_POSITION),
        Name::new("Rotational CCD Test Rotor"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), rotor_material.as_ref()) {
        rotor.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(ROTOR_POSITION).with_scale(ROTOR_SIZE),
        ));
    }

    let mut target = commands.spawn((
        RotationalCcdTarget,
        RigidBody::Static,
        Collider::sphere(TARGET_RADIUS),
        SpeculativeMargin::ZERO,
        Transform::from_translation(TARGET_POSITION),
        Name::new("Rotational CCD Sweep Target"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), target_material.as_ref()) {
        target.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(TARGET_POSITION)
                .with_scale(Vec3::splat(TARGET_RADIUS * 2.0)),
        ));
    }
}

fn queue_rotational_ccd_actions(
    controls: Query<(&Interaction, &RotationalCcdControl), Changed<Interaction>>,
    mut actions: MessageWriter<RotationalCcdAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match control {
            RotationalCcdControl::Mode(mode) => RotationalCcdAction::SetMode(*mode),
            RotationalCcdControl::Run => RotationalCcdAction::Run,
            RotationalCcdControl::Reset => RotationalCcdAction::Reset,
        });
    }
}

fn apply_rotational_ccd_actions(
    mut commands: Commands,
    mut actions: MessageReader<RotationalCcdAction>,
    mut status: ResMut<RotationalCcdStatus>,
    rotors: Query<Entity, With<RotationalCcdRotor>>,
) {
    for action in actions.read() {
        match *action {
            RotationalCcdAction::SetMode(mode) => status.mode = mode,
            RotationalCcdAction::Run => {
                for rotor in &rotors {
                    commands.entity(rotor).insert((
                        RigidBody::Dynamic,
                        Position(ROTOR_POSITION),
                        Rotation::IDENTITY,
                        Transform::from_translation(ROTOR_POSITION),
                        AngularVelocity(Vec3::Y * SPIN_SPEED),
                        SweptCcd::new_with_mode(status.mode.sweep_mode()),
                    ));
                }
                status.outcome = RotationalCcdOutcome::Spinning;
                status.frames = 0;
            }
            RotationalCcdAction::Reset => {
                for rotor in &rotors {
                    commands.entity(rotor).insert((
                        RigidBody::Dynamic,
                        Position(ROTOR_POSITION),
                        Rotation::IDENTITY,
                        Transform::from_translation(ROTOR_POSITION),
                        AngularVelocity::ZERO,
                    ));
                    commands.entity(rotor).remove::<SweptCcd>();
                }
                status.outcome = RotationalCcdOutcome::Ready;
                status.frames = 0;
            }
        }
    }
}

fn record_rotational_ccd_hit(
    contact_graph: Res<ContactGraph>,
    rotors: Query<Entity, With<RotationalCcdRotor>>,
    targets: Query<Entity, With<RotationalCcdTarget>>,
    mut status: ResMut<RotationalCcdStatus>,
) {
    if status.outcome != RotationalCcdOutcome::Spinning {
        return;
    }
    let (Ok(rotor), Ok(target)) = (rotors.single(), targets.single()) else {
        return;
    };
    if contact_graph.contains(rotor, target) {
        status.outcome = RotationalCcdOutcome::Hit;
    }
}

fn finish_rotational_ccd_run(mut status: ResMut<RotationalCcdStatus>) {
    if status.outcome != RotationalCcdOutcome::Spinning {
        return;
    }
    status.frames = status.frames.saturating_add(1);
    if status.frames >= OBSERVATION_FRAMES {
        status.outcome = RotationalCcdOutcome::NoHit;
    }
}

fn draw_rotational_ccd_labels(
    mut gizmos: Gizmos,
    rotor: Query<&Transform, With<RotationalCcdRotor>>,
    target: Query<&Transform, With<RotationalCcdTarget>>,
) {
    if let Ok(transform) = rotor.single() {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.35, Quat::IDENTITY),
            "ROTATING BAR",
            0.3,
            Vec2::ZERO,
            Color::srgb(1.0, 0.55, 0.2),
        );
    }
    if let Ok(transform) = target.single() {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.35, Quat::IDENTITY),
            "ROTATION TARGET",
            0.28,
            Vec2::ZERO,
            Color::srgb(0.25, 0.85, 1.0),
        );
    }
}

fn spawn_rotational_ccd_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(390.0),
                width: px(360.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            Name::new("Rotational CCD Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("H — ROTATIONAL CCD TEST"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Spin a long bar past the target; linear and nonlinear swept CCD differ on angular motion."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            for mode in RotationalCcdMode::ALL {
                spawn_rotational_ccd_button(parent, mode.label(), RotationalCcdControl::Mode(mode));
            }
            spawn_rotational_ccd_button(parent, "SPIN / REPEAT", RotationalCcdControl::Run);
            spawn_rotational_ccd_button(parent, "RESET", RotationalCcdControl::Reset);
            parent.spawn((
                Text::new("READY"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                RotationalCcdStatusText,
            ));
        });
}

fn spawn_rotational_ccd_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    control: RotationalCcdControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                height: px(22.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn update_rotational_ccd_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<RotationalCcdControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn update_rotational_ccd_status_text(
    status: Res<RotationalCcdStatus>,
    mut labels: Query<&mut Text, With<RotationalCcdStatusText>>,
) {
    if !status.is_changed() {
        return;
    }
    for mut label in &mut labels {
        *label = Text::new(format!(
            "{} · {} · {} frames",
            status.outcome.label(),
            status.mode.label(),
            status.frames
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    fn rotational_ccd_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            RotationalCcdPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn send_action(app: &mut App, action: RotationalCcdAction) {
        app.world_mut()
            .resource_mut::<Messages<RotationalCcdAction>>()
            .write(action);
    }

    #[test]
    fn both_swept_modes_are_repeatable_and_record_rotational_results() {
        for mode in RotationalCcdMode::ALL {
            let mut app = rotational_ccd_app();
            app.update();
            for _ in 0..2 {
                send_action(&mut app, RotationalCcdAction::SetMode(mode));
                send_action(&mut app, RotationalCcdAction::Run);
                for _ in 0..OBSERVATION_FRAMES + 2 {
                    app.update();
                }

                let status = app.world().resource::<RotationalCcdStatus>();
                assert!(matches!(
                    status.outcome,
                    RotationalCcdOutcome::Hit | RotationalCcdOutcome::NoHit
                ));
                assert!(status.frames > 0);
                assert!(matches!(
                    status.outcome,
                    RotationalCcdOutcome::Hit | RotationalCcdOutcome::NoHit
                ));
                send_action(&mut app, RotationalCcdAction::Reset);
                app.update();
                assert_eq!(
                    app.world().resource::<RotationalCcdStatus>().outcome,
                    RotationalCcdOutcome::Ready
                );
            }
        }
    }

    #[test]
    fn each_selected_mode_is_applied_to_the_rotor_and_reset_removes_it() {
        let mut app = rotational_ccd_app();
        app.update();

        for mode in RotationalCcdMode::ALL {
            send_action(&mut app, RotationalCcdAction::SetMode(mode));
            send_action(&mut app, RotationalCcdAction::Run);
            app.update();

            let world = app.world_mut();
            let mut rotors =
                world.query_filtered::<(&SweptCcd, &AngularVelocity), With<RotationalCcdRotor>>();
            let (ccd, angular_velocity) = rotors.single(world).expect("the test rotor is spawned");
            assert_eq!(ccd.mode, mode.sweep_mode());
            assert_eq!(angular_velocity.0, Vec3::Y * SPIN_SPEED);

            send_action(&mut app, RotationalCcdAction::Reset);
            app.update();
            let world = app.world_mut();
            let mut rotors = world.query_filtered::<
                (Option<&SweptCcd>, &AngularVelocity, &Rotation),
                With<RotationalCcdRotor>,
            >();
            let (ccd, angular_velocity, rotation) =
                rotors.single(world).expect("the test rotor remains");
            assert!(ccd.is_none());
            assert_eq!(angular_velocity.0, Vec3::ZERO);
            assert_eq!(rotation.0, Quat::IDENTITY);
        }
    }

    #[test]
    fn nonlinear_swept_ccd_detects_the_rotating_bar_sweep() {
        let mut app = rotational_ccd_app();
        app.update();
        send_action(
            &mut app,
            RotationalCcdAction::SetMode(RotationalCcdMode::SweptNonLinear),
        );
        send_action(&mut app, RotationalCcdAction::Run);
        for _ in 0..OBSERVATION_FRAMES + 2 {
            app.update();
            if app.world().resource::<RotationalCcdStatus>().outcome
                != RotationalCcdOutcome::Spinning
            {
                break;
            }
        }
        assert_eq!(
            app.world().resource::<RotationalCcdStatus>().outcome,
            RotationalCcdOutcome::Hit
        );
    }
}
