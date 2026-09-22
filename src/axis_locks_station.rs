use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::ResetStation;
use crate::stations::StationObject;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

const DEMO_POSITION: Vec3 = Vec3::new(0.0, 0.6, 8.5);
const DOMINANCE_DISTANCE: f32 = 1.8;
const DOMINANCE_SPEED: f32 = 2.0;

/// The six independently locked-axis bodies used by the runtime comparison.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisLockDemoKind {
    TranslationX,
    TranslationY,
    TranslationZ,
    RotationX,
    RotationY,
    RotationZ,
}

impl AxisLockDemoKind {
    const ALL: [Self; 6] = [
        Self::TranslationX,
        Self::TranslationY,
        Self::TranslationZ,
        Self::RotationX,
        Self::RotationY,
        Self::RotationZ,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::TranslationX => "Translation X",
            Self::TranslationY => "Translation Y",
            Self::TranslationZ => "Translation Z",
            Self::RotationX => "Rotation X",
            Self::RotationY => "Rotation Y",
            Self::RotationZ => "Rotation Z",
        }
    }

    const fn locked_axes(self) -> LockedAxes {
        match self {
            Self::TranslationX => LockedAxes::new().lock_translation_x(),
            Self::TranslationY => LockedAxes::new().lock_translation_y(),
            Self::TranslationZ => LockedAxes::new().lock_translation_z(),
            Self::RotationX => LockedAxes::new().lock_rotation_x(),
            Self::RotationY => LockedAxes::new().lock_rotation_y(),
            Self::RotationZ => LockedAxes::new().lock_rotation_z(),
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::TranslationX | Self::RotationX => Color::srgb(0.25, 0.7, 0.95),
            Self::TranslationY | Self::RotationY => Color::srgb(0.95, 0.7, 0.25),
            Self::TranslationZ | Self::RotationZ => Color::srgb(0.85, 0.35, 0.8),
        }
    }
}

/// Marks an axis-lock comparison body in the J station.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct AxisLockDemoObject {
    pub kind: AxisLockDemoKind,
}

/// Marks one side of the dominance collision comparison.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum DominanceDemoKind {
    Normal,
    Dominant,
}

impl DominanceDemoKind {
    const ALL: [Self; 2] = [Self::Normal, Self::Dominant];

    const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Dominance 0",
            Self::Dominant => "Dominance 5",
        }
    }

    const fn value(self) -> i8 {
        match self {
            Self::Normal => 0,
            Self::Dominant => 5,
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Normal => Color::srgb(0.3, 0.9, 0.45),
            Self::Dominant => Color::srgb(0.95, 0.3, 0.25),
        }
    }
}

/// Owns the locked-axis and dominance comparison bodies.
pub struct AxisLocksStationPlugin;

impl Plugin for AxisLocksStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (spawn_axis_lock_demo_objects, spawn_axis_locks_station_ui),
        )
        .add_systems(
            Update,
            (
                queue_axis_locks_station_actions,
                update_axis_locks_button_colors,
            ),
        );
    }
}

fn spawn_axis_lock_demo_objects(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(0.38))));

    for (index, kind) in AxisLockDemoKind::ALL.into_iter().enumerate() {
        let position = DEMO_POSITION + Vec3::new(-1.2 + index as f32 * 0.48, 0.0, 0.0);
        let initial_transform = Transform::from_translation(position);
        let mut object = commands.spawn((
            AxisLockDemoObject { kind },
            StationObject::new('J', initial_transform),
            RigidBody::Dynamic,
            Collider::cuboid(0.19, 0.19, 0.19),
            LockedAxes::new(),
            GravityScale(0.0),
            LinearDamping(0.8),
            AngularDamping(0.8),
            ConstantForce(Vec3::splat(1.8)),
            ConstantTorque(Vec3::splat(1.2)),
            SleepingDisabled,
            initial_transform,
            Name::new(format!("Axis lock demo - {}", kind.label())),
        ));
        object.insert(kind.locked_axes());

        if let (Some(mesh), Some(materials)) = (mesh.as_ref(), materials.as_mut()) {
            object.insert((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(kind.color())),
            ));
        }
    }

    for kind in DominanceDemoKind::ALL {
        let x = match kind {
            DominanceDemoKind::Normal => -DOMINANCE_DISTANCE * 0.5,
            DominanceDemoKind::Dominant => DOMINANCE_DISTANCE * 0.5,
        };
        let position = Vec3::new(x, 0.6, 9.25);
        let velocity = match kind {
            DominanceDemoKind::Normal => Vec3::X * DOMINANCE_SPEED,
            DominanceDemoKind::Dominant => Vec3::NEG_X * DOMINANCE_SPEED,
        };
        let initial_transform = Transform::from_translation(position);
        let mut object = commands.spawn((
            kind,
            StationObject::new('J', initial_transform).with_velocities(velocity, Vec3::ZERO),
            RigidBody::Dynamic,
            Collider::cuboid(0.28, 0.28, 0.28),
            Mass(1.0),
            Dominance(kind.value()),
            Friction::ZERO,
            Restitution::ZERO,
            GravityScale(0.0),
            LinearDamping(0.0),
            AngularDamping(0.0),
            LinearVelocity(velocity),
            SleepingDisabled,
            initial_transform,
            Name::new(format!("Dominance demo - {}", kind.label())),
        ));

        if let (Some(mesh), Some(materials)) = (mesh.as_ref(), materials.as_mut()) {
            object.insert((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(materials.add(kind.color())),
            ));
        }
    }
}

