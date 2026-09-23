use avian3d::prelude::*;
use bevy::prelude::*;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const WALL_POSITION: Vec3 = Vec3::new(0.0, 1.0, 6.0);
const WALL_SIZE: Vec3 = Vec3::new(0.08, 2.0, 1.2);
const LAUNCH_POSITION: Vec3 = Vec3::new(-2.0, 1.0, 6.0);
const PROJECTILE_RADIUS: f32 = 0.12;

/// The velocity presets used by the linear CCD experiment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LauncherSpeed {
    Low,
    Medium,
    High,
    Extreme,
}

impl LauncherSpeed {
    /// Every speed shown by the launcher, from low to extreme.
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Extreme];

    pub const fn meters_per_second(self) -> f32 {
        match self {
            Self::Low => 10.0,
            Self::Medium => 50.0,
            Self::High => 100.0,
            Self::Extreme => 500.0,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Low => "10",
            Self::Medium => "50",
            Self::High => "100",
            Self::Extreme => "500",
        }
    }
}

/// The collision-detection mode applied to the next launched body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LauncherCcdMode {
    None,
    Speculative,
    SweptLinear,
    SweptNonLinear,
}

impl LauncherCcdMode {
    pub const ALL: [Self; 4] = [
        Self::None,
        Self::Speculative,
        Self::SweptLinear,
        Self::SweptNonLinear,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Speculative => "SPECULATIVE",
            Self::SweptLinear => "SWEPT LINEAR",
            Self::SweptNonLinear => "SWEPT NONLINEAR",
        }
    }
}

/// The observed result of a fired projectile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LauncherOutcome {
    Idle,
    InFlight,
    Collided,
    Tunneled,
}

impl LauncherOutcome {
    const fn label(self) -> &'static str {
        match self {
            Self::Idle => "READY",
            Self::InFlight => "IN FLIGHT",
            Self::Collided => "COLLIDED",
            Self::Tunneled => "TUNNELED",
        }
    }
}

/// Current launcher selection and the last recorded launch result.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CcdLauncherStatus {
    pub speed: LauncherSpeed,
    pub mode: LauncherCcdMode,
    pub outcome: LauncherOutcome,
    pub frames: u32,
}

impl Default for CcdLauncherStatus {
    fn default() -> Self {
        Self {
            speed: LauncherSpeed::Low,
            mode: LauncherCcdMode::None,
            outcome: LauncherOutcome::Idle,
            frames: 0,
        }
    }
}

/// Headless-friendly commands exposed by the launcher controls.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CcdLauncherAction {
    SetSpeed(LauncherSpeed),
    SetMode(LauncherCcdMode),
    Launch,
    Reset,
}

#[derive(Component)]
struct CcdLauncherWall;

#[derive(Component)]
struct CcdLauncherProjectile;

#[derive(Component)]
struct CcdLauncherStatusText;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum CcdLauncherControl {
    Speed(LauncherSpeed),
    Mode(LauncherCcdMode),
    Launch,
    Reset,
}

/// Owns station H's thin-wall high-speed launcher and CCD comparison controls.
pub struct CcdLauncherPlugin;

impl Plugin for CcdLauncherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CcdLauncherStatus>()
            .add_message::<CcdLauncherAction>()
            .add_systems(Startup, (spawn_ccd_launcher_wall, spawn_ccd_launcher_ui))
            .add_systems(
                Update,
                (
                    queue_ccd_launcher_actions,
                    apply_ccd_launcher_actions,
                    update_ccd_launcher_button_colors,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    record_ccd_launcher_outcome,
                    update_ccd_launcher_status_text,
                    draw_ccd_launcher_labels.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

fn spawn_ccd_launcher_wall(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_length(1.0)));
    let material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.25, 0.78, 0.9)));

    let mut wall = commands.spawn((
        CcdLauncherWall,
        RigidBody::Static,
        Collider::cuboid(WALL_SIZE.x, WALL_SIZE.y, WALL_SIZE.z),
        // Keep this lane's baseline discrete mode genuinely discrete. The
        // selected projectile enables speculative or swept CCD explicitly.
        SpeculativeMargin::ZERO,
        Transform::from_translation(WALL_POSITION),
        Name::new("CCD Launcher Thin Wall"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), material.as_ref()) {
        wall.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(WALL_POSITION).with_scale(WALL_SIZE),
        ));
    }
}

fn queue_ccd_launcher_actions(
    controls: Query<(&Interaction, &CcdLauncherControl), Changed<Interaction>>,
    mut actions: MessageWriter<CcdLauncherAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match control {
            CcdLauncherControl::Speed(speed) => CcdLauncherAction::SetSpeed(*speed),
            CcdLauncherControl::Mode(mode) => CcdLauncherAction::SetMode(*mode),
            CcdLauncherControl::Launch => CcdLauncherAction::Launch,
            CcdLauncherControl::Reset => CcdLauncherAction::Reset,
        });
    }
}

