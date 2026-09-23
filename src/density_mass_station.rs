use avian3d::prelude::*;
use bevy::prelude::*;

use crate::player::{
    PLAYER_CAPSULE_LENGTH, PLAYER_RADIUS, Player, PlayerInput, PlayerMovementSet,
    PlayerMovementSettings,
};
use crate::stations::StationObject;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

const OBJECT_SIZE: f32 = 0.7;
const OBJECT_HALF_SIZE: f32 = OBJECT_SIZE * 0.5;
const LIGHT_DENSITY: f32 = 0.5;
const MEDIUM_DENSITY: f32 = 3.0;
const HEAVY_MASS: f32 = 12.0;
const VALUE_STEP: f32 = 0.25;
const PLAYER_PUSH_FORCE: f32 = 3.0;

/// The three visually identical bodies compared by station C.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DensityMassLabKind {
    Light,
    Medium,
    Heavy,
}

impl DensityMassLabKind {
    pub const ALL: [Self; 3] = [Self::Light, Self::Medium, Self::Heavy];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Light => "LIGHT",
            Self::Medium => "MEDIUM",
            Self::Heavy => "HEAVY",
        }
    }

    pub const fn color(self) -> Color {
        match self {
            Self::Light => Color::srgb(0.35, 0.82, 1.0),
            Self::Medium => Color::srgb(0.98, 0.72, 0.22),
            Self::Heavy => Color::srgb(0.95, 0.3, 0.25),
        }
    }

    const fn lane_x(self) -> f32 {
        match self {
            Self::Light => 4.8,
            Self::Medium => 6.0,
            Self::Heavy => 7.2,
        }
    }

    const fn initial_transform(self) -> Transform {
        Transform::from_xyz(self.lane_x(), OBJECT_HALF_SIZE + 0.12, -6.0)
    }

    const fn uses_density(self) -> bool {
        !matches!(self, Self::Heavy)
    }

    const fn initial_density(self) -> f32 {
        match self {
            Self::Light => LIGHT_DENSITY,
            Self::Medium => MEDIUM_DENSITY,
            Self::Heavy => 1.0,
        }
    }

    const fn initial_mass(self) -> f32 {
        match self {
            Self::Heavy => HEAVY_MASS,
            Self::Light | Self::Medium => 0.0,
        }
    }
}

/// Marks a body in station C's density and mass comparison.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct DensityMassLabObject {
    pub kind: DensityMassLabKind,
}

/// Requests a station C value adjustment or reset.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum DensityMassLabAction {
    Adjust {
        kind: DensityMassLabKind,
        delta: f32,
    },
    Reset,
}

/// Owns station C's identical-looking light, medium, and heavy bodies.
pub struct DensityMassStationPlugin;

impl Plugin for DensityMassStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DensityMassLabAction>()
            .add_systems(
                Startup,
                (spawn_density_mass_lab, spawn_density_mass_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_density_mass_station_actions,
                    apply_density_mass_station_actions,
                    update_density_mass_button_colors,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                draw_density_mass_labels.run_if(resource_exists::<GizmoConfigStore>),
            )
            .add_systems(
                FixedUpdate,
                apply_density_mass_player_push.after(PlayerMovementSet),
            );
    }
}

fn spawn_density_mass_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(OBJECT_SIZE))));
    // One mesh and one material are deliberately shared: density and mass must
    // be the only visible difference between the three bodies.
    let material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.34, 0.58, 0.82)));

    for kind in DensityMassLabKind::ALL {
        let initial_transform = kind.initial_transform();
        let mut object = commands.spawn((
            DensityMassLabObject { kind },
            StationObject::new('C', initial_transform),
            RigidBody::Dynamic,
            Collider::cuboid(OBJECT_SIZE, OBJECT_SIZE, OBJECT_SIZE),
            Friction::new(0.2),
            Restitution::ZERO,
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            SleepingDisabled,
            initial_transform,
            Name::new(format!("Density and Mass Lab - {}", kind.label())),
        ));

        match kind {
            DensityMassLabKind::Light | DensityMassLabKind::Medium => {
                object.insert(ColliderDensity(kind.initial_density()));
            }
            DensityMassLabKind::Heavy => {
                object.insert(Mass(kind.initial_mass()));
            }
        }

        if let (Some(mesh), Some(material)) = (mesh.as_ref(), material.as_ref()) {
            object.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
        }
    }
}