#[derive(Component)]
struct AxisLocksStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum AxisLocksStationControl {
    Reset,
}

fn spawn_axis_locks_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(16.0),
                width: px(390.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            AxisLocksStationPanel,
            Name::new("Axis Locks and Dominance Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("J — AXIS LOCKS & DOMINANCE"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Each colored body receives force and torque with one axis locked. The dominance pair collides head-on: the Dominance 5 body should remain the effective immovable body."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            for kind in AxisLockDemoKind::ALL {
                parent.spawn((
                    Text::new(format!("{} locked", kind.label())),
                    TextFont::from_font_size(12.0),
                    TextColor(kind.color()),
                ));
            }
            parent.spawn((
                Text::new("Dominance 0  ↔  Dominance 5"),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            parent
                .spawn((
                    Button,
                    AxisLocksStationControl::Reset,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Axis locks station reset"),
                ))
                .with_child((
                    Text::new("reset station J"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_axis_locks_station_actions(
    controls: Query<(&Interaction, &AxisLocksStationControl), Changed<Interaction>>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed && *control == AxisLocksStationControl::Reset {
            resets.write(ResetStation { code: 'J' });
        }
    }
}

fn update_axis_locks_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<AxisLocksStationControl>>,
) {
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

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn station_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            AxisLocksStationPlugin,
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

    #[test]
    fn station_spawns_all_axis_locks_and_dominance_values() {
        let mut app = station_app();
        app.update();

        let world = app.world_mut();
        let mut axis_objects = world.query::<(&AxisLockDemoObject, &LockedAxes)>();
        let axis_objects: Vec<_> = axis_objects.iter(world).collect();
        assert_eq!(axis_objects.len(), AxisLockDemoKind::ALL.len());
        for (demo, locked_axes) in axis_objects {
            assert_eq!(locked_axes.to_bits(), demo.kind.locked_axes().to_bits());
        }

        let mut dominance = world.query::<(&DominanceDemoKind, &Dominance)>();
        let dominance: Vec<_> = dominance.iter(world).collect();
        assert_eq!(dominance.len(), DominanceDemoKind::ALL.len());
        for (kind, value) in dominance {
            assert_eq!(value.0, kind.value());
        }
    }

    #[test]
    fn dominant_body_is_less_affected_by_a_collision() {
        let mut app = station_app();
        app.update();

        let (normal, dominant) = {
            let world = app.world_mut();
            let mut bodies = world.query::<(Entity, &DominanceDemoKind)>();
            let mut normal = None;
            let mut dominant = None;
            for (entity, kind) in bodies.iter(world) {
                match kind {
                    DominanceDemoKind::Normal => normal = Some(entity),
                    DominanceDemoKind::Dominant => dominant = Some(entity),
                }
            }
            (
                normal.expect("normal dominance body"),
                dominant.expect("dominant body"),
            )
        };

        let normal_start = app.world().entity(normal).get::<Position>().unwrap().0;
        let dominant_start = app.world().entity(dominant).get::<Position>().unwrap().0;
        run_steps(&mut app, 120);

        let normal_velocity = app
            .world()
            .entity(normal)
            .get::<LinearVelocity>()
            .unwrap()
            .0;
        let dominant_velocity = app
            .world()
            .entity(dominant)
            .get::<LinearVelocity>()
            .unwrap()
            .0;
        let normal_distance = app
            .world()
            .entity(normal)
            .get::<Position>()
            .unwrap()
            .0
            .distance(normal_start);
        let dominant_distance = app
            .world()
            .entity(dominant)
            .get::<Position>()
            .unwrap()
            .0
            .distance(dominant_start);

        assert!(
            dominant_velocity.x < -1.0,
            "dominant body lost its incoming velocity: {dominant_velocity:?}"
        );
        assert!(
            normal_velocity.x < 0.5,
            "normal body was not affected by the dominant collision: {normal_velocity:?}"
        );
        assert!(
            dominant_distance > normal_distance,
            "dominance did not produce different collision outcomes: dominant distance {dominant_distance}, normal distance {normal_distance}"
        );
    }
}
