use avian3d::prelude::*;
use bevy::prelude::*;

use crate::camera::FixedFollowCamera;
use crate::player::{Player, PlayerInput, PlayerMovementSet, PlayerMovementSettings};
use crate::stations::StationObject;

/// The four lightweight rigid bodies used by the basic interaction test.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicObjectKind {
    Cube,
    Sphere,
    Crate,
    Barrel,
}

impl DynamicObjectKind {
    #[cfg(test)]
    const ALL: [Self; 4] = [Self::Cube, Self::Sphere, Self::Crate, Self::Barrel];

    fn label(self) -> &'static str {
        match self {
            Self::Cube => "Cube",
            Self::Sphere => "Sphere",
            Self::Crate => "Crate",
            Self::Barrel => "Barrel",
        }
    }
}

/// Marks one of the basic dynamic bodies placed in the sandbox.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct DynamicTestObject {
    pub kind: DynamicObjectKind,
}

/// Owns the reusable dynamic objects used to test gravity, contacts, and player pushes.
pub struct DynamicObjectsPlugin;

impl Plugin for DynamicObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dynamic_test_objects)
            .add_systems(FixedUpdate, apply_player_push.after(PlayerMovementSet));
    }
}

#[derive(Clone, Copy)]
struct ObjectDefinition {
    kind: DynamicObjectKind,
    position: Vec3,
    size: Vec3,
    radius: f32,
    density: f32,
}

impl ObjectDefinition {
    const fn cube(position: Vec3) -> Self {
        Self {
            kind: DynamicObjectKind::Cube,
            position,
            size: Vec3::splat(0.8),
            radius: 0.0,
            density: 0.75,
        }
    }

    const fn sphere(position: Vec3) -> Self {
        Self {
            kind: DynamicObjectKind::Sphere,
            position,
            size: Vec3::ZERO,
            radius: 0.45,
            density: 0.75,
        }
    }

    const fn crate_object(position: Vec3) -> Self {
        Self {
            kind: DynamicObjectKind::Crate,
            position,
            size: Vec3::new(1.15, 0.8, 0.95),
            radius: 0.0,
            density: 0.75,
        }
    }

    const fn barrel(position: Vec3) -> Self {
        Self {
            kind: DynamicObjectKind::Barrel,
            position,
            size: Vec3::new(0.9, 1.0, 0.9),
            radius: 0.45,
            density: 0.75,
        }
    }

    fn collider(self) -> Collider {
        match self.kind {
            DynamicObjectKind::Cube | DynamicObjectKind::Crate => {
                Collider::cuboid(self.size.x, self.size.y, self.size.z)
            }
            DynamicObjectKind::Sphere => Collider::sphere(self.radius),
            DynamicObjectKind::Barrel => Collider::cylinder(self.radius, self.size.y),
        }
    }
}

const OBJECT_DEFINITIONS: [ObjectDefinition; 4] = [
    ObjectDefinition::cube(Vec3::new(-5.5, 5.0, -5.5)),
    ObjectDefinition::sphere(Vec3::new(-3.5, 5.0, -5.5)),
    ObjectDefinition::crate_object(Vec3::new(3.5, 5.0, -5.5)),
    ObjectDefinition::barrel(Vec3::new(5.5, 5.0, -5.5)),
];