#[derive(Component)]
struct DensityMassStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum DensityMassStationControl {
    Adjust {
        kind: DensityMassLabKind,
        direction: StepDirection,
    },
    Reset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StepDirection {
    Down,
    Up,
}

fn spawn_density_mass_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(272.0),
                width: px(390.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            DensityMassStationPanel,
            Name::new("Density and Mass Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("C — DENSITY & MASS LABORATORY"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Identical cubes use different density or explicit mass. Push and grab each one to compare acceleration and lag.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for kind in DensityMassLabKind::ALL {
                parent
                    .spawn(Node {
                        width: percent(100.0),
                        height: px(26.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(format!(
                                "{}  {} {:.2}",
                                kind.label(),
                                if kind.uses_density() {
                                    "density"
                                } else {
                                    "mass   "
                                },
                                if kind.uses_density() {
                                    kind.initial_density()
                                } else {
                                    kind.initial_mass()
                                }
                            )),
                            TextFont::from_font_size(12.0),
                            TextColor(kind.color()),
                            Node {
                                width: px(220.0),
                                ..default()
                            },
                        ));
                        spawn_density_mass_button(
                            row,
                            "-",
                            DensityMassStationControl::Adjust {
                                kind,
                                direction: StepDirection::Down,
                            },
                        );
                        spawn_density_mass_button(
                            row,
                            "+",
                            DensityMassStationControl::Adjust {
                                kind,
                                direction: StepDirection::Up,
                            },
                        );
                    });
            }

            parent.spawn(Node {
                height: px(4.0),
                ..default()
            });
            spawn_density_mass_button(parent, "reset station C", DensityMassStationControl::Reset);
        });
}

