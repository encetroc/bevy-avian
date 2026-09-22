use bevy::prelude::*;

/// Owns the connected, labelled test stations that make up the sandbox map.
///
/// The layout is deliberately static: every station is spawned into the same
/// world so the player can walk between experiments without a scene transition.
pub struct StationLayoutPlugin;

impl Plugin for StationLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_station_layout).add_systems(
            Update,
            draw_station_labels.run_if(resource_exists::<GizmoConfigStore>),
        );
    }
}

/// A station's stable identifier and display name.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Station {
    pub code: char,
    pub name: &'static str,
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
    use std::collections::{HashMap, HashSet};
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
}
