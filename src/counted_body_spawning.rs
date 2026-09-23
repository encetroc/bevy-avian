use avian3d::prelude::*;
use bevy::prelude::*;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const BODY_RADIUS: f32 = 0.18;
const BODY_HALF_EXTENT: f32 = 0.18;
const GRID_WIDTH: usize = 10;
const LAYER_CAPACITY: usize = GRID_WIDTH * GRID_WIDTH;
const GRID_SPACING: f32 = 0.43;
const STATION_CENTER: Vec3 = Vec3::new(6.0, 0.0, 6.0);

/// Marks a dynamic body created by station I's stress-spawning controls.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct CountedBody;

/// The stress-test shape represented by one spawned body.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum StressObjectKind {
    #[default]
    Cube,
    Sphere,
    Mixed,
    Compound,
    JointChain,
}

impl StressObjectKind {
    const ALL: [Self; 5] = [
        Self::Cube,
        Self::Sphere,
        Self::Mixed,
        Self::Compound,
        Self::JointChain,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Cube => "CUBES",
            Self::Sphere => "SPHERES",
            Self::Mixed => "MIXED",
            Self::Compound => "COMPOUND",
            Self::JointChain => "JOINT CHAINS",
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Cube => Color::srgb(0.98, 0.72, 0.22),
            Self::Sphere => Color::srgb(0.35, 0.82, 1.0),
            Self::Mixed => Color::srgb(0.72, 0.4, 0.95),
            Self::Compound => Color::srgb(0.25, 0.9, 0.5),
            Self::JointChain => Color::srgb(0.98, 0.42, 0.28),
        }
    }
}

/// Owns the stress-test station's shape, load, and cleanup controls.
pub struct CountedBodySpawningPlugin;

impl Plugin for CountedBodySpawningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnedBodyCount>()
            .init_resource::<SelectedStressMode>()
            .add_systems(Startup, spawn_counted_body_controls)
            .add_systems(
                Update,
                (apply_counted_body_controls, update_stress_load_text),
            );
    }
}

#[derive(Resource, Default)]
struct SpawnedBodyCount(usize);

#[derive(Resource, Default)]
struct SelectedStressMode(StressObjectKind);

#[derive(Component, Clone, Copy)]
enum CountedBodyControl {
    Select(StressObjectKind),
    Spawn(usize),
    Clear,
}

#[derive(Component)]
struct StressLoadText;

#[derive(Component)]
struct StressSpawned;

#[derive(Component)]
struct StressJoint;