fn spawn_density_mass_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: DensityMassStationControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(34.0),
                height: px(22.0),
                margin: UiRect::horizontal(px(3.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Density and mass control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_density_mass_station_actions(
    controls: Query<(&Interaction, &DensityMassStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<DensityMassLabAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *control {
            DensityMassStationControl::Adjust { kind, direction } => {
                actions.write(DensityMassLabAction::Adjust {
                    kind,
                    delta: match direction {
                        StepDirection::Down => -VALUE_STEP,
                        StepDirection::Up => VALUE_STEP,
                    },
                });
            }
            DensityMassStationControl::Reset => {
                actions.write(DensityMassLabAction::Reset);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_density_mass_station_actions(
    mut actions: MessageReader<DensityMassLabAction>,
    mut commands: Commands,
    mut objects: Query<(
        Entity,
        &DensityMassLabObject,
        &StationObject,
        &mut Transform,
        Option<&mut Position>,
        Option<&mut Rotation>,
        Option<&mut LinearVelocity>,
        Option<&mut AngularVelocity>,
        Option<&mut ColliderDensity>,
        Option<&mut Mass>,
    )>,
) {
    for action in actions.read().copied() {
        match action {
            DensityMassLabAction::Adjust { kind, delta } => {
                let Some((entity, _, _, _, _, _, _, _, mut density, mut mass)) = objects
                    .iter_mut()
                    .find(|(_, object, ..)| object.kind == kind)
                else {
                    continue;
                };

                if kind.uses_density() {
                    if let Some(density) = density.as_deref_mut() {
                        density.0 = (density.0 + delta).max(0.05);
                    } else {
                        commands
                            .entity(entity)
                            .insert(ColliderDensity((kind.initial_density() + delta).max(0.05)));
                    }
                    commands.entity(entity).remove::<Mass>();
                } else if let Some(mass) = mass.as_deref_mut() {
                    mass.0 = (mass.0 + delta).max(0.05);
                } else {
                    commands
                        .entity(entity)
                        .insert(Mass((HEAVY_MASS + delta).max(0.05)));
                }
            }
            DensityMassLabAction::Reset => {
                for (
                    entity,
                    object,
                    initial,
                    mut transform,
                    mut position,
                    mut rotation,
                    mut linear_velocity,
                    mut angular_velocity,
                    mut density,
                    mut mass,
                ) in &mut objects
                {
                    *transform = initial.initial_transform;
                    if let Some(position) = position.as_deref_mut() {
                        position.0 = initial.initial_transform.translation;
                    }
                    if let Some(rotation) = rotation.as_deref_mut() {
                        rotation.0 = initial.initial_transform.rotation;
                    }
                    if let Some(velocity) = linear_velocity.as_deref_mut() {
                        velocity.0 = initial.initial_linear_velocity;
                    }
                    if let Some(velocity) = angular_velocity.as_deref_mut() {
                        velocity.0 = initial.initial_angular_velocity;
                    }

                    if object.kind.uses_density() {
                        if let Some(density) = density.as_deref_mut() {
                            density.0 = object.kind.initial_density();
                        } else {
                            commands
                                .entity(entity)
                                .insert(ColliderDensity(object.kind.initial_density()));
                        }
                        commands.entity(entity).remove::<Mass>();
                    } else if let Some(mass) = mass.as_deref_mut() {
                        mass.0 = object.kind.initial_mass();
                    } else {
                        commands
                            .entity(entity)
                            .insert(Mass(object.kind.initial_mass()));
                    }
                }
            }
        }
    }
}

fn update_density_mass_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<DensityMassStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn effective_mass(computed_mass: Option<&ComputedMass>, explicit_mass: Option<&Mass>) -> f32 {
    computed_mass
        .map(|mass| mass.value())
        .filter(|mass| mass.is_finite() && *mass > 0.0)
        .or_else(|| explicit_mass.map(|mass| mass.0))
        .filter(|mass| mass.is_finite() && *mass > 0.0)
        .unwrap_or(1.0)
}

#[allow(clippy::type_complexity)]
fn apply_density_mass_player_push(
    settings: Option<Res<PlayerMovementSettings>>,
    time: Res<Time>,
    players: Query<(&Transform, &PlayerInput), With<Player>>,
    mut objects: Query<
        (
            &Transform,
            &mut LinearVelocity,
            Option<&ComputedMass>,
            Option<&Mass>,
        ),
        With<DensityMassLabObject>,
    >,
) {
    let push_force = settings
        .map(|settings| PLAYER_PUSH_FORCE * (settings.speed / 5.0).max(0.1))
        .unwrap_or(PLAYER_PUSH_FORCE);
    let delta_secs = time.delta_secs();

    for (player_transform, input) in &players {
        let push_direction = Vec3::new(input.0.x, 0.0, -input.0.y).normalize_or_zero();
        if push_direction == Vec3::ZERO {
            continue;
        }

        for (object_transform, mut velocity, computed_mass, explicit_mass) in &mut objects {
            let offset = object_transform.translation - player_transform.translation;
            let horizontal_offset = Vec2::new(offset.x, offset.z);
            let vertical_overlap =
                offset.y.abs() < PLAYER_CAPSULE_LENGTH * 0.5 + PLAYER_RADIUS + OBJECT_HALF_SIZE;
            if !vertical_overlap
                || horizontal_offset.length() > PLAYER_RADIUS + OBJECT_HALF_SIZE + 0.08
            {
                continue;
            }

            let mass = effective_mass(computed_mass, explicit_mass);
            velocity.0 += push_direction * (push_force * delta_secs / mass);
        }
    }
}

#[allow(clippy::type_complexity)]
fn draw_density_mass_labels(
    mut gizmos: Gizmos,
    objects: Query<(
        &DensityMassLabObject,
        &Transform,
        Option<&ComputedMass>,
        Option<&Mass>,
        Option<&ColliderDensity>,
    )>,
) {
    for (object, transform, computed_mass, explicit_mass, density) in &objects {
        let value = if let Some(density) = density.filter(|_| object.kind.uses_density()) {
            format!("density {:.2}", density.0)
        } else if let Some(mass) = explicit_mass {
            format!("mass {:.2}", mass.0)
        } else {
            "mass pending".to_owned()
        };
        let computed = computed_mass
            .map(|mass| format!("\ncomputed {:.2}", mass.value()))
            .unwrap_or_default();
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.55, Quat::IDENTITY),
            &format!("{}\n{}{}", object.kind.label(), value, computed),
            0.34,
            Vec2::ZERO,
            object.kind.color(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::{ButtonState, InputPlugin, mouse::MouseButtonInput},
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::{
        cursor_hover::CursorHoverPlugin, object_grabbing::ObjectGrabbingPlugin,
        stations::StationLayoutPlugin,
    };

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn lab_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            DensityMassStationPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn interactive_lab_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            CursorHoverPlugin,
            ObjectGrabbingPlugin,
            DensityMassStationPlugin,
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

    fn body(app: &mut App, kind: DensityMassLabKind) -> Entity {
        let world = app.world_mut();
        let mut objects = world.query::<(Entity, &DensityMassLabObject)>();
        objects
            .iter(world)
            .find_map(|(entity, object)| (object.kind == kind).then_some(entity))
            .expect("density/mass lab body")
    }

    fn position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("density/mass body position")
            .0
    }

    fn same_material_and_shape(app: &mut App) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
        let world = app.world_mut();
        let mut objects = world.query::<(
            &DensityMassLabObject,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        )>();
        objects
            .iter(world)
            .map(|(_, mesh, material)| (mesh.0.clone(), material.0.clone()))
            .collect()
    }

    #[test]
    fn station_spawns_visually_identical_dynamic_objects_with_distinct_mass_sources() {
        let mut app = lab_app();
        app.update();
        // Startup commands are applied at the end of the first frame; give Avian
        // one more frame to recompute mass properties from the configured values.
        app.update();

        let objects = same_material_and_shape(&mut app);
        assert_eq!(objects.len(), DensityMassLabKind::ALL.len());
        assert!(
            objects
                .windows(2)
                .all(|pair| pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1)
        );

        let world = app.world_mut();
        let mut colliders = world.query::<(&DensityMassLabObject, &Collider)>();
        assert!(colliders.iter(world).all(|(_, collider)| {
            matches!(
                collider.shape().as_typed_shape(),
                avian3d::parry::shape::TypedShape::Cuboid(_)
            )
        }));
        let mut query = world.query::<(
            &DensityMassLabObject,
            &RigidBody,
            Option<&ColliderDensity>,
            Option<&Mass>,
            &ComputedMass,
        )>();
        let mut masses = Vec::new();
        for (object, rigid_body, density, mass, computed_mass) in query.iter(world) {
            assert_eq!(*rigid_body, RigidBody::Dynamic);
            match object.kind {
                DensityMassLabKind::Light => {
                    assert_eq!(density.map(|density| density.0), Some(LIGHT_DENSITY));
                    assert!(mass.is_none());
                }
                DensityMassLabKind::Medium => {
                    assert_eq!(density.map(|density| density.0), Some(MEDIUM_DENSITY));
                    assert!(mass.is_none());
                }
                DensityMassLabKind::Heavy => {
                    assert_eq!(mass.map(|mass| mass.0), Some(HEAVY_MASS));
                }
            }
            masses.push((object.kind, computed_mass.value()));
        }
        masses.sort_by_key(|(kind, _)| *kind as u8);
        assert!(
            masses[0].1 < masses[1].1 && masses[1].1 < masses[2].1,
            "computed masses were not ordered: {masses:?}"
        );
    }

    #[test]
    fn density_and_mass_controls_change_the_live_physics_properties() {
        let mut app = lab_app();
        app.update();

        app.world_mut()
            .resource_mut::<Messages<DensityMassLabAction>>()
            .write(DensityMassLabAction::Adjust {
                kind: DensityMassLabKind::Light,
                delta: VALUE_STEP,
            });
        app.world_mut()
            .resource_mut::<Messages<DensityMassLabAction>>()
            .write(DensityMassLabAction::Adjust {
                kind: DensityMassLabKind::Heavy,
                delta: VALUE_STEP,
            });
        app.update();

        let light = body(&mut app, DensityMassLabKind::Light);
        let heavy = body(&mut app, DensityMassLabKind::Heavy);
        assert_eq!(
            app.world().entity(light).get::<ColliderDensity>(),
            Some(&ColliderDensity(LIGHT_DENSITY + VALUE_STEP))
        );
        assert_eq!(
            app.world().entity(heavy).get::<Mass>(),
            Some(&Mass(HEAVY_MASS + VALUE_STEP))
        );

        app.world_mut()
            .resource_mut::<Messages<DensityMassLabAction>>()
            .write(DensityMassLabAction::Reset);
        app.update();
        assert_eq!(
            app.world().entity(light).get::<ColliderDensity>(),
            Some(&ColliderDensity(LIGHT_DENSITY))
        );
        assert_eq!(
            app.world().entity(heavy).get::<Mass>(),
            Some(&Mass(HEAVY_MASS))
        );
    }

    #[test]
    fn player_push_acceleration_decreases_from_light_to_heavy() {
        let mut app = lab_app();
        app.insert_resource(PlayerMovementSettings::default());
        app.update();
        let player = app
            .world_mut()
            .spawn((
                Player,
                PlayerInput(Vec2::X),
                Position(Vec3::ZERO),
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();
        let mut velocities = Vec::new();

        for kind in DensityMassLabKind::ALL {
            let target = body(&mut app, kind);
            let target_position = Vec3::new(0.75, OBJECT_HALF_SIZE + 0.12, 0.0);
            app.world_mut().entity_mut(target).insert((
                Position(target_position),
                Transform::from_translation(target_position),
                LinearVelocity::ZERO,
            ));
            app.update();
            velocities.push((
                kind,
                app.world()
                    .entity(target)
                    .get::<LinearVelocity>()
                    .unwrap()
                    .0
                    .x,
            ));
            app.world_mut().entity_mut(target).insert((
                Position(kind.initial_transform().translation),
                Transform::from_translation(kind.initial_transform().translation),
                LinearVelocity::ZERO,
            ));
        }

        assert!(velocities[0].1 > velocities[1].1 && velocities[1].1 > velocities[2].1);
        app.world_mut().entity_mut(player).despawn();
    }

    fn point_cursor_at(app: &mut App, target: Vec3) {
        app.world_mut()
            .resource_mut::<crate::cursor_hover::CursorRay>()
            .set(
                Vec3::ZERO,
                Dir3::new(target.normalize()).expect("grab ray direction"),
            );
        app.update();
    }

    fn send_left_button(app: &mut App, state: ButtonState) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(MouseButtonInput {
                button: MouseButton::Left,
                state,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    #[test]
    fn spring_grab_follows_light_body_farther_than_heavy_body() {
        let mut distances = Vec::new();
        for kind in [DensityMassLabKind::Light, DensityMassLabKind::Heavy] {
            let mut app = interactive_lab_app();
            app.update();
            let target = body(&mut app, kind);
            let start = position(&app, target);
            point_cursor_at(&mut app, start);
            send_left_button(&mut app, ButtonState::Pressed);
            assert_eq!(
                app.world()
                    .resource::<crate::object_grabbing::GrabState>()
                    .grab
                    .map(|grab| grab.entity),
                Some(target)
            );

            let moved_target = start + Vec3::X;
            app.world_mut()
                .resource_mut::<crate::cursor_hover::CursorRay>()
                .set(
                    Vec3::ZERO,
                    Dir3::new(moved_target.normalize()).expect("moved grab ray direction"),
                );
            run_steps(&mut app, 30);
            distances.push((kind, position(&app, target).x - start.x));
        }

        assert!(
            distances[0].1 > distances[1].1 + 0.1,
            "light and heavy grab distances were too similar: {distances:?}"
        );
    }
}
