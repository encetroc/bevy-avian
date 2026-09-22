use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

const LINEAR_VELOCITY: Vec3 = Vec3::new(2.0, 0.0, 0.0);
const ANGULAR_VELOCITY: Vec3 = Vec3::new(0.0, 6.0, 0.0);

/// The six bodies used to compare linear and angular damping independently.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum DampingDemoKind {
    LinearLow,
    LinearMedium,
    LinearHigh,
    AngularLow,
    AngularMedium,
    AngularHigh,
}

impl DampingDemoKind {
    const ALL: [Self; 6] = [
        Self::LinearLow,
        Self::LinearMedium,
        Self::LinearHigh,
        Self::AngularLow,
        Self::AngularMedium,
        Self::AngularHigh,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::LinearLow => "Linear low",
            Self::LinearMedium => "Linear medium",
            Self::LinearHigh => "Linear high",
            Self::AngularLow => "Angular low",
            Self::AngularMedium => "Angular medium",
            Self::AngularHigh => "Angular high",
        }
    }

    const fn damping_values(self) -> (f32, f32) {
        match self {
            Self::LinearLow => (0.0, 0.0),
            Self::LinearMedium => (1.0, 0.0),
            Self::LinearHigh => (5.0, 0.0),
            Self::AngularLow => (0.0, 0.0),
            Self::AngularMedium => (0.0, 1.0),
            Self::AngularHigh => (0.0, 5.0),
        }
    }

    const fn initial_velocities(self) -> (Vec3, Vec3) {
        match self {
            Self::LinearLow | Self::LinearMedium | Self::LinearHigh => {
                (LINEAR_VELOCITY, Vec3::ZERO)
            }
            Self::AngularLow | Self::AngularMedium | Self::AngularHigh => {
                (Vec3::ZERO, ANGULAR_VELOCITY)
            }
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::LinearLow | Self::AngularLow => Color::srgb(0.2, 0.65, 0.95),
            Self::LinearMedium | Self::AngularMedium => Color::srgb(0.95, 0.7, 0.2),
            Self::LinearHigh | Self::AngularHigh => Color::srgb(0.9, 0.3, 0.25),
        }
    }
}

/// Marks a body in station D's damping comparison lanes.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct DampingDemoObject {
    pub kind: DampingDemoKind,
}

/// Owns the damping comparison bodies and their station controls.
pub struct DampingStationPlugin;

impl Plugin for DampingStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (spawn_damping_demo_objects, spawn_damping_station_ui),
        )
        .add_systems(
            Update,
            (queue_damping_station_actions, update_damping_button_colors),
        );
    }
}