fn apply_player_push(
    settings: Option<Res<PlayerMovementSettings>>,
    camera: Option<Single<&Transform, With<FixedFollowCamera>>>,
    players: Query<(&Transform, &PlayerInput), With<Player>>,
    mut objects: Query<(&DynamicTestObject, &Transform, &mut LinearVelocity)>,
) {
    let Some(settings) = settings else {
        return;
    };
    let camera_rotation = camera
        .map(|camera| camera.rotation)
        .unwrap_or(Quat::IDENTITY);

    for (player_transform, input) in &players {
        let local_direction = Vec3::new(input.0.x, 0.0, -input.0.y);
        let mut push_direction = camera_rotation * local_direction;
        push_direction.y = 0.0;
        push_direction = push_direction.normalize_or_zero();
        if push_direction == Vec3::ZERO {
            continue;
        }

        for (object, object_transform, mut velocity) in &mut objects {
            let offset = object_transform.translation - player_transform.translation;
            let horizontal_offset = Vec2::new(offset.x, offset.z);
            let push_distance =
                player_push_radius(object.kind) + crate::player::PLAYER_RADIUS + 0.08;
            let vertical_overlap = offset.y.abs() < player_vertical_half_height(object.kind);
            if !vertical_overlap || horizontal_offset.length() > push_distance {
                continue;
            }

            // MoveAndSlide correctly prevents the kinematic player from entering the
            // object, but it projects the player's velocity to zero at the contact.
            // Transfer the intended horizontal movement here so lightweight bodies
            // receive a stable, bounded push instead of a teleport or impulse spike.
            let target_speed = settings.speed * 0.8;
            let current_speed = velocity.0.dot(push_direction);
            if current_speed < target_speed {
                velocity.0 += push_direction * (target_speed - current_speed);
            }
        }
    }
}

fn player_push_radius(kind: DynamicObjectKind) -> f32 {
    match kind {
        DynamicObjectKind::Cube => 0.4,
        DynamicObjectKind::Sphere => 0.45,
        DynamicObjectKind::Crate => 0.58,
        DynamicObjectKind::Barrel => 0.45,
    }
}

fn player_vertical_half_height(kind: DynamicObjectKind) -> f32 {
    crate::player::PLAYER_CAPSULE_LENGTH * 0.5
        + crate::player::PLAYER_RADIUS
        + match kind {
            DynamicObjectKind::Cube => 0.4,
            DynamicObjectKind::Sphere => 0.45,
            DynamicObjectKind::Crate => 0.4,
            DynamicObjectKind::Barrel => 0.5,
        }
}

