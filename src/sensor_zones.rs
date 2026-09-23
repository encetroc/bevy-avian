use avian3d::prelude::*;
use bevy::prelude::*;

use crate::dynamic_objects::{DynamicObjectKind, DynamicTestObject};
use crate::stations::StationObject;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const SENSOR_COLOR: Color = Color::srgba(0.15, 0.8, 1.0, 0.24);
const SENSOR_ZONE_SIZE: Vec3 = Vec3::new(2.4, 1.8, 1.8);
const SENSOR_ZONE_POSITION: Vec3 = Vec3::new(6.0, 0.9, 0.0);
const SENSOR_OBJECT_POSITION: Vec3 = Vec3::new(4.0, 1.0, 0.0);

/// Marks the transparent trigger volume used by station F.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SensorZone;

/// Marks the movable demonstration object placed beside station F's sensor.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SensorZoneObject;

/// Owns station F's non-blocking sensor volume and its demonstration object.
pub struct SensorZonesPlugin;

impl Plugin for SensorZonesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_sensor_zone, spawn_sensor_zone_ui))
            .add_systems(
                Update,
                draw_sensor_zone_labels.run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

fn spawn_sensor_zone(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let sensor_transform = Transform::from_translation(SENSOR_ZONE_POSITION);
    let sensor_material = materials.as_mut().map(|materials| {
        materials.add(StandardMaterial {
            base_color: SENSOR_COLOR,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })
    });

    let mut sensor = commands.spawn((
        SensorZone,
        RigidBody::Static,
        Collider::cuboid(SENSOR_ZONE_SIZE.x, SENSOR_ZONE_SIZE.y, SENSOR_ZONE_SIZE.z),
        Sensor,
        CollisionEventsEnabled,
        sensor_transform,
        Name::new("Sensor Zone F"),
    ));
    if let (Some(meshes), Some(material)) = (meshes.as_mut(), sensor_material.as_ref()) {
        sensor.insert((
            Mesh3d(meshes.add(Cuboid::from_size(SENSOR_ZONE_SIZE))),
            MeshMaterial3d(material.clone()),
        ));
    }

    let object_transform = Transform::from_translation(SENSOR_OBJECT_POSITION);
    let object_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.98, 0.72, 0.22)));
    let mut object = commands.spawn((
        SensorZoneObject,
        DynamicTestObject {
            kind: DynamicObjectKind::Cube,
        },
        StationObject::new('F', object_transform),
        RigidBody::Dynamic,
        Collider::cuboid(0.8, 0.8, 0.8),
        SleepingDisabled,
        object_transform,
        Name::new("Sensor Zone Test Object"),
    ));
    if let (Some(meshes), Some(material)) = (meshes.as_mut(), object_material.as_ref()) {
        object.insert((
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.8)))),
            MeshMaterial3d(material.clone()),
        ));
    }
}

#[derive(Component)]
struct SensorZonePanel;

fn spawn_sensor_zone_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            bottom: px(280.0),
            width: px(390.0),
            padding: UiRect::all(px(14.0)),
            flex_direction: FlexDirection::Column,
            row_gap: px(4.0),
            ..default()
        },
        BackgroundColor(PANEL_BACKGROUND),
        SensorZonePanel,
        Name::new("Sensor Zone Controls"),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("F — SENSOR ZONE"),
            TextFont::from_font_size(18.0),
            TextColor(SENSOR_COLOR),
        ));
        parent.spawn((
            Text::new(
                "The transparent volume is a sensor: the player and test object pass through it. Crossing the boundary records ENTER and EXIT in the physics event log.",
            ),
            TextFont::from_font_size(12.0),
            TextColor(PANEL_TEXT),
        ));
    });
}