fn spawn_damping_demo_objects(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(0.6))));

    for (index, kind) in DampingDemoKind::ALL.into_iter().enumerate() {
        let lane_index = index % 3;
        let lane_z = if index < 3 { -0.85 } else { 0.85 };
        let position = Vec3::new(-7.2 + lane_index as f32 * 1.2, 0.5, lane_z);
        let initial_transform = Transform::from_translation(position);
        let (linear_damping, angular_damping) = kind.damping_values();
        let (linear_velocity, angular_velocity) = kind.initial_velocities();

        let mut object = commands.spawn((
            DampingDemoObject { kind },
            StationObject::new('D', initial_transform)
                .with_velocities(linear_velocity, angular_velocity),
            RigidBody::Dynamic,
            Collider::cuboid(0.3, 0.3, 0.3),
            Friction::ZERO,
            GravityScale(0.0),
            LinearDamping(linear_damping),
            AngularDamping(angular_damping),
            SleepingDisabled,
            LinearVelocity(linear_velocity),
            AngularVelocity(angular_velocity),
            initial_transform,
            Name::new(format!("Damping Demo - {}", kind.label())),
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
struct DampingStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum DampingStationControl {
    Reset,
}

fn spawn_damping_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                bottom: px(16.0),
                width: px(390.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            DampingStationPanel,
            Name::new("Damping Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("D — DAMPING COMPARISON"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Linear and angular damping are varied independently. Select a body to edit either value in the inspector."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for kind in DampingDemoKind::ALL {
                parent.spawn((
                    Text::new(format!(
                        "{}    linear {:>4.1}    angular {:>4.1}",
                        kind.label(),
                        kind.damping_values().0,
                        kind.damping_values().1,
                    )),
                    TextFont::from_font_size(12.0),
                    TextColor(kind.color()),
                ));
            }

            parent.spawn(Node {
                height: px(4.0),
                ..default()
            });
            parent
                .spawn((
                    Button,
                    DampingStationControl::Reset,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Damping station reset"),
                ))
                .with_child((
                    Text::new("reset station D"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_damping_station_actions(
    controls: Query<(&Interaction, &DampingStationControl), Changed<Interaction>>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed && *control == DampingStationControl::Reset {
            resets.write(ResetStation { code: 'D' });
        }
    }
}

fn update_damping_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<DampingStationControl>>,
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

    fn damping_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            DampingStationPlugin,
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

    fn body(app: &mut App, kind: DampingDemoKind) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query::<(Entity, &DampingDemoObject)>();
        bodies
            .iter(world)
            .find_map(|(entity, demo)| (demo.kind == kind).then_some(entity))
            .expect("damping demonstration body")
    }

    fn position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("damping body position")
            .0
    }

    fn linear_velocity(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<LinearVelocity>()
            .expect("damping body linear velocity")
            .0
    }

    fn angular_velocity(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<AngularVelocity>()
            .expect("damping body angular velocity")
            .0
    }

    #[test]
    fn station_spawns_distinct_linear_and_angular_damping_configurations() {
        let mut app = damping_app();
        app.update();

        let world = app.world_mut();
        let mut bodies = world.query::<(
            &DampingDemoObject,
            &RigidBody,
            &LinearDamping,
            &AngularDamping,
            &Friction,
        )>();
        let bodies: Vec<_> = bodies.iter(world).collect();

        assert_eq!(bodies.len(), DampingDemoKind::ALL.len());
        for (demo, rigid_body, linear_damping, angular_damping, friction) in bodies {
            assert_eq!(*rigid_body, RigidBody::Dynamic);
            assert_eq!(
                (linear_damping.0, angular_damping.0),
                demo.kind.damping_values()
            );
            assert_eq!(*friction, Friction::ZERO);
        }
    }

    #[test]
    fn linear_and_angular_damping_are_editable_without_changing_each_other() {
        let mut app = damping_app();
        app.update();
        let body = body(&mut app, DampingDemoKind::LinearMedium);

        app.world_mut()
            .entity_mut(body)
            .insert((LinearDamping(3.0), AngularDamping(0.25)));

        let entity = app.world().entity(body);
        assert_eq!(entity.get::<LinearDamping>(), Some(&LinearDamping(3.0)));
        assert_eq!(entity.get::<AngularDamping>(), Some(&AngularDamping(0.25)));
    }

    #[test]
    fn greater_linear_damping_stops_sliding_bodies_sooner() {
        let mut app = damping_app();
        app.update();
        let low = body(&mut app, DampingDemoKind::LinearLow);
        let medium = body(&mut app, DampingDemoKind::LinearMedium);
        let high = body(&mut app, DampingDemoKind::LinearHigh);

        run_steps(&mut app, 60);

        let low_speed = linear_velocity(&app, low).x;
        let medium_speed = linear_velocity(&app, medium).x;
        let high_speed = linear_velocity(&app, high).x;
        assert!(low_speed > medium_speed + 0.2);
        assert!(medium_speed > high_speed + 0.2);
        let low_displacement = position(&app, low).x + 7.2;
        let medium_displacement = position(&app, medium).x + 6.0;
        let high_displacement = position(&app, high).x + 4.8;
        assert!(low_displacement > medium_displacement);
        assert!(medium_displacement > high_displacement);
    }

    #[test]
    fn greater_angular_damping_stops_spinning_bodies_sooner() {
        let mut app = damping_app();
        app.update();
        let low = body(&mut app, DampingDemoKind::AngularLow);
        let medium = body(&mut app, DampingDemoKind::AngularMedium);
        let high = body(&mut app, DampingDemoKind::AngularHigh);

        run_steps(&mut app, 60);

        let low_speed = angular_velocity(&app, low).y;
        let medium_speed = angular_velocity(&app, medium).y;
        let high_speed = angular_velocity(&app, high).y;
        assert!(low_speed > medium_speed + 0.2);
        assert!(medium_speed > high_speed + 0.2);
        assert!(linear_velocity(&app, low).length() < 0.01);
        assert!(linear_velocity(&app, medium).length() < 0.01);
        assert!(linear_velocity(&app, high).length() < 0.01);
    }

    #[test]
    fn reset_control_restores_damping_body_motion() {
        let mut app = damping_app();
        app.update();
        let body = body(&mut app, DampingDemoKind::LinearHigh);
        run_steps(&mut app, 20);
        assert_ne!(position(&app, body), Vec3::new(-4.8, 0.5, -0.85));

        app.world_mut()
            .resource_mut::<Messages<ResetStation>>()
            .write(ResetStation { code: 'D' });
        app.update();

        assert!(position(&app, body).distance(Vec3::new(-4.8, 0.5, -0.85)) < 0.001);
        assert_eq!(linear_velocity(&app, body), LINEAR_VELOCITY);
        assert_eq!(angular_velocity(&app, body), Vec3::ZERO);
    }
}
