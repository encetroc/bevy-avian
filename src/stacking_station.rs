use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const SLEEPING_COLOR: Color = Color::srgb(0.45, 0.78, 1.0);
const AWAKE_COLOR: Color = Color::srgb(1.0, 0.72, 0.32);
const STACK_Z: f32 = 8.15;
const OBJECT_DENSITY: f32 = 0.75;

/// The reusable object shapes in the free construction area.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StackingObjectKind {
    Cube,
    Brick,
    Plank,
    Cylinder,
    Sphere,
}

impl StackingObjectKind {
    /// Every shape supplied by the stacking laboratory.
    pub const ALL: [Self; 5] = [
        Self::Cube,
        Self::Brick,
        Self::Plank,
        Self::Cylinder,
        Self::Sphere,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Cube => "CUBE",
            Self::Brick => "BRICK",
            Self::Plank => "PLANK",
            Self::Cylinder => "CYLINDER",
            Self::Sphere => "SPHERE",
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Cube => Color::srgb(0.2, 0.65, 0.95),
            Self::Brick => Color::srgb(0.88, 0.3, 0.2),
            Self::Plank => Color::srgb(0.78, 0.48, 0.2),
            Self::Cylinder => Color::srgb(0.25, 0.78, 0.45),
            Self::Sphere => Color::srgb(0.72, 0.4, 0.95),
        }
    }

    const fn size(self) -> Vec3 {
        match self {
            Self::Cube => Vec3::splat(0.7),
            Self::Brick => Vec3::new(1.15, 0.35, 0.6),
            Self::Plank => Vec3::new(1.65, 0.24, 0.45),
            Self::Cylinder => Vec3::new(0.68, 0.72, 0.68),
            Self::Sphere => Vec3::splat(0.72),
        }
    }

    const fn half_height(self) -> f32 {
        match self {
            Self::Cube => 0.35,
            Self::Brick => 0.175,
            Self::Plank => 0.12,
            Self::Cylinder => 0.36,
            Self::Sphere => 0.36,
        }
    }

    fn collider(self) -> Collider {
        match self {
            Self::Cube => Collider::cuboid(0.7, 0.7, 0.7),
            Self::Brick => Collider::cuboid(1.15, 0.35, 0.6),
            Self::Plank => Collider::cuboid(1.65, 0.24, 0.45),
            Self::Cylinder => Collider::cylinder(0.34, 0.72),
            Self::Sphere => Collider::sphere(0.36),
        }
    }

    fn mesh(self) -> Mesh {
        match self {
            Self::Cube => Cuboid::from_size(self.size()).into(),
            Self::Brick => Cuboid::from_size(self.size()).into(),
            Self::Plank => Cuboid::from_size(self.size()).into(),
            Self::Cylinder => Cylinder::new(0.34, 0.72).into(),
            Self::Sphere => Sphere::new(0.36).into(),
        }
    }
}

/// Marks an object that can be freely grabbed, stacked, and disturbed in J.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackingLabObject {
    pub kind: StackingObjectKind,
}

#[derive(Clone, Copy)]
struct ObjectDefinition {
    kind: StackingObjectKind,
    position: Vec3,
    rotation_y: f32,
}

impl ObjectDefinition {
    const fn new(kind: StackingObjectKind, position: Vec3, rotation_y: f32) -> Self {
        Self {
            kind,
            position,
            rotation_y,
        }
    }

    fn transform(self) -> Transform {
        Transform::from_translation(self.position)
            .with_rotation(Quat::from_rotation_y(self.rotation_y))
    }
}