fn spawn_counted_body_controls(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(16.0),
                width: px(310.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            Name::new("Stress Test Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("I — STRESS TEST"),
                TextFont::from_font_size(16.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Choose a body mode, then a spawn count."),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));
            for mode in StressObjectKind::ALL {
                spawn_control_button(parent, CountedBodyControl::Select(mode), mode.label());
            }
            for count in [10, 100, 500, 1_000] {
                spawn_control_button(
                    parent,
                    CountedBodyControl::Spawn(count),
                    &format!("spawn {count}"),
                );
            }
            spawn_control_button(parent, CountedBodyControl::Clear, "clear spawned load");
            parent.spawn((
                Text::new("MODE: CUBES\nLOAD: 0 bodies · 0 colliders · 0 joints"),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
                StressLoadText,
            ));
        });
}

fn spawn_control_button(
    parent: &mut ChildSpawnerCommands,
    control: CountedBodyControl,
    label: &str,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: percent(100),
                height: px(24.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Stress test control: {label}")),
        ))
        .with_child((
            Text::new(label.to_owned()),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn apply_counted_body_controls(
    mut commands: Commands,
    controls: Query<(&Interaction, &CountedBodyControl), Changed<Interaction>>,
    mut spawned_count: ResMut<SpawnedBodyCount>,
    mut selected_mode: ResMut<SelectedStressMode>,
    spawned_entities: Query<Entity, With<StressSpawned>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match control {
            CountedBodyControl::Select(mode) => selected_mode.0 = *mode,
            CountedBodyControl::Spawn(count) => {
                let materials: Option<Vec<_>> = materials.as_mut().map(|materials| {
                    StressObjectKind::ALL
                        .into_iter()
                        .map(|kind| materials.add(kind.color()))
                        .collect()
                });
                let body_meshes: Option<Vec<_>> = meshes.as_mut().map(|meshes| {
                    vec![
                        meshes.add(Cuboid::from_size(Vec3::splat(BODY_HALF_EXTENT * 2.0))),
                        meshes.add(Sphere::new(BODY_RADIUS)),
                        meshes.add(Cuboid::from_size(Vec3::splat(BODY_HALF_EXTENT * 2.0))),
                        meshes.add(Cuboid::from_size(Vec3::new(0.42, 0.18, 0.72))),
                        meshes.add(Cuboid::from_size(Vec3::splat(BODY_HALF_EXTENT * 2.0))),
                    ]
                });
                spawn_stress_bodies(
                    &mut commands,
                    &mut spawned_count,
                    *count,
                    selected_mode.0,
                    body_meshes.as_deref(),
                    materials.as_deref(),
                );
            }
            CountedBodyControl::Clear => {
                for entity in &spawned_entities {
                    commands.entity(entity).despawn();
                }
                spawned_count.0 = 0;
            }
        }
    }
}

fn spawn_stress_bodies(
    commands: &mut Commands,
    spawned_count: &mut SpawnedBodyCount,
    count: usize,
    mode: StressObjectKind,
    meshes: Option<&[Handle<Mesh>]>,
    materials: Option<&[Handle<StandardMaterial>]>,
) {
    let mut previous_chain_body = None;
    for batch_index in 0..count {
        if mode == StressObjectKind::JointChain && batch_index.is_multiple_of(GRID_WIDTH) {
            previous_chain_body = None;
        }
        let index = spawned_count.0;
        let kind = match mode {
            StressObjectKind::Mixed => {
                if index.is_multiple_of(2) {
                    StressObjectKind::Cube
                } else {
                    StressObjectKind::Sphere
                }
            }
            other => other,
        };
        let column = index % GRID_WIDTH;
        let row = (index / GRID_WIDTH) % GRID_WIDTH;
        let layer = index / LAYER_CAPACITY;
        let mut position = Vec3::new(
            STATION_CENTER.x + (column as f32 - (GRID_WIDTH - 1) as f32 * 0.5) * GRID_SPACING,
            0.45 + layer as f32 * GRID_SPACING,
            STATION_CENTER.z + (row as f32 - (GRID_WIDTH - 1) as f32 * 0.5) * GRID_SPACING,
        );
        if mode == StressObjectKind::JointChain {
            let chain_index = batch_index % GRID_WIDTH;
            let chain_row = batch_index / GRID_WIDTH;
            position = Vec3::new(
                STATION_CENTER.x - 2.0 + chain_index as f32 * 0.45,
                2.0 + chain_row as f32 * 0.65,
                STATION_CENTER.z,
            );
        }
        let transform = Transform::from_translation(position);
        let collider = match kind {
            StressObjectKind::Cube | StressObjectKind::Mixed | StressObjectKind::JointChain => {
                Collider::cuboid(
                    BODY_HALF_EXTENT * 2.0,
                    BODY_HALF_EXTENT * 2.0,
                    BODY_HALF_EXTENT * 2.0,
                )
            }
            StressObjectKind::Sphere => Collider::sphere(BODY_RADIUS),
            StressObjectKind::Compound => Collider::compound(vec![
                (
                    Vec3::new(-0.2, 0.0, 0.0),
                    Quat::IDENTITY,
                    Collider::cuboid(0.42, 0.18, 0.72),
                ),
                (
                    Vec3::new(0.2, 0.0, 0.0),
                    Quat::IDENTITY,
                    Collider::cuboid(0.42, 0.18, 0.72),
                ),
                (
                    Vec3::new(0.0, 0.18, 0.0),
                    Quat::IDENTITY,
                    Collider::cuboid(0.18, 0.36, 0.18),
                ),
            ]),
        };
        let shape_index = match kind {
            StressObjectKind::Cube => 0,
            StressObjectKind::Sphere => 1,
            StressObjectKind::Mixed => {
                if index.is_multiple_of(2) {
                    0
                } else {
                    1
                }
            }
            StressObjectKind::Compound => 3,
            StressObjectKind::JointChain => 4,
        };
        let mut body = commands.spawn((
            CountedBody,
            StressSpawned,
            kind,
            RigidBody::Dynamic,
            collider,
            ColliderDensity(1.0),
            layers_for(SandboxLayer::Objects),
            StationObject::new('I', transform),
            transform,
            Name::new(format!("Stress {} Body {index}", kind.label())),
        ));
        if kind != StressObjectKind::Compound
            && let (Some(meshes), Some(materials)) = (meshes, materials)
        {
            body.insert((
                Mesh3d(meshes[shape_index].clone()),
                MeshMaterial3d(
                    materials[match kind {
                        StressObjectKind::Mixed => {
                            if shape_index == 0 {
                                0
                            } else {
                                1
                            }
                        }
                        _ => StressObjectKind::ALL
                            .iter()
                            .position(|candidate| *candidate == kind)
                            .unwrap_or(0),
                    }]
                    .clone(),
                ),
            ));
        }
        let entity = body.id();
        if kind == StressObjectKind::Compound
            && let (Some(meshes), Some(materials)) = (meshes, materials)
        {
            for (part_position, part_size) in [
                (Vec3::new(-0.2, 0.0, 0.0), Vec3::new(0.42, 0.18, 0.72)),
                (Vec3::new(0.2, 0.0, 0.0), Vec3::new(0.42, 0.18, 0.72)),
                (Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.18, 0.36, 0.18)),
            ] {
                commands.spawn((
                    ChildOf(entity),
                    Mesh3d(meshes[3].clone()),
                    MeshMaterial3d(materials[3].clone()),
                    Transform::from_translation(part_position)
                        .with_scale(part_size / Vec3::new(0.42, 0.18, 0.72)),
                ));
            }
        }
        if mode == StressObjectKind::JointChain {
            if let Some(previous) = previous_chain_body {
                commands.spawn((
                    StressSpawned,
                    StressJoint,
                    DistanceJoint::new(previous, entity).with_limits(0.42, 0.52),
                    JointCollisionDisabled,
                    Name::new(format!("Stress Joint Chain Link {index}")),
                ));
            }
            previous_chain_body = Some(entity);
        }
        spawned_count.0 += 1;
    }
}

fn update_stress_load_text(
    selected_mode: Res<SelectedStressMode>,
    bodies: Query<(&StressObjectKind, &Collider), With<CountedBody>>,
    joints: Query<(), With<StressJoint>>,
    mut text: Query<&mut Text, With<StressLoadText>>,
) {
    let body_count = bodies.iter().count();
    let collider_count = body_count;
    let joint_count = joints.iter().count();
    for mut text in &mut text {
        text.0 = format!(
            "MODE: {}\nLOAD: {body_count} bodies · {collider_count} colliders · {joint_count} joints",
            selected_mode.0.label()
        );
    }
}

use crate::{
    collision_layers::{SandboxLayer, layers_for},
    stations::StationObject,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stations::{Station, StationLayoutPlugin};
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            CountedBodySpawningPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn control_entity(app: &mut App, target: impl Fn(CountedBodyControl) -> bool) -> Entity {
        let world = app.world_mut();
        let mut controls = world.query::<(Entity, &CountedBodyControl)>();
        controls
            .iter(world)
            .find_map(|(entity, control)| target(*control).then_some(entity))
            .expect("requested spawning control")
    }

    fn click(app: &mut App, target: impl Fn(CountedBodyControl) -> bool) {
        let entity = control_entity(app, target);
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.world_mut().entity_mut(entity).insert(Interaction::None);
        app.update();
    }

    fn select_and_spawn(app: &mut App, mode: StressObjectKind, count: usize) {
        click(
            app,
            |control| matches!(control, CountedBodyControl::Select(kind) if kind == mode),
        );
        click(
            app,
            |control| matches!(control, CountedBodyControl::Spawn(n) if n == count),
        );
    }

    fn count_bodies(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<Entity, With<CountedBody>>();
        bodies.iter(world).count()
    }

    fn count_joints(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut joints = world.query_filtered::<Entity, With<StressJoint>>();
        joints.iter(world).count()
    }

    #[test]
    fn each_stress_mode_spawns_the_expected_shape_and_load() {
        for mode in StressObjectKind::ALL {
            let mut app = test_app();
            app.update();
            select_and_spawn(&mut app, mode, 10);

            assert_eq!(count_bodies(&mut app), 10, "mode {mode:?}");
            let world = app.world_mut();
            let mut spawned = world.query::<(&StressObjectKind, &Collider, &RigidBody)>();
            let bodies: Vec<_> = spawned.iter(world).collect();
            assert_eq!(bodies.len(), 10);
            assert!(
                bodies
                    .iter()
                    .all(|(_, _, body)| **body == RigidBody::Dynamic)
            );
            match mode {
                StressObjectKind::Cube => assert!(bodies.iter().all(|(kind, _, _)| **kind == mode)),
                StressObjectKind::Sphere => {
                    assert!(bodies.iter().all(|(kind, _, _)| **kind == mode))
                }
                StressObjectKind::Mixed => {
                    assert!(
                        bodies
                            .iter()
                            .any(|(kind, _, _)| **kind == StressObjectKind::Cube)
                    );
                    assert!(
                        bodies
                            .iter()
                            .any(|(kind, _, _)| **kind == StressObjectKind::Sphere)
                    );
                }
                StressObjectKind::Compound => {
                    assert!(bodies.iter().all(|(kind, _, _)| **kind == mode))
                }
                StressObjectKind::JointChain => {
                    assert!(bodies.iter().all(|(kind, _, _)| **kind == mode))
                }
            }
            assert_eq!(
                count_joints(&mut app),
                if mode == StressObjectKind::JointChain {
                    9
                } else {
                    0
                }
            );
            let world = app.world_mut();
            let mut texts = world.query::<&Text>();
            assert!(
                texts
                    .iter(world)
                    .any(|text| text.0.contains("LOAD: 10 bodies · 10 colliders"))
            );
        }
    }

    #[test]
    fn each_count_control_creates_the_requested_number_of_bodies() {
        let mut app = test_app();
        app.update();
        for expected in [10, 100, 500, 1_000] {
            select_and_spawn(&mut app, StressObjectKind::Cube, expected);
            assert_eq!(count_bodies(&mut app), expected);
            click(&mut app, |control| {
                matches!(control, CountedBodyControl::Clear)
            });
            assert_eq!(count_bodies(&mut app), 0);
        }
    }

    #[test]
    fn clear_and_repeat_with_another_mode_isolated_from_previous_load() {
        let mut app = test_app();
        app.update();
        select_and_spawn(&mut app, StressObjectKind::Compound, 10);
        assert_eq!(count_bodies(&mut app), 10);
        click(&mut app, |control| {
            matches!(control, CountedBodyControl::Clear)
        });
        assert_eq!(count_bodies(&mut app), 0);
        assert_eq!(count_joints(&mut app), 0);

        select_and_spawn(&mut app, StressObjectKind::JointChain, 10);
        assert_eq!(count_bodies(&mut app), 10);
        assert_eq!(count_joints(&mut app), 9);
        let world = app.world_mut();
        let mut kinds = world.query::<&StressObjectKind>();
        assert!(
            kinds
                .iter(world)
                .all(|kind| *kind == StressObjectKind::JointChain)
        );
    }

    #[test]
    fn clear_removes_spawned_bodies_and_preserves_station_geometry() {
        let mut app = test_app();
        app.update();
        let station = {
            let world = app.world_mut();
            let mut stations = world.query::<(Entity, &Station)>();
            stations
                .iter(world)
                .find_map(|(entity, station)| (station.code == 'I').then_some(entity))
                .expect("stress-test station")
        };

        select_and_spawn(&mut app, StressObjectKind::Sphere, 10);
        click(&mut app, |control| {
            matches!(control, CountedBodyControl::Clear)
        });
        assert_eq!(count_bodies(&mut app), 0);
        assert!(app.world().entities().contains(station));
        assert_eq!(app.world().resource::<SpawnedBodyCount>().0, 0);
    }
}
