use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::{ResetStation, StationObject};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// The body types compared by station A.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum RigidBodyLabKind {
    Static,
    Dynamic,
    MovingKinematic,
}

impl RigidBodyLabKind {
    const ALL: [Self; 3] = [Self::Static, Self::Dynamic, Self::MovingKinematic];

    const fn label(self) -> &'static str {
        match self {
            Self::Static => "STATIC",
            Self::Dynamic => "DYNAMIC",
            Self::MovingKinematic => "MOVING KINEMATIC",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Static => "fixed · collision anchor",
            Self::Dynamic => "gravity · responds to impacts",
            Self::MovingKinematic => "scripted motion · carries bodies",
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Static => Color::srgb(0.35, 0.65, 0.95),
            Self::Dynamic => Color::srgb(0.95, 0.42, 0.25),
            Self::MovingKinematic => Color::srgb(0.35, 0.9, 0.5),
        }
    }
}

/// Marks one of station A's three labelled body-type demonstrations.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RigidBodyLabObject {
    pub kind: RigidBodyLabKind,
}

/// Motion state for the station's position-driven kinematic platform.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MovingKinematicPlatform {
    pub start: Vec3,
    pub end: Vec3,
    pub speed: f32,
    pub direction: f32,
}

impl MovingKinematicPlatform {
    const fn new(start: Vec3, end: Vec3, speed: f32) -> Self {
        Self {
            start,
            end,
            speed,
            direction: 1.0,
        }
    }

    fn target(self) -> Vec3 {
        if self.direction >= 0.0 {
            self.end
        } else {
            self.start
        }
    }

    fn travel_direction(self) -> Vec3 {
        (self.target() - self.start_or_end()).normalize_or_zero()
    }

    fn start_or_end(self) -> Vec3 {
        if self.direction >= 0.0 {
            self.start
        } else {
            self.end
        }
    }

    fn reverse(&mut self) {
        self.direction = -self.direction;
    }
}

/// Owns station A's static, dynamic, and moving-kinematic comparison.
pub struct RigidBodyStationPlugin;

impl Plugin for RigidBodyStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_rigid_body_lab, spawn_rigid_body_station_ui))
            .add_systems(
                Update,
                (
                    reset_rigid_body_lab,
                    queue_rigid_body_station_actions,
                    update_rigid_body_button_colors,
                    draw_rigid_body_labels.run_if(resource_exists::<GizmoConfigStore>),
                ),
            )
            .add_systems(FixedUpdate, move_kinematic_platform);
    }
}

const STATIC_POSITION: Vec3 = Vec3::new(-7.25, 0.7, -5.25);
const PLATFORM_START: Vec3 = Vec3::new(-7.1, 0.55, -6.75);
const PLATFORM_END: Vec3 = Vec3::new(-4.35, 0.55, -6.75);
const PLATFORM_SIZE: Vec3 = Vec3::new(1.35, 0.3, 0.9);
const PLATFORM_SPEED: f32 = 1.7;
const RIDER_POSITION: Vec3 = Vec3::new(-7.1, 1.1, -6.75);
const BODY_SIZE: f32 = 0.8;