// Five separate three-piece stacks leave open floor around the objects for
// grabbing and rearranging. The alternating plank orientations and the round
// bodies make tipping, sliding, and jitter visible without pre-solving the
// experiment for the player.
const OBJECT_DEFINITIONS: [ObjectDefinition; 15] = [
    ObjectDefinition::new(
        StackingObjectKind::Cube,
        Vec3::new(-3.0, 0.35, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Cube,
        Vec3::new(-3.0, 1.05, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Cube,
        Vec3::new(-3.0, 1.75, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Brick,
        Vec3::new(-1.5, 0.175, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Brick,
        Vec3::new(-1.5, 0.525, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Brick,
        Vec3::new(-1.5, 0.875, STACK_Z),
        std::f32::consts::FRAC_PI_2,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Plank,
        Vec3::new(0.0, 0.12, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Plank,
        Vec3::new(0.0, 0.36, STACK_Z),
        std::f32::consts::FRAC_PI_2,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Plank,
        Vec3::new(0.0, 0.60, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Cylinder,
        Vec3::new(1.7, 0.36, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Cylinder,
        Vec3::new(1.7, 1.08, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Cylinder,
        Vec3::new(1.7, 1.80, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Sphere,
        Vec3::new(3.15, 0.36, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Sphere,
        Vec3::new(3.15, 1.08, STACK_Z),
        0.0,
    ),
    ObjectDefinition::new(
        StackingObjectKind::Sphere,
        Vec3::new(3.15, 1.80, STACK_Z),
        0.0,
    ),
];

/// Owns station J's open construction area and its reset control.
pub struct StackingStationPlugin;

impl Plugin for StackingStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_message::<SleepingControlAction>()
            .init_resource::<SleepingControlsState>()
            .add_systems(Startup, (spawn_stacking_lab, spawn_stacking_station_ui))
            .add_systems(
                Update,
                (
                    queue_stacking_station_actions,
                    apply_sleeping_controls,
                    update_sleeping_status,
                    update_stacking_button_colors,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                draw_stacking_labels.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_stacking_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for definition in OBJECT_DEFINITIONS {
        let transform = definition.transform();
        let mesh = meshes
            .as_mut()
            .map(|meshes| meshes.add(definition.kind.mesh()));
        let material = materials
            .as_mut()
            .map(|materials| materials.add(definition.kind.color()));

        let mut object = commands.spawn((
            StackingLabObject {
                kind: definition.kind,
            },
            StationObject::new('J', transform),
            RigidBody::Dynamic,
            definition.kind.collider(),
            ColliderDensity(OBJECT_DENSITY),
            Friction::new(0.55),
            Restitution::new(0.05),
            LinearDamping(0.12),
            AngularDamping(0.2),
            transform,
            Name::new(format!("Stacking Lab - {}", definition.kind.label())),
        ));

        if let (Some(mesh), Some(material)) = (mesh, material) {
            object.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    }
}

#[derive(Component)]
struct StackingStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum StackingStationControl {
    Reset,
    SleepAll,
    WakeAll,
    ToggleSleeping,
}

#[derive(Component)]
struct SleepingStatusText;

#[derive(Component)]
struct SleepingDisabledByStackingControls;

#[derive(Resource)]
struct SleepingControlsState {
    enabled: bool,
}

impl Default for SleepingControlsState {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
enum SleepingControlAction {
    SleepAll,
    WakeAll,
    ToggleSleeping,
}

fn spawn_stacking_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(16.0),
                width: px(390.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            StackingStationPanel,
            Name::new("Stacking Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("J — STACKING LABORATORY"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Grab any object with the left mouse button. Build a stack, tip it over, and watch sliding bodies settle and sleep.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));
            for kind in StackingObjectKind::ALL {
                parent.spawn((
                    Text::new(format!("3 × {}", kind.label())),
                    TextFont::from_font_size(13.0),
                    TextColor(kind.color()),
                ));
            }
            parent.spawn((
                Text::new("F1 shows Avian colliders; colored labels identify each body's sleep state."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("Awake: 0   Sleeping: 0"),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                SleepingStatusText,
                Name::new("Stacking sleep state counts"),
            ));
            parent
                .spawn(Node {
                    width: percent(100.0),
                    height: px(26.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for (label, control) in [
                        ("WAKE ALL", StackingStationControl::WakeAll),
                        ("SLEEP ALL", StackingStationControl::SleepAll),
                        ("TOGGLE SLEEP", StackingStationControl::ToggleSleeping),
                    ] {
                        spawn_stacking_control(row, label, control);
                    }
                });
            parent
                .spawn((
                    Button,
                    StackingStationControl::Reset,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Stacking station reset"),
                ))
                .with_child((
                    Text::new("reset station J"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn spawn_stacking_control(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: StackingStationControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(112.0),
                height: px(22.0),
                margin: UiRect::right(px(3.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Stacking control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(10.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_stacking_station_actions(
    controls: Query<(&Interaction, &StackingStationControl), Changed<Interaction>>,
    mut resets: MessageWriter<ResetStation>,
    mut sleeping_actions: MessageWriter<SleepingControlAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match control {
            StackingStationControl::Reset => {
                resets.write(ResetStation { code: 'J' });
            }
            StackingStationControl::SleepAll => {
                sleeping_actions.write(SleepingControlAction::SleepAll);
            }
            StackingStationControl::WakeAll => {
                sleeping_actions.write(SleepingControlAction::WakeAll);
            }
            StackingStationControl::ToggleSleeping => {
                sleeping_actions.write(SleepingControlAction::ToggleSleeping);
            }
        }
    }
}

type SleepingControlBodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static RigidBody,
        Has<SleepingDisabled>,
        Has<SleepingDisabledByStackingControls>,
    ),
    With<StackingLabObject>,
>;

fn apply_sleeping_controls(
    mut actions: MessageReader<SleepingControlAction>,
    mut state: ResMut<SleepingControlsState>,
    bodies: SleepingControlBodies<'_, '_>,
    mut commands: Commands,
) {
    for action in actions.read().copied() {
        match action {
            SleepingControlAction::SleepAll if state.enabled => {
                for (entity, body, sleeping_disabled, _) in &bodies {
                    if body.is_dynamic() && !sleeping_disabled {
                        commands.entity(entity).insert(Sleeping);
                    }
                }
            }
            SleepingControlAction::WakeAll => {
                for (entity, body, sleeping_disabled, _) in &bodies {
                    if body.is_dynamic() && !sleeping_disabled {
                        commands.entity(entity).remove::<Sleeping>();
                    }
                }
            }
            SleepingControlAction::ToggleSleeping => {
                state.enabled = !state.enabled;
                for (entity, body, sleeping_disabled, disabled_by_controls) in &bodies {
                    if !body.is_dynamic() {
                        continue;
                    }
                    if state.enabled && disabled_by_controls {
                        commands
                            .entity(entity)
                            .remove::<(SleepingDisabled, SleepingDisabledByStackingControls)>();
                    } else if !state.enabled && !sleeping_disabled {
                        commands
                            .entity(entity)
                            .remove::<Sleeping>()
                            .insert((SleepingDisabled, SleepingDisabledByStackingControls));
                    }
                }
            }
            SleepingControlAction::SleepAll => {}
        }
    }
}

fn update_sleeping_status(
    state: Res<SleepingControlsState>,
    bodies: Query<(&RigidBody, Has<Sleeping>, Has<SleepingDisabled>), With<StackingLabObject>>,
    mut status: Query<&mut Text, With<SleepingStatusText>>,
) {
    let mut awake = 0;
    let mut sleeping = 0;
    for (body, is_sleeping, _) in &bodies {
        if body.is_dynamic() {
            if is_sleeping {
                sleeping += 1;
            } else {
                awake += 1;
            }
        }
    }
    for mut text in &mut status {
        *text = Text::new(format!(
            "Awake: {awake}   Sleeping: {sleeping}   Sleeping {}",
            if state.enabled { "ON" } else { "OFF" }
        ));
    }
}

fn update_stacking_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<StackingStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn draw_stacking_labels(
    mut gizmos: Gizmos,
    objects: Query<(&StackingLabObject, &Transform, Has<Sleeping>)>,
) {
    for (object, transform, is_sleeping) in &objects {
        let (label, color) = if is_sleeping {
            ("SLEEPING", SLEEPING_COLOR)
        } else {
            ("AWAKE", AWAKE_COLOR)
        };
        let label = format!("{} · {label}", object.kind.label());
        gizmos.text(
            Isometry3d::new(
                transform.translation + Vec3::Y * (object.kind.half_height() + 0.2),
                Quat::IDENTITY,
            ),
            &label,
            0.22,
            Vec2::ZERO,
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    #[derive(Component)]
    struct TestFloor;

    fn stacking_app(with_layout: bool) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StackingStationPlugin,
        ));
        if with_layout {
            app.add_plugins(StationLayoutPlugin);
        }
        app.insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                PHYSICS_STEP,
            )))
            .add_systems(Startup, spawn_test_floor);
        app.finish();
        app
    }

    fn spawn_test_floor(mut commands: Commands) {
        commands.spawn((
            TestFloor,
            RigidBody::Static,
            Collider::cuboid(22.0, 0.5, 22.0),
            Transform::from_xyz(0.0, -0.25, 8.0),
        ));
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn objects(app: &mut App) -> HashMap<StackingObjectKind, Vec<Entity>> {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &StackingLabObject)>();
        let mut objects = HashMap::new();
        for (entity, object) in query.iter(world) {
            objects
                .entry(object.kind)
                .or_insert_with(Vec::new)
                .push(entity);
        }
        objects
    }

    #[test]
    fn station_supplies_three_named_dynamic_objects_of_each_stackable_kind() {
        let mut app = stacking_app(false);
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(
            &StackingLabObject,
            &StationObject,
            &RigidBody,
            &Collider,
            &Mesh3d,
            &Name,
        )>();
        let objects: Vec<_> = query.iter(world).collect();
        assert_eq!(objects.len(), OBJECT_DEFINITIONS.len());

        for kind in StackingObjectKind::ALL {
            let matching: Vec<_> = objects
                .iter()
                .filter(|(object, _, _, _, _, _)| object.kind == kind)
                .collect();
            assert_eq!(matching.len(), 3, "missing {} objects", kind.label());
            for (object, station, body, _, _, name) in matching {
                assert_eq!(object.kind, kind);
                assert_eq!(station.station, 'J');
                assert_eq!(**body, RigidBody::Dynamic);
                assert!(name.as_str().contains(kind.label()));
            }
        }
    }

    #[test]
    fn disturbed_objects_move_and_the_stacks_can_come_to_rest_and_sleep() {
        let mut app = stacking_app(false);
        app.update();
        let by_kind = objects(&mut app);
        let top_cube = by_kind[&StackingObjectKind::Cube][2];
        let starting_position = app.world().entity(top_cube).get::<Position>().unwrap().0;

        app.world_mut().entity_mut(top_cube).insert((
            LinearVelocity(Vec3::new(2.0, 2.0, 0.0)),
            AngularVelocity(Vec3::new(0.0, 0.0, 4.0)),
        ));
        run_steps(&mut app, 45);
        let disturbed_position = app.world().entity(top_cube).get::<Position>().unwrap().0;
        assert!(
            disturbed_position.distance(starting_position) > 0.2,
            "disturbing a stack did not move its top cube: {starting_position:?} -> {disturbed_position:?}"
        );

        run_steps(&mut app, 420);
        let world = app.world_mut();
        let mut query = world.query::<(
            &StackingLabObject,
            &Position,
            &LinearVelocity,
            Has<Sleeping>,
        )>();
        let mut sleeping = 0;
        let mut resting = 0;
        for (object, position, velocity, is_sleeping) in query.iter(world) {
            assert!(
                position.0.y >= object.kind.half_height() - 0.2,
                "{} fell through the floor: {:?}",
                object.kind.label(),
                position.0
            );
            if velocity.0.length() < 0.2 {
                resting += 1;
            }
            sleeping += is_sleeping as usize;
        }
        assert!(
            resting >= 10,
            "too few stack objects came to rest: {resting}"
        );
        assert!(sleeping >= 5, "stack objects never entered sleeping state");
    }

    #[test]
    fn sleep_controls_count_sleep_wake_and_toggle_only_eligible_lab_bodies() {
        let mut app = stacking_app(false);
        app.update();
        let by_kind = objects(&mut app);
        let eligible = by_kind[&StackingObjectKind::Cube][0];
        let disabled = by_kind[&StackingObjectKind::Cube][1];
        let sleeping_disabled = by_kind[&StackingObjectKind::Cube][2];
        app.world_mut()
            .entity_mut(sleeping_disabled)
            .insert(SleepingDisabled);

        let sleep_all_button = {
            let world = app.world_mut();
            let mut buttons = world.query_filtered::<Entity, With<StackingStationControl>>();
            buttons
                .iter(world)
                .find(|entity| {
                    world.entity(*entity).get::<StackingStationControl>()
                        == Some(&StackingStationControl::SleepAll)
                })
                .expect("Sleep All button")
        };
        app.world_mut()
            .entity_mut(sleep_all_button)
            .insert(Interaction::Pressed);
        app.update();
        assert!(app.world().entity(eligible).contains::<Sleeping>());
        assert!(app.world().entity(disabled).contains::<Sleeping>());
        assert!(!app.world().entity(sleeping_disabled).contains::<Sleeping>());
        let status = app
            .world_mut()
            .query_filtered::<&Text, With<SleepingStatusText>>()
            .single(app.world())
            .expect("sleep status text");
        assert!(status.0.contains("Sleeping: 14"), "{status:?}");

        app.world_mut()
            .resource_mut::<Messages<SleepingControlAction>>()
            .write(SleepingControlAction::WakeAll);
        app.update();
        assert!(!app.world().entity(eligible).contains::<Sleeping>());
        assert!(!app.world().entity(disabled).contains::<Sleeping>());

        app.world_mut()
            .resource_mut::<Messages<SleepingControlAction>>()
            .write(SleepingControlAction::ToggleSleeping);
        app.update();
        assert!(!app.world().resource::<SleepingControlsState>().enabled);
        assert!(app.world().entity(eligible).contains::<SleepingDisabled>());
        assert!(
            app.world()
                .entity(eligible)
                .contains::<SleepingDisabledByStackingControls>()
        );
        run_steps(&mut app, 480);
        let world = app.world_mut();
        let mut disabled_bodies = world.query_filtered::<&Sleeping, With<StackingLabObject>>();
        assert_eq!(disabled_bodies.iter(world).count(), 0);
        app.world_mut()
            .resource_mut::<Messages<SleepingControlAction>>()
            .write(SleepingControlAction::SleepAll);
        app.update();
        assert!(!app.world().entity(eligible).contains::<Sleeping>());

        app.world_mut()
            .resource_mut::<Messages<SleepingControlAction>>()
            .write(SleepingControlAction::ToggleSleeping);
        app.update();
        assert!(app.world().resource::<SleepingControlsState>().enabled);
        assert!(!app.world().entity(eligible).contains::<SleepingDisabled>());
        assert!(
            app.world()
                .entity(sleeping_disabled)
                .contains::<SleepingDisabled>()
        );
        run_steps(&mut app, 480);
        let world = app.world_mut();
        let mut sleeping_bodies = world.query_filtered::<&Sleeping, With<StackingLabObject>>();
        assert!(sleeping_bodies.iter(world).count() > 0);
    }

    #[test]
    fn resetting_station_j_restores_the_stack_without_touching_another_station() {
        let mut app = stacking_app(true);
        app.update();
        let by_kind = objects(&mut app);
        let cube = by_kind[&StackingObjectKind::Cube][0];
        let initial_cube = OBJECT_DEFINITIONS[0].transform();
        let unrelated_initial = Transform::from_xyz(4.0, 1.0, 4.0);
        let unrelated = app
            .world_mut()
            .spawn((
                StationObject::new('A', unrelated_initial),
                unrelated_initial,
            ))
            .id();

        app.world_mut().entity_mut(cube).insert((
            Position(Vec3::new(5.0, 4.0, 8.0)),
            Transform::from_xyz(5.0, 4.0, 8.0),
            LinearVelocity(Vec3::new(4.0, 3.0, 2.0)),
            AngularVelocity(Vec3::new(2.0, 1.0, 0.0)),
        ));
        app.world_mut()
            .entity_mut(unrelated)
            .insert(Transform::from_xyz(-4.0, 2.0, -4.0));
        app.world_mut()
            .resource_mut::<Messages<ResetStation>>()
            .write(ResetStation { code: 'J' });
        app.update();

        let cube = app.world().entity(cube);
        assert_eq!(cube.get::<Transform>(), Some(&initial_cube));
        assert_eq!(
            cube.get::<Position>(),
            Some(&Position(initial_cube.translation))
        );
        assert_eq!(cube.get::<LinearVelocity>(), Some(&LinearVelocity::ZERO));
        assert_eq!(
            app.world().entity(unrelated).get::<Transform>(),
            Some(&Transform::from_xyz(-4.0, 2.0, -4.0))
        );
    }
}