fn draw_sensor_zone_labels(mut gizmos: Gizmos, zones: Query<(&SensorZone, &Transform)>) {
    for (_, transform) in &zones {
        gizmos.text(
            Isometry3d::new(
                transform.translation + Vec3::Y * (SENSOR_ZONE_SIZE.y * 0.5 + 0.3),
                Quat::IDENTITY,
            ),
            "SENSOR\nENTER / EXIT",
            0.25,
            Vec2::ZERO,
            SENSOR_COLOR,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::collision_event_log::{
        CollisionEventLog, CollisionEventLogPlugin, CollisionEventType,
    };

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn sensor_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CollisionEventLogPlugin,
            SensorZonesPlugin,
        ))
        .insert_resource(Assets::<StandardMaterial>::default())
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn sensor_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut sensors = world.query_filtered::<Entity, With<SensorZone>>();
        sensors.iter(world).next().expect("sensor zone")
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    #[derive(Component)]
    struct TestKinematicPlayer;

    fn spawn_crossing_body(app: &mut App, name: &'static str, body: RigidBody) -> Entity {
        app.world_mut()
            .spawn((
                body,
                Collider::cuboid(0.4, 0.4, 0.4),
                Position(Vec3::new(3.0, 0.9, 0.0)),
                Transform::from_xyz(3.0, 0.9, 0.0),
                LinearVelocity(Vec3::new(3.0, 0.0, 0.0)),
                CollisionEventsEnabled,
                Name::new(name),
            ))
            .id()
    }

    fn pair_event(
        log: &CollisionEventLog,
        event_type: CollisionEventType,
        sensor: Entity,
        body: Entity,
    ) -> bool {
        log.entries.iter().any(|entry| {
            entry.event_type == event_type
                && entry.entities.contains(&sensor)
                && entry.entities.contains(&body)
        })
    }

    #[test]
    fn sensor_zone_is_static_sensor_with_transparent_visuals() {
        let mut app = sensor_app();
        app.update();

        let sensor = sensor_entity(&mut app);
        let entity = app.world().entity(sensor);
        assert_eq!(entity.get::<RigidBody>(), Some(&RigidBody::Static));
        assert!(entity.contains::<Sensor>());
        assert!(entity.contains::<CollisionEventsEnabled>());
        assert!(entity.contains::<Collider>());

        let material_handle = entity
            .get::<MeshMaterial3d<StandardMaterial>>()
            .expect("sensor visual material")
            .0
            .clone();
        let material = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&material_handle)
            .expect("sensor material asset");
        assert_eq!(material.alpha_mode, AlphaMode::Blend);
        assert!(material.base_color.alpha() < 1.0);
    }

    #[test]
    fn dynamic_object_and_kinematic_player_cross_sensor_without_blocking() {
        let mut app = sensor_app();
        app.update();
        let sensor = sensor_entity(&mut app);
        let object = spawn_crossing_body(&mut app, "Sensor Test Object", RigidBody::Dynamic);
        let player = spawn_crossing_body(&mut app, "Sensor Test Player", RigidBody::Kinematic);
        app.world_mut()
            .entity_mut(player)
            .insert(TestKinematicPlayer);

        app.add_systems(FixedUpdate, move_test_player);
        run_steps(&mut app, 150);

        for (name, entity) in [("object", object), ("player", player)] {
            let position = app.world().entity(entity).get::<Position>().unwrap().0;
            assert!(
                position.x > 8.0,
                "sensor blocked the {name}: final position {position:?}"
            );
            let log = app.world().resource::<CollisionEventLog>();
            assert!(
                pair_event(log, CollisionEventType::Enter, sensor, entity),
                "sensor did not record {name} entry: {:?}",
                log.entries
            );
            assert!(
                pair_event(log, CollisionEventType::Exit, sensor, entity),
                "sensor did not record {name} exit: {:?}",
                log.entries
            );
            assert!(
                !log.entries.iter().any(|entry| {
                    entry.event_type == CollisionEventType::Contact
                        && entry.entities.contains(&sensor)
                        && entry.entities.contains(&entity)
                }),
                "sensor should not produce physical contact events: {:?}",
                log.entries
            );
        }
    }

    fn move_test_player(
        mut player: Query<(&mut Position, &mut Transform), With<TestKinematicPlayer>>,
    ) {
        for (mut position, mut transform) in &mut player {
            position.0.x += 3.0 / 60.0;
            transform.translation = position.0;
        }
    }
}