fn spawn_dynamic_test_objects(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for definition in OBJECT_DEFINITIONS {
        let mesh = meshes.as_mut().map(|meshes| match definition.kind {
            DynamicObjectKind::Cube => meshes.add(Cuboid::from_size(definition.size)),
            DynamicObjectKind::Sphere => meshes.add(Sphere::new(definition.radius)),
            DynamicObjectKind::Crate => meshes.add(Cuboid::from_size(definition.size)),
            DynamicObjectKind::Barrel => {
                meshes.add(Cylinder::new(definition.radius, definition.size.y))
            }
        });
        let material = materials.as_mut().map(|materials| {
            let color = match definition.kind {
                DynamicObjectKind::Cube => Color::srgb(0.2, 0.65, 0.95),
                DynamicObjectKind::Sphere => Color::srgb(0.95, 0.3, 0.25),
                DynamicObjectKind::Crate => Color::srgb(0.7, 0.42, 0.16),
                DynamicObjectKind::Barrel => Color::srgb(0.3, 0.75, 0.35),
            };
            materials.add(color)
        });

        let initial_transform = Transform::from_translation(definition.position);
        let mut object = commands.spawn((
            DynamicTestObject {
                kind: definition.kind,
            },
            StationObject::new('A', initial_transform).with_velocities(Vec3::ZERO, Vec3::ZERO),
            RigidBody::Dynamic,
            definition.collider(),
            ColliderDensity(definition.density),
            initial_transform,
            Name::new(format!("Dynamic {}", definition.kind.label())),
        ));

        if let (Some(mesh), Some(material)) = (mesh, material) {
            object.insert((Mesh3d(mesh), MeshMaterial3d(material)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::player::{PLAYER_START_POSITION, Player};

    const PHYSICS_STEP: f32 = 1.0 / 60.0;
    const FLOOR_THICKNESS: f32 = 0.5;

    #[derive(Component)]
    struct TestFloor;

    fn physics_app(with_player: bool) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            DynamicObjectsPlugin,
        ))
        .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
        .add_systems(Startup, spawn_test_floor);

        if with_player {
            app.add_plugins(crate::player::PlayerPlugin);
        }

        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn spawn_test_floor(mut commands: Commands) {
        commands.spawn((
            TestFloor,
            RigidBody::Static,
            Collider::cuboid(22.0, FLOOR_THICKNESS, 22.0),
            Transform::from_xyz(0.0, -FLOOR_THICKNESS * 0.5, 0.0),
        ));
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn object_entities(app: &mut App) -> Vec<(Entity, DynamicObjectKind)> {
        let world = app.world_mut();
        let mut objects = world.query::<(Entity, &DynamicTestObject)>();
        objects
            .iter(world)
            .map(|(entity, object)| (entity, object.kind))
            .collect()
    }

    fn object_position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("dynamic object position")
            .0
    }

    fn support_height(kind: DynamicObjectKind) -> f32 {
        match kind {
            DynamicObjectKind::Cube => 0.4,
            DynamicObjectKind::Sphere => 0.45,
            DynamicObjectKind::Crate => 0.4,
            DynamicObjectKind::Barrel => 0.5,
        }
    }

    #[test]
    fn plugin_spawns_each_object_as_a_named_dynamic_body() {
        let mut app = physics_app(false);
        app.update();

        let objects = object_entities(&mut app);
        assert_eq!(objects.len(), DynamicObjectKind::ALL.len());
        for (entity, kind) in objects {
            let object = app.world().entity(entity);
            assert_eq!(object.get::<RigidBody>(), Some(&RigidBody::Dynamic));
            assert!(object.contains::<Collider>());
            assert_eq!(object.get::<DynamicTestObject>().unwrap().kind, kind);
            assert!(object.contains::<Name>());
        }
    }

    #[test]
    fn every_object_falls_and_remains_in_contact_with_the_floor() {
        let mut app = physics_app(false);
        app.update();
        let floor = {
            let world = app.world_mut();
            let mut floors = world.query_filtered::<Entity, With<TestFloor>>();
            floors.iter(world).next().expect("test floor")
        };
        let objects = object_entities(&mut app);

        run_steps(&mut app, 240);

        for (entity, kind) in objects {
            let position = object_position(&app, entity);
            assert!(
                (support_height(kind) - 0.12..=support_height(kind) + 0.12).contains(&position.y),
                "{kind:?} did not settle on the floor: {position:?}"
            );
            assert!(
                app.world()
                    .resource::<ContactGraph>()
                    .contains(entity, floor),
                "{kind:?} did not produce a floor contact"
            );
            assert!(
                app.world()
                    .entity(entity)
                    .get::<LinearVelocity>()
                    .expect("linear velocity")
                    .length()
                    < 0.2,
                "{kind:?} kept moving after settling"
            );
        }
    }

    #[test]
    fn player_pushes_each_lightweight_object_without_tipping() {
        for kind in DynamicObjectKind::ALL {
            let mut app = physics_app(true);
            app.update();

            let objects = object_entities(&mut app);
            let target = objects
                .iter()
                .find_map(|&(entity, object_kind)| (object_kind == kind).then_some(entity))
                .expect("target object");

            for (entity, object_kind) in objects {
                let position = if entity == target {
                    Vec3::new(2.0, support_height(object_kind), 0.0)
                } else {
                    Vec3::new(8.0, support_height(object_kind), 8.0)
                };
                app.world_mut().entity_mut(entity).insert((
                    Position(position),
                    Transform::from_translation(position),
                    LinearVelocity::default(),
                    AngularVelocity::default(),
                ));
            }

            run_steps(&mut app, 120);
            let before_push = object_position(&app, target).x;

            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyD);
            run_steps(&mut app, 60);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::KeyD);
            run_steps(&mut app, 4);

            let after_push = object_position(&app, target).x;
            let player = {
                let world = app.world_mut();
                let mut players = world.query_filtered::<Entity, With<Player>>();
                players.iter(world).next().expect("player")
            };
            let player_entity = app.world().entity(player);
            let player_position = player_entity
                .get::<Transform>()
                .expect("player transform")
                .translation;
            assert!(
                after_push > before_push + 0.1,
                "{kind:?} did not respond to the player: {before_push} -> {after_push}; player at {player_position:?}"
            );
            assert!(player_position.y > PLAYER_START_POSITION.y - 0.15);
            assert_eq!(
                player_entity.get::<Transform>().unwrap().rotation,
                Quat::IDENTITY,
                "player tipped while pushing {kind:?}"
            );
        }
    }
}