fn apply_ccd_launcher_actions(
    mut commands: Commands,
    mut actions: MessageReader<CcdLauncherAction>,
    mut status: ResMut<CcdLauncherStatus>,
    projectiles: Query<Entity, With<CcdLauncherProjectile>>,
) {
    for action in actions.read() {
        match *action {
            CcdLauncherAction::SetSpeed(speed) => status.speed = speed,
            CcdLauncherAction::SetMode(mode) => status.mode = mode,
            CcdLauncherAction::Launch => {
                for projectile in &projectiles {
                    commands.entity(projectile).despawn();
                }

                let speed = status.speed.meters_per_second();
                let mode = status.mode;
                let mut projectile = commands.spawn((
                    CcdLauncherProjectile,
                    RigidBody::Dynamic,
                    Collider::sphere(PROJECTILE_RADIUS),
                    ColliderDensity(1.0),
                    SleepingDisabled,
                    SpeculativeMargin::ZERO,
                    LinearVelocity(Vec3::X * speed),
                    Transform::from_translation(LAUNCH_POSITION),
                    Name::new(format!("CCD Launcher Projectile - {}", mode.label())),
                ));
                match mode {
                    LauncherCcdMode::None => {}
                    LauncherCcdMode::Speculative => {
                        projectile.insert(SpeculativeMargin::MAX);
                    }
                    LauncherCcdMode::SweptLinear => {
                        projectile.insert(SweptCcd::LINEAR);
                    }
                    LauncherCcdMode::SweptNonLinear => {
                        projectile.insert(SweptCcd::NON_LINEAR);
                    }
                }

                status.outcome = LauncherOutcome::InFlight;
                status.frames = 0;
            }
            CcdLauncherAction::Reset => {
                for projectile in &projectiles {
                    commands.entity(projectile).despawn();
                }
                status.outcome = LauncherOutcome::Idle;
                status.frames = 0;
            }
        }
    }
}

fn record_ccd_launcher_outcome(
    contact_graph: Res<ContactGraph>,
    walls: Query<Entity, With<CcdLauncherWall>>,
    projectiles: Query<(Entity, &Position), With<CcdLauncherProjectile>>,
    mut status: ResMut<CcdLauncherStatus>,
) {
    if status.outcome != LauncherOutcome::InFlight {
        return;
    }

    status.frames = status.frames.saturating_add(1);
    let Ok(wall) = walls.single() else {
        return;
    };
    let Ok((projectile, position)) = projectiles.single() else {
        return;
    };

    if contact_graph.contains(wall, projectile) {
        status.outcome = LauncherOutcome::Collided;
    } else if position.0.x > WALL_POSITION.x + WALL_SIZE.x * 0.5 + PROJECTILE_RADIUS {
        status.outcome = LauncherOutcome::Tunneled;
    }
}

fn draw_ccd_launcher_labels(
    mut gizmos: Gizmos,
    wall: Query<&Transform, With<CcdLauncherWall>>,
    projectile: Query<&Transform, With<CcdLauncherProjectile>>,
) {
    if let Ok(transform) = wall.single() {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 1.3, Quat::IDENTITY),
            "THIN WALL",
            0.42,
            Vec2::ZERO,
            Color::srgb(0.25, 0.85, 1.0),
        );
    }
    if let Ok(transform) = projectile.single() {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.35, Quat::IDENTITY),
            "PROJECTILE",
            0.3,
            Vec2::ZERO,
            Color::srgb(1.0, 0.55, 0.2),
        );
    }
}