fn spawn_rigid_body_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_length(1.0)));

    let static_transform = Transform::from_translation(STATIC_POSITION);
    let static_material = materials
        .as_mut()
        .map(|materials| materials.add(RigidBodyLabKind::Static.color()));
    let mut static_body = commands.spawn((
        RigidBodyLabObject {
            kind: RigidBodyLabKind::Static,
        },
        StationObject::new('A', static_transform),
        RigidBody::Static,
        Collider::cuboid(BODY_SIZE, BODY_SIZE, BODY_SIZE),
        static_transform,
        Name::new("Rigid Body Lab - Static"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), static_material.as_ref()) {
        static_body.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }

    let rider_transform = Transform::from_translation(RIDER_POSITION);
    let rider_material = materials
        .as_mut()
        .map(|materials| materials.add(RigidBodyLabKind::Dynamic.color()));
    let mut rider = commands.spawn((
        RigidBodyLabObject {
            kind: RigidBodyLabKind::Dynamic,
        },
        StationObject::new('A', rider_transform),
        RigidBody::Dynamic,
        Collider::cuboid(BODY_SIZE, BODY_SIZE, BODY_SIZE),
        ColliderDensity(1.0),
        SleepingDisabled,
        rider_transform,
        Name::new("Rigid Body Lab - Dynamic Rider"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), rider_material.as_ref()) {
        rider.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }

    let platform_transform = Transform::from_translation(PLATFORM_START);
    let platform_material = materials
        .as_mut()
        .map(|materials| materials.add(RigidBodyLabKind::MovingKinematic.color()));
    let mut platform = commands.spawn((
        RigidBodyLabObject {
            kind: RigidBodyLabKind::MovingKinematic,
        },
        MovingKinematicPlatform::new(PLATFORM_START, PLATFORM_END, PLATFORM_SPEED),
        StationObject::new('A', platform_transform),
        RigidBody::Kinematic,
        Collider::cuboid(PLATFORM_SIZE.x, PLATFORM_SIZE.y, PLATFORM_SIZE.z),
        LinearVelocity::ZERO,
        platform_transform,
        Name::new("Rigid Body Lab - Moving Kinematic Platform"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), platform_material.as_ref()) {
        platform.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }
}

fn move_kinematic_platform(
    time: Res<Time>,
    mut platforms: Query<(
        &mut MovingKinematicPlatform,
        &mut Position,
        &mut LinearVelocity,
    )>,
) {
    let delta_secs = time.delta_secs();
    for (mut platform, position, mut velocity) in &mut platforms {
        let current = position.0;
        let target = platform.target();
        let offset = target - current;
        let distance = offset.length();
        let step = platform.speed * delta_secs;

        if distance <= step {
            platform.reverse();
            velocity.0 = platform.travel_direction() * platform.speed;
        } else {
            velocity.0 = offset / distance * platform.speed;
        }
    }
}

fn reset_rigid_body_lab(
    mut requests: MessageReader<ResetStation>,
    mut platforms: Query<&mut MovingKinematicPlatform>,
) {
    if !requests.read().any(|request| request.code == 'A') {
        return;
    }

    for mut platform in &mut platforms {
        platform.direction = 1.0;
    }
}

#[derive(Component)]
struct RigidBodyStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum RigidBodyStationControl {
    Reset,
}

fn spawn_rigid_body_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(16.0),
                width: px(390.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            RigidBodyStationPanel,
            Name::new("Rigid Body Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("A — RIGID-BODY TYPES"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Collide with the fixed body, the dynamic rider, and the repeatedly moving platform to compare their responses."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for kind in RigidBodyLabKind::ALL {
                parent.spawn((
                    Text::new(format!("{}    {}", kind.label(), kind.description())),
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
                    RigidBodyStationControl::Reset,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Rigid body station reset"),
                ))
                .with_child((
                    Text::new("reset station A"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_rigid_body_station_actions(
    controls: Query<(&Interaction, &RigidBodyStationControl), Changed<Interaction>>,
    mut resets: MessageWriter<ResetStation>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed && *control == RigidBodyStationControl::Reset {
            resets.write(ResetStation { code: 'A' });
        }
    }
}

fn update_rigid_body_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<RigidBodyStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn draw_rigid_body_labels(mut gizmos: Gizmos, bodies: Query<(&RigidBodyLabObject, &Transform)>) {
    for (body, transform) in &bodies {
        let height = match body.kind {
            RigidBodyLabKind::MovingKinematic => 1.0,
            _ => 0.7,
        };
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * height, Quat::IDENTITY),
            body.kind.label(),
            0.48,
            Vec2::ZERO,
            body.kind.color(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn station_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            RigidBodyStationPlugin,
        ))
        .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
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

    fn body(app: &mut App, kind: RigidBodyLabKind) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query::<(Entity, &RigidBodyLabObject)>();
        bodies
            .iter(world)
            .find_map(|(entity, object)| (object.kind == kind).then_some(entity))
            .expect("rigid-body laboratory object")
    }

    fn position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("rigid-body laboratory position")
            .0
    }

    #[test]
    fn station_spawns_one_labelled_body_of_each_type() {
        let mut app = station_app();
        app.update();

        let world = app.world_mut();
        let mut bodies = world.query::<(&RigidBodyLabObject, &RigidBody, &Name)>();
        let bodies: Vec<_> = bodies.iter(world).collect();
        assert_eq!(bodies.len(), RigidBodyLabKind::ALL.len());
        for (body, rigid_body, name) in bodies {
            assert!(
                name.as_str()
                    .to_ascii_uppercase()
                    .contains(body.kind.label()),
                "body name does not identify its type: {}",
                name.as_str()
            );
            match body.kind {
                RigidBodyLabKind::Static => assert_eq!(*rigid_body, RigidBody::Static),
                RigidBodyLabKind::Dynamic => assert_eq!(*rigid_body, RigidBody::Dynamic),
                RigidBodyLabKind::MovingKinematic => {
                    assert_eq!(*rigid_body, RigidBody::Kinematic)
                }
            }
        }
    }

    #[test]
    fn moving_kinematic_platform_reaches_both_endpoints_and_reverses() {
        let mut app = station_app();
        app.update();
        let platform = body(&mut app, RigidBodyLabKind::MovingKinematic);
        let start = position(&app, platform);

        let mut positions = Vec::new();
        for _ in 0..220 {
            run_steps(&mut app, 1);
            positions.push(position(&app, platform));
        }

        let farthest = positions
            .iter()
            .copied()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .expect("platform samples");
        let nearest = positions
            .iter()
            .copied()
            .min_by(|a, b| a.x.total_cmp(&b.x))
            .expect("platform samples");
        assert!(
            farthest.distance(PLATFORM_END) < 0.12,
            "platform did not reach end: {farthest:?}"
        );
        assert!(
            nearest.distance(PLATFORM_START) < 0.12,
            "platform did not return to start: {nearest:?}"
        );
        assert!(farthest.distance(start) > 2.0);
    }

    #[test]
    fn dynamic_rider_is_supported_by_and_moved_with_the_platform() {
        let mut app = station_app();
        app.update();
        let platform = body(&mut app, RigidBodyLabKind::MovingKinematic);
        let rider = body(&mut app, RigidBodyLabKind::Dynamic);
        let rider_start = position(&app, rider);

        run_steps(&mut app, 70);

        let rider_position = position(&app, rider);
        let platform_position = position(&app, platform);
        assert!(
            rider_position.x > rider_start.x + 0.5,
            "dynamic rider was not carried: {rider_start:?} -> {rider_position:?}"
        );
        assert!(
            (rider_position.y - rider_start.y).abs() < 0.3,
            "rider fell through the moving platform: {rider_position:?}"
        );
        assert!(platform_position.x > rider_start.x + 1.0);
    }

    #[test]
    fn static_body_stays_fixed_while_a_dynamic_body_responds_to_its_collision() {
        let mut app = station_app();
        app.update();
        let static_body = body(&mut app, RigidBodyLabKind::Static);
        let dynamic_body = body(&mut app, RigidBodyLabKind::Dynamic);
        let static_start = position(&app, static_body);

        app.world_mut().resource_mut::<Gravity>().0 = Vec3::ZERO;
        app.world_mut().entity_mut(dynamic_body).insert((
            Position(static_start + Vec3::new(-2.0, 0.0, 0.0)),
            Transform::from_translation(static_start + Vec3::new(-2.0, 0.0, 0.0)),
            LinearVelocity(Vec3::X * 8.0),
        ));
        run_steps(&mut app, 45);

        let static_position = position(&app, static_body);
        let dynamic_position = position(&app, dynamic_body);
        assert!(static_position.distance(static_start) < 0.001);
        assert!(
            dynamic_position.x > static_start.x - 2.0 + 0.3,
            "dynamic body did not advance toward the static collision: {dynamic_position:?}"
        );
        assert!(
            app.world()
                .resource::<ContactGraph>()
                .contains(static_body, dynamic_body),
            "static and dynamic bodies never formed a contact: static={static_position:?}, dynamic={dynamic_position:?}, velocity={:?}",
            app.world()
                .entity(dynamic_body)
                .get::<LinearVelocity>()
                .map(|velocity| velocity.0)
        );
    }
}
