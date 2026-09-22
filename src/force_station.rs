use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

const FORCE_VECTOR: Vec3 = Vec3::new(5.0, 0.0, 0.0);
const IMPULSE_VECTOR: Vec3 = Vec3::new(4.0, 0.0, 0.0);
const TORQUE_VECTOR: Vec3 = Vec3::new(0.0, 3.0, 0.0);
const ANGULAR_IMPULSE_VECTOR: Vec3 = Vec3::new(0.0, 2.5, 0.0);
const ACCELERATION_VECTOR: Vec3 = Vec3::new(2.5, 0.0, 0.0);

/// The five demonstrations hosted by station D.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ForceDemoKind {
    Force,
    Impulse,
    Torque,
    AngularImpulse,
    Acceleration,
}

impl ForceDemoKind {
    const ALL: [Self; 5] = [
        Self::Force,
        Self::Impulse,
        Self::Torque,
        Self::AngularImpulse,
        Self::Acceleration,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Force => "Force",
            Self::Impulse => "Impulse",
            Self::Torque => "Torque",
            Self::AngularImpulse => "Angular impulse",
            Self::Acceleration => "Acceleration",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Force => "persistent · N",
            Self::Impulse => "one shot · N·s",
            Self::Torque => "persistent · N·m",
            Self::AngularImpulse => "one shot · N·m·s",
            Self::Acceleration => "persistent · m/s²",
        }
    }
}

/// Marks one of the identical bodies used to compare force controls.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForceDemoObject {
    pub kind: ForceDemoKind,
}

/// A request emitted by the station controls and applied on the next physics step.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForceStationAction {
    pub kind: ForceDemoKind,
}

/// Owns the force, impulse, torque, angular impulse, and acceleration station.
pub struct ForceStationPlugin;

impl Plugin for ForceStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ForceStationAction>()
            .add_systems(Startup, (spawn_force_demo_objects, spawn_force_station_ui))
            .add_systems(
                Update,
                (
                    queue_force_station_actions,
                    update_force_station_button_colors,
                    reset_force_station,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, apply_force_station_actions);
    }
}

fn spawn_force_demo_objects(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(0.6))));
    let material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.2, 0.65, 0.95)));

    for (index, kind) in ForceDemoKind::ALL.into_iter().enumerate() {
        let local_x = index as f32 * 0.75 - 1.5;
        let initial_transform = Transform::from_xyz(-6.0 + local_x, 0.5, 0.0);
        let mut object = commands.spawn((
            ForceDemoObject { kind },
            StationObject::new('D', initial_transform),
            RigidBody::Dynamic,
            Collider::cuboid(0.3, 0.3, 0.3),
            Mass(1.0),
            LinearDamping(0.0),
            AngularDamping(0.0),
            initial_transform,
            Name::new(format!("Force Demo - {}", kind.label())),
        ));

        if let (Some(mesh), Some(material)) = (mesh.as_ref(), material.as_ref()) {
            object.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
        }
    }
}

#[derive(Component)]
struct ForceStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum ForceStationControl {
    Apply(ForceDemoKind),
    Reset,
}

fn spawn_force_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                bottom: px(16.0),
                width: px(360.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            ForceStationPanel,
            Name::new("Force Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("D — FORCES"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Each button acts on the matching identical body. Persistent controls keep acting until reset."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for kind in ForceDemoKind::ALL {
                parent
                    .spawn(force_control_row())
                    .with_children(|row| {
                        row.spawn((
                            Text::new(kind.label()),
                            TextFont::from_font_size(13.0),
                            TextColor(PANEL_TEXT),
                            Node {
                                width: px(156.0),
                                ..default()
                            },
                        ));
                        spawn_force_control_button(
                            row,
                            match kind {
                                ForceDemoKind::Force
                                | ForceDemoKind::Torque
                                | ForceDemoKind::Acceleration => "apply",
                                ForceDemoKind::Impulse | ForceDemoKind::AngularImpulse => "pulse",
                            },
                            ForceStationControl::Apply(kind),
                        );
                        row.spawn((
                            Text::new(kind.description()),
                            TextFont::from_font_size(11.0),
                            TextColor(PANEL_TEXT),
                        ));
                    });
            }

            parent.spawn(Node {
                height: px(4.0),
                ..default()
            });
            spawn_force_control_button(parent, "reset station", ForceStationControl::Reset);
        });
}

