use avian3d::prelude::*;
use bevy::prelude::*;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const BODY_RADIUS: f32 = 0.18;
const GRID_WIDTH: usize = 10;
const LAYER_CAPACITY: usize = GRID_WIDTH * GRID_WIDTH;
const GRID_SPACING: f32 = 0.43;
const STATION_CENTER: Vec3 = Vec3::new(6.0, 0.0, 6.0);

/// Marks a dynamic body created by station I's counted-spawning controls.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct CountedBody;

/// Owns the stress-test station's counted body spawning and cleanup controls.
pub struct CountedBodySpawningPlugin;

impl Plugin for CountedBodySpawningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnedBodyCount>()
            .add_systems(Startup, spawn_counted_body_controls)
            .add_systems(Update, apply_counted_body_controls);
    }
}

#[derive(Resource, Default)]
struct SpawnedBodyCount(usize);

#[derive(Component, Clone, Copy)]
enum CountedBodyControl {
    Spawn(usize),
    Clear,
}

fn spawn_counted_body_controls(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(16.0),
                width: px(300.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            Name::new("Counted Body Spawning Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("I — COUNTED BODY STRESS TEST"),
                TextFont::from_font_size(16.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Spawn dynamic bodies to compare physics diagnostics."),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));

            for count in [10, 100, 500, 1_000] {
                spawn_control_button(
                    parent,
                    CountedBodyControl::Spawn(count),
                    &format!("spawn {count}"),
                );
            }
            spawn_control_button(parent, CountedBodyControl::Clear, "clear spawned bodies");
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
                height: px(26.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Counted body control: {label}")),
        ))
        .with_child((
            Text::new(label.to_owned()),
            TextFont::from_font_size(12.0),
            TextColor(PANEL_TEXT),
        ));
}

fn apply_counted_body_controls(
    mut commands: Commands,
    controls: Query<(&Interaction, &CountedBodyControl), Changed<Interaction>>,
    mut spawned_count: ResMut<SpawnedBodyCount>,
    spawned_bodies: Query<Entity, With<CountedBody>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match control {
            CountedBodyControl::Spawn(count) => {
                let mesh = meshes
                    .as_mut()
                    .map(|meshes| meshes.add(Sphere::new(BODY_RADIUS)));
                let material = materials
                    .as_mut()
                    .map(|materials| materials.add(Color::srgb(0.95, 0.48, 0.2)));
                spawn_counted_bodies(
                    &mut commands,
                    &mut spawned_count,
                    *count,
                    mesh.as_ref(),
                    material.as_ref(),
                );
            }
            CountedBodyControl::Clear => {
                for entity in &spawned_bodies {
                    commands.entity(entity).despawn();
                }
                spawned_count.0 = 0;
            }
        }
    }
}

fn spawn_counted_bodies(
    commands: &mut Commands,
    spawned_count: &mut SpawnedBodyCount,
    count: usize,
    mesh: Option<&Handle<Mesh>>,
    material: Option<&Handle<StandardMaterial>>,
) {
    for _ in 0..count {
        let index = spawned_count.0;
        let column = index % GRID_WIDTH;
        let row = (index / GRID_WIDTH) % GRID_WIDTH;
        let layer = index / LAYER_CAPACITY;
        let position = Vec3::new(
            STATION_CENTER.x + (column as f32 - (GRID_WIDTH - 1) as f32 * 0.5) * GRID_SPACING,
            0.45 + layer as f32 * GRID_SPACING,
            STATION_CENTER.z + (row as f32 - (GRID_WIDTH - 1) as f32 * 0.5) * GRID_SPACING,
        );
        let transform = Transform::from_translation(position);
        let mut body = commands.spawn((
            CountedBody,
            RigidBody::Dynamic,
            Collider::sphere(BODY_RADIUS),
            ColliderDensity(1.0),
            layers_for(SandboxLayer::Objects),
            StationObject::new('I', transform),
            transform,
            Name::new(format!("Counted Stress Body {index}")),
        ));
        if let (Some(mesh), Some(material)) = (mesh, material) {
            body.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
        }
        spawned_count.0 += 1;
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

    fn count_spawned(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<Entity, With<CountedBody>>();
        bodies.iter(world).count()
    }

    #[test]
    fn each_counted_spawn_action_creates_the_requested_dynamic_bodies() {
        for expected in [10, 100, 500, 1_000] {
            let mut app = test_app();
            app.update();
            let baseline_dynamic = {
                let world = app.world_mut();
                let mut bodies = world.query_filtered::<Entity, With<RigidBody>>();
                bodies.iter(world).count()
            };

            click(
                &mut app,
                |control| matches!(control, CountedBodyControl::Spawn(count) if count == expected),
            );

            assert_eq!(count_spawned(&mut app), expected);
            let diagnostic_counts = {
                let world = app.world_mut();
                let mut bodies = world.query::<(&RigidBody, &Collider)>();
                let all: Vec<_> = bodies.iter(world).collect();
                (
                    all.iter()
                        .filter(|(body, _)| **body == RigidBody::Dynamic)
                        .count(),
                    all.len(),
                )
            };
            assert_eq!(diagnostic_counts, (expected, expected));
            let spawned: Vec<Entity> = {
                let world = app.world_mut();
                let mut dynamics = world.query_filtered::<Entity, With<CountedBody>>();
                dynamics.iter(world).collect()
            };
            assert!(spawned.iter().all(|&entity| {
                app.world().entity(entity).get::<RigidBody>() == Some(&RigidBody::Dynamic)
                    && app.world().entity(entity).contains::<Collider>()
            }));
            assert_eq!(baseline_dynamic, 0);
        }
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

        for expected in [10, 100, 500, 1_000] {
            click(
                &mut app,
                |control| matches!(control, CountedBodyControl::Spawn(count) if count == expected),
            );
            assert_eq!(count_spawned(&mut app), expected);
            click(&mut app, |control| {
                matches!(control, CountedBodyControl::Clear)
            });
            assert_eq!(count_spawned(&mut app), 0);
            let remaining_diagnostics = {
                let world = app.world_mut();
                let mut bodies = world.query::<(&RigidBody, &Collider)>();
                bodies.iter(world).count()
            };
            assert_eq!(remaining_diagnostics, 0);
            assert!(app.world().entities().contains(station));
            assert_eq!(app.world().resource::<SpawnedBodyCount>().0, 0);
        }
    }
}