fn spawn_ccd_launcher_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(16.0),
                width: px(360.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            Name::new("CCD Launcher Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("H — LINEAR CCD LAUNCHER"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Fire a small body through a thin wall and compare tunneling with CCD."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("SPEED (m/s)"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            for speed in LauncherSpeed::ALL {
                spawn_launcher_button(parent, speed.label(), CcdLauncherControl::Speed(speed));
            }
            parent.spawn((
                Text::new("CCD MODE"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            for mode in LauncherCcdMode::ALL {
                spawn_launcher_button(parent, mode.label(), CcdLauncherControl::Mode(mode));
            }
            parent
                .spawn((
                    Button,
                    CcdLauncherControl::Launch,
                    Node {
                        height: px(24.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("CCD launcher fire"),
                ))
                .with_child((
                    Text::new("FIRE"),
                    TextFont::from_font_size(12.0),
                    TextColor(PANEL_TEXT),
                ));
            parent
                .spawn((
                    Button,
                    CcdLauncherControl::Reset,
                    Node {
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("CCD launcher reset"),
                ))
                .with_child((
                    Text::new("RESET"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
            parent.spawn((
                Text::new("READY"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                CcdLauncherStatusText,
            ));
        });
}

fn spawn_launcher_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    control: CcdLauncherControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                height: px(21.0),
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

fn update_ccd_launcher_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<CcdLauncherControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn update_ccd_launcher_status_text(
    status: Res<CcdLauncherStatus>,
    mut labels: Query<&mut Text, With<CcdLauncherStatusText>>,
) {
    if !status.is_changed() {
        return;
    }
    for mut label in &mut labels {
        *label = Text::new(format!(
            "{} · {} m/s · {} · frame {}",
            status.outcome.label(),
            status.speed.meters_per_second(),
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

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn launcher_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CcdLauncherPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn action(app: &mut App, action: CcdLauncherAction) {
        app.world_mut()
            .resource_mut::<Messages<CcdLauncherAction>>()
            .write(action);
    }

    fn run_until_complete(app: &mut App) {
        for _ in 0..180 {
            app.update();
            if app.world().resource::<CcdLauncherStatus>().outcome != LauncherOutcome::InFlight {
                return;
            }
        }
        panic!("launcher did not record a collision or tunnel");
    }

    #[test]
    fn speed_presets_include_low_and_extreme_velocities() {
        let speeds: Vec<_> = LauncherSpeed::ALL
            .into_iter()
            .map(LauncherSpeed::meters_per_second)
            .collect();
        assert_eq!(speeds, [10.0, 50.0, 100.0, 500.0]);
    }

    #[test]
    fn launch_applies_the_selected_speed_and_swept_linear_mode() {
        let mut app = launcher_app();
        app.update();
        action(
            &mut app,
            CcdLauncherAction::SetSpeed(LauncherSpeed::Extreme),
        );
        action(
            &mut app,
            CcdLauncherAction::SetMode(LauncherCcdMode::SweptLinear),
        );
        action(&mut app, CcdLauncherAction::Launch);
        app.update();

        let world = app.world_mut();
        let mut projectiles = world.query::<(&LinearVelocity, &SweptCcd)>();
        let (velocity, ccd) = projectiles.iter(world).next().expect("launched projectile");
        assert_eq!(velocity.0, Vec3::X * 500.0);
        assert_eq!(ccd.mode, SweepMode::Linear);
        assert_eq!(
            world.resource::<CcdLauncherStatus>().mode,
            LauncherCcdMode::SweptLinear
        );
    }

    #[test]
    fn every_speed_records_a_result_for_the_discrete_launcher() {
        for speed in LauncherSpeed::ALL {
            let mut app = launcher_app();
            app.update();
            action(&mut app, CcdLauncherAction::SetSpeed(speed));
            action(&mut app, CcdLauncherAction::SetMode(LauncherCcdMode::None));
            action(&mut app, CcdLauncherAction::Launch);
            run_until_complete(&mut app);

            assert_ne!(
                app.world().resource::<CcdLauncherStatus>().outcome,
                LauncherOutcome::InFlight,
                "speed {} did not produce a recorded result",
                speed.meters_per_second()
            );
        }
    }

    #[test]
    fn extreme_discrete_launch_tunnels_but_each_supported_ccd_mode_collides() {
        let mut discrete = launcher_app();
        discrete.update();
        action(
            &mut discrete,
            CcdLauncherAction::SetSpeed(LauncherSpeed::Extreme),
        );
        action(
            &mut discrete,
            CcdLauncherAction::SetMode(LauncherCcdMode::None),
        );
        action(&mut discrete, CcdLauncherAction::Launch);
        run_until_complete(&mut discrete);
        assert_eq!(
            discrete.world().resource::<CcdLauncherStatus>().outcome,
            LauncherOutcome::Tunneled
        );

        for mode in [
            LauncherCcdMode::Speculative,
            LauncherCcdMode::SweptLinear,
            LauncherCcdMode::SweptNonLinear,
        ] {
            let mut app = launcher_app();
            app.update();
            action(&mut app, CcdLauncherAction::SetSpeed(LauncherSpeed::High));
            action(&mut app, CcdLauncherAction::SetMode(mode));
            action(&mut app, CcdLauncherAction::Launch);
            run_until_complete(&mut app);
            let outcome = app.world().resource::<CcdLauncherStatus>().outcome;
            assert_ne!(outcome, LauncherOutcome::InFlight);
            if matches!(
                mode,
                LauncherCcdMode::SweptLinear | LauncherCcdMode::SweptNonLinear
            ) {
                assert_eq!(
                    outcome,
                    LauncherOutcome::Collided,
                    "{mode:?} did not stop the extreme projectile"
                );
            }
        }
    }
}