fn force_control_row() -> Node {
    Node {
        width: percent(100.0),
        height: px(26.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_force_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: ForceStationControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(78.0),
                height: px(22.0),
                margin: UiRect::horizontal(px(3.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Force station control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_force_station_actions(
    controls: Query<(&Interaction, &ForceStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<ForceStationAction>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *control {
            ForceStationControl::Apply(kind) => {
                actions.write(ForceStationAction { kind });
            }
            ForceStationControl::Reset => {
                resets.write(ResetStation { code: 'D' });
            }
        }
    }
}

fn update_force_station_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<ForceStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn apply_force_station_actions(
    mut actions: MessageReader<ForceStationAction>,
    mut bodies: Query<(Entity, &ForceDemoObject, Forces)>,
    mut commands: Commands,
) {
    for action in actions.read().copied() {
        for (entity, demo, mut forces) in &mut bodies {
            if demo.kind != action.kind {
                continue;
            }

            match action.kind {
                ForceDemoKind::Force => {
                    commands.entity(entity).insert(ConstantForce(FORCE_VECTOR));
                }
                ForceDemoKind::Impulse => forces.apply_linear_impulse(IMPULSE_VECTOR),
                ForceDemoKind::Torque => {
                    commands
                        .entity(entity)
                        .insert(ConstantTorque(TORQUE_VECTOR));
                }
                ForceDemoKind::AngularImpulse => {
                    forces.apply_angular_impulse(ANGULAR_IMPULSE_VECTOR)
                }
                ForceDemoKind::Acceleration => {
                    commands
                        .entity(entity)
                        .insert(ConstantLinearAcceleration(ACCELERATION_VECTOR));
                }
            }
        }
    }
}

fn reset_force_station(
    mut requests: MessageReader<ResetStation>,
    mut commands: Commands,
    bodies: Query<(Entity, &ForceDemoObject)>,
) {
    if !requests.read().any(|request| request.code == 'D') {
        return;
    }

    for (entity, _) in &bodies {
        commands
            .entity(entity)
            .remove::<(ConstantForce, ConstantTorque, ConstantLinearAcceleration)>();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn force_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            ForceStationPlugin,
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

    fn body(app: &mut App, kind: ForceDemoKind) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query::<(Entity, &ForceDemoObject)>();
        bodies
            .iter(world)
            .find_map(|(entity, demo)| (demo.kind == kind).then_some(entity))
            .expect("force demonstration body")
    }

    fn request(app: &mut App, kind: ForceDemoKind) {
        app.world_mut()
            .resource_mut::<Messages<ForceStationAction>>()
            .write(ForceStationAction { kind });
        run_steps(app, 3);
    }

    fn separate_demo_bodies(app: &mut App) {
        let bodies = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &ForceDemoObject)>();
            query
                .iter(world)
                .enumerate()
                .map(|(index, (entity, _))| (entity, index))
                .collect::<Vec<_>>()
        };
        for (entity, index) in bodies {
            let position = Vec3::new(index as f32 * 10.0, 0.5, 0.0);
            app.world_mut()
                .entity_mut(entity)
                .insert((Position(position), Transform::from_translation(position)));
        }
        app.update();
    }

    fn velocity(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<LinearVelocity>()
            .expect("linear velocity")
            .0
    }

    fn angular_velocity(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<AngularVelocity>()
            .expect("angular velocity")
            .0
    }

    fn position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("position")
            .0
    }

    #[test]
    fn station_spawns_five_identical_dynamic_demo_objects_and_controls() {
        let mut app = force_app();
        app.update();

        let world = app.world_mut();
        let mut bodies = world.query::<(&ForceDemoObject, &RigidBody, &Collider, &Mass)>();
        let demos: Vec<_> = bodies.iter(world).collect();
        assert_eq!(demos.len(), ForceDemoKind::ALL.len());
        assert!(
            demos
                .iter()
                .all(|(_, body, _, mass)| { **body == RigidBody::Dynamic && mass.0 == 1.0 })
        );
        assert!(demos.iter().all(|(_, _, collider, _)| {
            matches!(
                collider.shape().as_typed_shape(),
                avian3d::parry::shape::TypedShape::Cuboid(_)
            )
        }));

        let mut controls = world.query::<&ForceStationControl>();
        assert_eq!(
            controls
                .iter(world)
                .filter(|control| matches!(control, ForceStationControl::Apply(_)))
                .count(),
            ForceDemoKind::ALL.len()
        );
        assert_eq!(
            controls
                .iter(world)
                .filter(|control| matches!(control, ForceStationControl::Reset))
                .count(),
            1
        );
    }

    #[test]
    fn persistent_force_keeps_accelerating_after_one_shot_impulse_stops() {
        let mut app = force_app();
        app.update();
        separate_demo_bodies(&mut app);
        let force_body = body(&mut app, ForceDemoKind::Force);
        let impulse_body = body(&mut app, ForceDemoKind::Impulse);

        request(&mut app, ForceDemoKind::Force);
        request(&mut app, ForceDemoKind::Impulse);
        let force_after_action = velocity(&app, force_body).x;
        let impulse_after_action = velocity(&app, impulse_body).x;
        run_steps(&mut app, 10);

        let force_later = velocity(&app, force_body).x;
        let impulse_later = velocity(&app, impulse_body).x;
        assert!(force_later > force_after_action + 0.5);
        assert!(
            (impulse_later - impulse_after_action).abs() < 0.05,
            "impulse changed after the one-shot action: {impulse_after_action} -> {impulse_later}"
        );
        assert!(app.world().entity(force_body).contains::<ConstantForce>());
        assert!(!app.world().entity(impulse_body).contains::<ConstantForce>());
    }

    #[test]
    fn acceleration_is_persistent_and_changes_linear_motion_without_mass_dependence() {
        let mut app = force_app();
        app.update();
        let acceleration_body = body(&mut app, ForceDemoKind::Acceleration);
        request(&mut app, ForceDemoKind::Acceleration);
        let initial_velocity = velocity(&app, acceleration_body);
        run_steps(&mut app, 30);
        let later_velocity = velocity(&app, acceleration_body);

        assert!(later_velocity.x > initial_velocity.x + 0.5);
        assert!(later_velocity.y.abs() < 0.01);
        assert!(
            app.world()
                .entity(acceleration_body)
                .contains::<ConstantLinearAcceleration>()
        );
    }

    #[test]
    fn torque_and_angular_impulse_change_rotation_without_translating_the_bodies() {
        let mut app = force_app();
        app.update();
        let torque_body = body(&mut app, ForceDemoKind::Torque);
        let angular_impulse_body = body(&mut app, ForceDemoKind::AngularImpulse);
        let torque_start = position(&app, torque_body);
        let impulse_start = position(&app, angular_impulse_body);

        request(&mut app, ForceDemoKind::Torque);
        request(&mut app, ForceDemoKind::AngularImpulse);
        run_steps(&mut app, 30);

        assert!(angular_velocity(&app, torque_body).y.abs() > 0.1);
        assert!(angular_velocity(&app, angular_impulse_body).y.abs() > 0.1);
        assert!(position(&app, torque_body).distance(torque_start) < 0.01);
        assert!(position(&app, angular_impulse_body).distance(impulse_start) < 0.01);
        assert!(app.world().entity(torque_body).contains::<ConstantTorque>());
        assert!(
            !app.world()
                .entity(angular_impulse_body)
                .contains::<ConstantTorque>()
        );
    }

    #[test]
    fn resetting_station_d_clears_persistent_effects() {
        let mut app = force_app();
        app.update();
        let force_body = body(&mut app, ForceDemoKind::Force);
        let torque_body = body(&mut app, ForceDemoKind::Torque);
        let acceleration_body = body(&mut app, ForceDemoKind::Acceleration);
        request(&mut app, ForceDemoKind::Force);
        request(&mut app, ForceDemoKind::Torque);
        request(&mut app, ForceDemoKind::Acceleration);

        app.world_mut()
            .resource_mut::<Messages<ResetStation>>()
            .write(ResetStation { code: 'D' });
        app.update();

        assert!(!app.world().entity(force_body).contains::<ConstantForce>());
        assert!(!app.world().entity(torque_body).contains::<ConstantTorque>());
        assert!(
            !app.world()
                .entity(acceleration_body)
                .contains::<ConstantLinearAcceleration>()
        );
    }
}
