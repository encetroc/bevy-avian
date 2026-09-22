use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use std::collections::HashSet;

/// Owns the connected, labelled test stations that make up the sandbox map.
///
/// The layout is deliberately static: every station is spawned into the same
/// world so the player can walk between experiments without a scene transition.
pub struct StationLayoutPlugin;

impl Plugin for StationLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ResetStation>()
            .add_systems(Startup, spawn_station_layout)
            .add_systems(
                Update,
                (
                    reset_station_objects,
                    draw_station_labels.run_if(resource_exists::<GizmoConfigStore>),
                ),
            );
    }
}

/// A station's stable identifier and display name.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Station {
    pub code: char,
    pub name: &'static str,
}

/// The initial state and station ownership of a resettable baseline object.
///
/// Baseline objects stay in the world when a station is reset. The reset
/// message restores this state rather than replacing the entity, so references
/// held by other systems remain valid.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StationObject {
    pub station: char,
    pub initial_transform: Transform,
    pub initial_linear_velocity: Vec3,
    pub initial_angular_velocity: Vec3,
}

impl StationObject {
    pub const fn new(station: char, initial_transform: Transform) -> Self {
        Self {
            station,
            initial_transform,
            initial_linear_velocity: Vec3::ZERO,
            initial_angular_velocity: Vec3::ZERO,
        }
    }

    pub const fn with_velocities(mut self, linear_velocity: Vec3, angular_velocity: Vec3) -> Self {
        self.initial_linear_velocity = linear_velocity;
        self.initial_angular_velocity = angular_velocity;
        self
    }
}

/// Requests that one station restore its baseline objects.
///
/// Station-specific controls can observe the same message to restore their
/// own state. Each message is scoped to one station, which keeps a local reset
/// from affecting unrelated stations.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetStation {
    pub code: char,
}

/// Marks the station entity as having a readable in-world label.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationLabel {
    pub code: char,
    pub name: &'static str,
}

/// A visible route between two stations. The arena floor underneath remains
/// continuous, while these routes make the connections easy to read from the
/// overview camera.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StationPath {
    pub from: char,
    pub to: char,
}

#[derive(Clone, Copy)]
struct StationDefinition {
    code: char,
    name: &'static str,
    position: Vec3,
    pad_size: Vec3,
}

const STATIONS: [StationDefinition; 10] = [
    station('A', "Rigid Bodies", Vec3::new(-6.0, 0.0, -6.0)),
    station('B', "Materials", Vec3::new(0.0, 0.0, -6.0)),
    station('C', "Colliders", Vec3::new(6.0, 0.0, -6.0)),
    station('D', "Forces", Vec3::new(-6.0, 0.0, 0.0)),
    station('E', "Joints", Vec3::new(0.0, 0.0, 0.0)),
    station('F', "Sensors", Vec3::new(6.0, 0.0, 0.0)),
    station('G', "Spatial Queries", Vec3::new(-6.0, 0.0, 6.0)),
    station('H', "CCD", Vec3::new(0.0, 0.0, 6.0)),
    station('I', "Stress Test", Vec3::new(6.0, 0.0, 6.0)),
    StationDefinition {
        code: 'J',
        name: "Free Sandbox",
        position: Vec3::new(0.0, 0.0, 8.8),
        pad_size: Vec3::new(4.3, 0.12, 1.8),
    },
];

const CONNECTIONS: [(char, char); 13] = [
    ('A', 'B'),
    ('B', 'C'),
    ('D', 'E'),
    ('E', 'F'),
    ('G', 'H'),
    ('H', 'I'),
    ('A', 'D'),
    ('B', 'E'),
    ('C', 'F'),
    ('D', 'G'),
    ('E', 'H'),
    ('F', 'I'),
    ('H', 'J'),
];

const fn station(code: char, name: &'static str, position: Vec3) -> StationDefinition {
    StationDefinition {
        code,
        name,
        position,
        pad_size: Vec3::new(4.3, 0.12, 2.8),
    }
}

fn spawn_station_layout(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let pad_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.10, 0.30, 0.38)));
    let path_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.22, 0.42, 0.48)));

    for definition in STATIONS {
        let station_entity = commands
            .spawn((
                Station {
                    code: definition.code,
                    name: definition.name,
                },
                StationLabel {
                    code: definition.code,
                    name: definition.name,
                },
                Transform::from_translation(definition.position),
                Name::new(format!("Station {} - {}", definition.code, definition.name)),
            ))
            .id();

        if let (Some(mesh), Some(material)) = (mesh.as_ref(), pad_material.as_ref()) {
            commands.entity(station_entity).with_child((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::Y * 0.08).with_scale(definition.pad_size),
                Name::new(format!("Station {} Pad", definition.code)),
            ));
        }
    }

    for (from, to) in CONNECTIONS {
        let start = station_position(from);
        let end = station_position(to);
        let delta = end - start;
        let horizontal = delta.x.abs() > delta.z.abs();
        let size = if horizontal {
            Vec3::new(delta.x.abs(), 0.08, 1.0)
        } else {
            Vec3::new(1.0, 0.08, delta.z.abs())
        };
        let midpoint = (start + end) * 0.5 + Vec3::Y * 0.04;

        let mut path = commands.spawn((
            StationPath { from, to },
            Transform::from_translation(midpoint).with_scale(size),
            Name::new(format!("Station Path {from}-{to}")),
        ));
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), path_material.as_ref()) {
            path.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
        }
    }
}

fn station_position(code: char) -> Vec3 {
    STATIONS
        .iter()
        .find(|station| station.code == code)
        .map(|station| station.position)
        .expect("station connection references a declared station")
}

type StationObjectQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static StationObject,
        &'static mut Transform,
        Option<&'static mut Position>,
        Option<&'static mut Rotation>,
        Option<&'static mut LinearVelocity>,
        Option<&'static mut AngularVelocity>,
    ),
>;

fn reset_station_objects(
    mut requests: MessageReader<ResetStation>,
    mut objects: StationObjectQuery,
) {
    let requested_stations: HashSet<char> = requests.read().map(|request| request.code).collect();
    if requested_stations.is_empty() {
        return;
    }

    for (object, mut transform, position, rotation, linear_velocity, angular_velocity) in
        &mut objects
    {
        if !requested_stations.contains(&object.station) {
            continue;
        }

        *transform = object.initial_transform;
        if let Some(mut position) = position {
            position.0 = object.initial_transform.translation;
        }
        if let Some(mut rotation) = rotation {
            rotation.0 = object.initial_transform.rotation;
        }
        if let Some(mut linear_velocity) = linear_velocity {
            linear_velocity.0 = object.initial_linear_velocity;
        }
        if let Some(mut angular_velocity) = angular_velocity {
            angular_velocity.0 = object.initial_angular_velocity;
        }
    }
}

fn draw_station_labels(mut gizmos: Gizmos, stations: Query<(&StationLabel, &Transform)>) {
    for (label, transform) in &stations {
        let text = format!("{}\n{}", label.code, label.name);
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 1.45, Quat::IDENTITY),
            &text,
            0.62,
            Vec2::ZERO,
            Color::srgb(1.0, 0.88, 0.25),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    fn station_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            StationLayoutPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    #[test]
    fn layout_spawns_exactly_one_labelled_station_for_each_code() {
        let mut app = station_app();
        app.update();

        let world = app.world_mut();
        let mut stations = world.query::<&Station>();
        let codes: HashSet<char> = stations.iter(world).map(|station| station.code).collect();
        assert_eq!(stations.iter(world).count(), 10);
        assert_eq!(codes, ('A'..='J').collect());

        let mut labels = world.query::<&StationLabel>();
        assert_eq!(labels.iter(world).count(), 10);
    }

    #[test]
    fn station_names_are_readable_and_unique() {
        let mut app = station_app();
        app.update();

        let world = app.world_mut();
        let mut labels = world.query::<&StationLabel>();
        let names: HashSet<&'static str> = labels.iter(world).map(|label| label.name).collect();
        assert_eq!(names.len(), 10);
        assert!(names.contains("Rigid Bodies"));
        assert!(names.contains("Free Sandbox"));
    }

    #[test]
    fn station_paths_connect_the_whole_map_without_scene_changes() {
        let mut app = station_app();
        app.update();

        let world = app.world_mut();
        let mut paths = world.query::<&StationPath>();
        let connections: Vec<_> = paths.iter(world).copied().collect();
        assert_eq!(connections.len(), CONNECTIONS.len());

        let mut graph: HashMap<char, Vec<char>> = HashMap::new();
        for connection in connections {
            graph
                .entry(connection.from)
                .or_default()
                .push(connection.to);
            graph
                .entry(connection.to)
                .or_default()
                .push(connection.from);
        }

        let mut visited = HashSet::new();
        let mut pending = vec!['A'];
        while let Some(code) = pending.pop() {
            if visited.insert(code) {
                pending.extend(graph.get(&code).into_iter().flatten().copied());
            }
        }

        assert_eq!(visited, ('A'..='J').collect());
    }

    #[test]
    fn all_station_pads_fit_inside_the_arena() {
        for station in STATIONS {
            let half_size = station.pad_size * 0.5;
            assert!(station.position.x.abs() + half_size.x <= 10.0);
            assert!(station.position.z.abs() + half_size.z <= 10.0);
        }
    }

    #[test]
    fn resetting_one_station_restores_its_objects_without_touching_another() {
        let mut app = station_app();
        app.update();

        let initial_a =
            Transform::from_xyz(-6.0, 1.0, -6.0).with_rotation(Quat::from_rotation_y(0.35));
        let initial_b =
            Transform::from_xyz(0.0, 1.0, -6.0).with_rotation(Quat::from_rotation_x(-0.2));
        let station_a = app
            .world_mut()
            .spawn((
                StationObject::new('A', initial_a)
                    .with_velocities(Vec3::new(0.5, 0.0, 0.0), Vec3::new(0.0, 0.25, 0.0)),
                initial_a,
                Position(Vec3::new(-6.0, 1.0, -6.0)),
                Rotation(initial_a.rotation),
                LinearVelocity(Vec3::new(0.5, 0.0, 0.0)),
                AngularVelocity(Vec3::new(0.0, 0.25, 0.0)),
            ))
            .id();
        let station_b = app
            .world_mut()
            .spawn((StationObject::new('B', initial_b), initial_b))
            .id();

        let disturbed_a = Transform::from_xyz(3.0, 4.0, 5.0);
        let disturbed_b = Transform::from_xyz(-3.0, 2.0, 1.0);
        app.world_mut().entity_mut(station_a).insert((
            disturbed_a,
            Position(disturbed_a.translation),
            Rotation(disturbed_a.rotation),
            LinearVelocity(Vec3::new(9.0, 8.0, 7.0)),
            AngularVelocity(Vec3::new(6.0, 5.0, 4.0)),
        ));
        app.world_mut().entity_mut(station_b).insert(disturbed_b);

        app.world_mut()
            .resource_mut::<Messages<ResetStation>>()
            .write(ResetStation { code: 'A' });
        app.update();

        let a = app.world().entity(station_a);
        assert_eq!(a.get::<Transform>(), Some(&initial_a));
        assert_eq!(a.get::<Position>(), Some(&Position(initial_a.translation)));
        assert_eq!(a.get::<Rotation>(), Some(&Rotation(initial_a.rotation)));
        assert_eq!(
            a.get::<LinearVelocity>(),
            Some(&LinearVelocity(Vec3::new(0.5, 0.0, 0.0)))
        );
        assert_eq!(
            a.get::<AngularVelocity>(),
            Some(&AngularVelocity(Vec3::new(0.0, 0.25, 0.0)))
        );
        assert_eq!(
            app.world().entity(station_b).get::<Transform>(),
            Some(&disturbed_b)
        );

        let mut objects = app.world_mut().query::<&StationObject>();
        assert_eq!(objects.iter(app.world()).count(), 2);
    }
}
