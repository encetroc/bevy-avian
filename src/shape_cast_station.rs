use avian3d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

use crate::collision_layers::{SandboxLayer, layers_for};
use crate::spatial_query_filters::{SpatialQueryFilterSet, SpatialQueryFilterState};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const PATH_COLOR: Color = Color::srgb(0.15, 0.85, 1.0);
const HIT_COLOR: Color = Color::srgb(1.0, 0.65, 0.15);
const NORMAL_COLOR: Color = Color::srgb(0.3, 1.0, 0.4);
const EMPTY_COLOR: Color = Color::srgb(0.55, 0.65, 0.75);
const SHAPE_COLOR: Color = Color::srgba(0.35, 0.7, 1.0, 0.75);
const SHAPE_RADIUS: f32 = 0.38;
const CAPSULE_LENGTH: f32 = 0.9;
const CAST_DISTANCE: f32 = 5.0;
const BLOCKED_START: Vec3 = Vec3::new(-8.0, 1.0, 5.0);
const EMPTY_START: Vec3 = Vec3::new(-8.0, 1.0, 6.9);
const CAST_DIRECTION: Dir3 = Dir3::X;
const OBSTACLE_POSITION: Vec3 = Vec3::new(-6.5, 1.0, 5.0);
const WORLD_OBSTACLE_POSITION: Vec3 = Vec3::new(-5.0, 1.0, 5.0);
const OBSTACLE_SIZE: Vec3 = Vec3::new(0.55, 1.6, 1.4);

/// The two primitive shapes supported by the interactive sweep demonstration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShapeCastKind {
    #[default]
    Sphere,
    Capsule,
}

impl ShapeCastKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sphere => "SPHERE",
            Self::Capsule => "CAPSULE",
        }
    }

    pub fn collider(self) -> Collider {
        match self {
            Self::Sphere => Collider::sphere(SHAPE_RADIUS),
            Self::Capsule => Collider::capsule(SHAPE_RADIUS, CAPSULE_LENGTH),
        }
    }
}

/// The two deterministic paths used to compare a hit with empty space.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShapeCastScenario {
    #[default]
    Obstacle,
    Empty,
}

impl ShapeCastScenario {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Obstacle => "OBSTACLE",
            Self::Empty => "EMPTY SPACE",
        }
    }

    pub const fn start(self) -> Vec3 {
        match self {
            Self::Obstacle => BLOCKED_START,
            Self::Empty => EMPTY_START,
        }
    }
}

/// Current shape and path selected by the station controls.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ShapeCastDemoState {
    pub shape: ShapeCastKind,
    pub scenario: ShapeCastScenario,
}

impl ShapeCastDemoState {
    pub const fn start(self) -> Vec3 {
        self.scenario.start()
    }

    pub const fn direction(self) -> Dir3 {
        CAST_DIRECTION
    }
}

/// Result of the latest first-hit shape cast.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ShapeCastDemoResult {
    pub hit: Option<ShapeHitData>,
}

/// Headless-friendly controls for the interactive shape-cast station.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeCastAction {
    SelectShape(ShapeCastKind),
    SelectScenario(ShapeCastScenario),
}

/// Marks the obstacle used by station H's interactive shape-cast demonstration.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapeCastDemoObstacle;

/// Marks the second obstacle used to prove that a layer exclusion changes the hit.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapeCastDemoWorldObstacle;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum ShapeCastControl {
    Shape(ShapeCastKind),
    Scenario(ShapeCastScenario),
}

#[derive(Component)]
struct ShapeCastDemoPanel;

#[derive(Component)]
struct ShapeCastDemoStatus;

/// Owns the one-shot sphere and capsule sweep demonstration.
pub struct ShapeCastStationPlugin;

impl Plugin for ShapeCastStationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShapeCastDemoState>()
            .init_resource::<ShapeCastDemoResult>()
            .init_resource::<SpatialQueryFilterState>()
            .add_message::<ShapeCastAction>()
            .add_systems(
                Startup,
                (spawn_shape_cast_demo, spawn_shape_cast_station_ui),
            )
            .add_systems(
                PreUpdate,
                queue_keyboard_shape_cast_actions.after(InputSystems),
            )
            .add_systems(
                Update,
                (
                    queue_shape_cast_actions,
                    apply_shape_cast_actions,
                    update_shape_cast_result,
                    update_shape_cast_ui,
                    draw_shape_cast_demo.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain()
                    .after(SpatialQueryFilterSet::ApplyControls),
            );
    }
}

fn spawn_shape_cast_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.9, 0.3, 0.18)));

    let mut obstacle = commands.spawn((
        ShapeCastDemoObstacle,
        RigidBody::Static,
        layers_for(SandboxLayer::Objects),
        Collider::cuboid(OBSTACLE_SIZE.x, OBSTACLE_SIZE.y, OBSTACLE_SIZE.z),
        Transform::from_translation(OBSTACLE_POSITION),
        Name::new("Shape Cast Objects Obstacle"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), material.as_ref()) {
        obstacle.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(OBSTACLE_POSITION).with_scale(OBSTACLE_SIZE),
        ));
    }

    let mut world_obstacle = commands.spawn((
        ShapeCastDemoWorldObstacle,
        RigidBody::Static,
        layers_for(SandboxLayer::World),
        Collider::cuboid(OBSTACLE_SIZE.x, OBSTACLE_SIZE.y, OBSTACLE_SIZE.z),
        Transform::from_translation(WORLD_OBSTACLE_POSITION),
        Name::new("Shape Cast World Obstacle"),
    ));
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), material.as_ref()) {
        world_obstacle.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(WORLD_OBSTACLE_POSITION).with_scale(OBSTACLE_SIZE),
        ));
    }
}

fn spawn_shape_cast_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(16.0),
                width: px(330.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            ShapeCastDemoPanel,
            Name::new("Shape Cast Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("H — INTERACTIVE SHAPE CASTS"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Sweep a sphere or capsule along a fixed path."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Shape: SPHERE"),
                TextFont::from_font_size(13.0),
                TextColor(PATH_COLOR),
                ShapeCastDemoStatus,
            ));
            parent.spawn((
                Text::new("Scenario: OBSTACLE"),
                TextFont::from_font_size(13.0),
                TextColor(PATH_COLOR),
                ShapeCastDemoStatus,
            ));
            spawn_control_row(
                parent,
                [
                    ("sphere [1]", ShapeCastControl::Shape(ShapeCastKind::Sphere)),
                    (
                        "capsule [2]",
                        ShapeCastControl::Shape(ShapeCastKind::Capsule),
                    ),
                ],
            );
            spawn_control_row(
                parent,
                [
                    (
                        "obstacle [B]",
                        ShapeCastControl::Scenario(ShapeCastScenario::Obstacle),
                    ),
                    (
                        "empty [E]",
                        ShapeCastControl::Scenario(ShapeCastScenario::Empty),
                    ),
                ],
            );
            parent.spawn((
                Text::new("Cyan path = sweep · orange cross = first hit · green arrow = normal"),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));
        });
}

fn spawn_control_row<const N: usize>(
    parent: &mut ChildSpawnerCommands,
    controls: [(&'static str, ShapeCastControl); N],
) {
    parent
        .spawn(Node {
            width: percent(100.0),
            height: px(25.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            for (label, control) in controls {
                row.spawn((
                    Button,
                    control,
                    Node {
                        width: px(140.0),
                        height: px(22.0),
                        margin: UiRect::horizontal(px(2.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new(format!("Shape cast {label}")),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
            }
        });
}

fn queue_shape_cast_actions(
    controls: Query<(&Interaction, &ShapeCastControl), Changed<Interaction>>,
    mut actions: MessageWriter<ShapeCastAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let action = match control {
            ShapeCastControl::Shape(shape) => ShapeCastAction::SelectShape(*shape),
            ShapeCastControl::Scenario(scenario) => ShapeCastAction::SelectScenario(*scenario),
        };
        actions.write(action);
    }
}

fn queue_keyboard_shape_cast_actions(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut actions: MessageWriter<ShapeCastAction>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        actions.write(ShapeCastAction::SelectShape(ShapeCastKind::Sphere));
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        actions.write(ShapeCastAction::SelectShape(ShapeCastKind::Capsule));
    }
    if keyboard.just_pressed(KeyCode::KeyB) {
        actions.write(ShapeCastAction::SelectScenario(ShapeCastScenario::Obstacle));
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        actions.write(ShapeCastAction::SelectScenario(ShapeCastScenario::Empty));
    }
}

fn apply_shape_cast_actions(
    mut actions: MessageReader<ShapeCastAction>,
    mut state: ResMut<ShapeCastDemoState>,
) {
    for action in actions.read() {
        match action {
            ShapeCastAction::SelectShape(shape) => state.shape = *shape,
            ShapeCastAction::SelectScenario(scenario) => state.scenario = *scenario,
        }
    }
}

fn update_shape_cast_result(
    spatial_query: SpatialQuery,
    state: Res<ShapeCastDemoState>,
    filter_state: Res<SpatialQueryFilterState>,
    mut result: ResMut<ShapeCastDemoResult>,
) {
    let hit = spatial_query.cast_shape(
        &state.shape.collider(),
        state.start(),
        Quat::IDENTITY,
        state.direction(),
        &ShapeCastConfig::from_max_distance(CAST_DISTANCE),
        &filter_state.query_filter(),
    );
    result.hit = hit;
}

fn update_shape_cast_ui(
    state: Res<ShapeCastDemoState>,
    mut statuses: Query<&mut Text, With<ShapeCastDemoStatus>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<ShapeCastControl>>,
) {
    let mut status = statuses.iter_mut();
    if let Some(mut shape_status) = status.next() {
        shape_status.0 = format!("Shape: {}", state.shape.label());
    }
    if let Some(mut scenario_status) = status.next() {
        scenario_status.0 = format!("Scenario: {}", state.scenario.label());
    }

    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn draw_shape_cast_demo(
    mut gizmos: Gizmos,
    state: Res<ShapeCastDemoState>,
    result: Res<ShapeCastDemoResult>,
    names: Query<&Name>,
) {
    let start = state.start();
    let end = start + state.direction().as_vec3() * CAST_DISTANCE;
    let hit_distance = result.hit.as_ref().map(|hit| hit.distance);
    let path_end = hit_distance
        .map(|distance| start + state.direction().as_vec3() * distance)
        .unwrap_or(end);

    gizmos.line(start, path_end, PATH_COLOR);
    if hit_distance.is_some() {
        gizmos.line(path_end, end, EMPTY_COLOR);
    }
    draw_shape_gizmo(&mut gizmos, state.shape, start, SHAPE_COLOR);
    draw_shape_gizmo(&mut gizmos, state.shape, end, EMPTY_COLOR);
    gizmos.sphere(start, 0.08, PATH_COLOR);
    gizmos.sphere(end, 0.08, EMPTY_COLOR);

    let Some(hit) = result.hit.as_ref() else {
        gizmos.text(
            Isometry3d::new(end + Vec3::Y * 0.65, Quat::IDENTITY),
            &format!("{} SWEEP\nNO HIT", state.shape.label()),
            0.28,
            Vec2::ZERO,
            PATH_COLOR,
        );
        return;
    };

    gizmos.cross(hit.point1, 0.14, HIT_COLOR);
    gizmos.arrow(hit.point1, hit.point1 + hit.normal1 * 0.75, NORMAL_COLOR);
    let name = names
        .get(hit.entity)
        .map(Name::as_str)
        .unwrap_or("Unknown collider");
    let label = format!(
        "{} SWEEP HIT\n{name}\nDistance: {:.2} m\nPoint: ({:.2}, {:.2}, {:.2})\nNormal: ({:.2}, {:.2}, {:.2})",
        state.shape.label(),
        hit.distance,
        hit.point1.x,
        hit.point1.y,
        hit.point1.z,
        hit.normal1.x,
        hit.normal1.y,
        hit.normal1.z,
    );
    gizmos.text(
        Isometry3d::new(hit.point1 + Vec3::Y * 0.9, Quat::IDENTITY),
        &label,
        0.28,
        Vec2::ZERO,
        HIT_COLOR,
    );
}

fn draw_shape_gizmo(gizmos: &mut Gizmos, shape: ShapeCastKind, center: Vec3, color: Color) {
    match shape {
        ShapeCastKind::Sphere => {
            gizmos.sphere(center, SHAPE_RADIUS, color);
        }
        ShapeCastKind::Capsule => {
            let half_length = CAPSULE_LENGTH * 0.5;
            gizmos.line(
                center - Vec3::Y * half_length,
                center + Vec3::Y * half_length,
                color,
            );
            gizmos.sphere(center - Vec3::Y * half_length, SHAPE_RADIUS, color);
            gizmos.sphere(center + Vec3::Y * half_length, SHAPE_RADIUS, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn shape_cast_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            ShapeCastStationPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn run_physics_frame(app: &mut App) {
        app.update();
        app.update();
    }

    fn obstacle_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<ShapeCastDemoObstacle>>()
            .iter(world)
            .next()
            .expect("shape-cast obstacle")
    }

    #[test]
    fn station_starts_with_a_sphere_sweep_and_visible_obstacle() {
        let mut app = shape_cast_app();
        run_physics_frame(&mut app);

        assert_eq!(
            app.world().resource::<ShapeCastDemoState>().shape,
            ShapeCastKind::Sphere
        );
        assert_eq!(
            app.world().resource::<ShapeCastDemoState>().scenario,
            ShapeCastScenario::Obstacle
        );
        assert_eq!(
            app.world()
                .resource::<ShapeCastDemoResult>()
                .hit
                .as_ref()
                .map(|hit| hit.entity),
            Some(obstacle_entity(&mut app))
        );
    }

    #[test]
    fn obstacle_sweep_reports_first_hit_distance_and_normal_for_both_shapes() {
        let mut app = shape_cast_app();
        run_physics_frame(&mut app);
        let obstacle = obstacle_entity(&mut app);

        for shape in [ShapeCastKind::Sphere, ShapeCastKind::Capsule] {
            app.world_mut()
                .resource_mut::<Messages<ShapeCastAction>>()
                .write(ShapeCastAction::SelectShape(shape));
            run_physics_frame(&mut app);

            let hit = app
                .world()
                .resource::<ShapeCastDemoResult>()
                .hit
                .expect("shape should hit the obstacle");
            assert_eq!(hit.entity, obstacle);
            assert!(hit.distance > 0.0 && hit.distance < CAST_DISTANCE);
            assert!(
                hit.normal1.x < -0.9,
                "expected obstacle's left normal: {hit:?}"
            );
            assert!((hit.point1.x - (OBSTACLE_POSITION.x - OBSTACLE_SIZE.x * 0.5)).abs() < 0.01);
        }
    }

    #[test]
    fn excluded_layers_cannot_become_the_reported_hit() {
        let mut app = shape_cast_app();
        run_physics_frame(&mut app);
        let objects_obstacle = obstacle_entity(&mut app);
        let world_obstacle = app
            .world_mut()
            .query_filtered::<Entity, With<ShapeCastDemoWorldObstacle>>()
            .iter(app.world())
            .next()
            .expect("world shape-cast obstacle");

        app.world_mut()
            .resource_mut::<SpatialQueryFilterState>()
            .mask = LayerMask::from(SandboxLayer::World);
        run_physics_frame(&mut app);
        assert_eq!(
            app.world()
                .resource::<ShapeCastDemoResult>()
                .hit
                .as_ref()
                .map(|hit| hit.entity),
            Some(world_obstacle)
        );
        assert_ne!(objects_obstacle, world_obstacle);

        app.world_mut()
            .resource_mut::<SpatialQueryFilterState>()
            .mask = LayerMask::NONE;
        run_physics_frame(&mut app);
        assert!(app.world().resource::<ShapeCastDemoResult>().hit.is_none());
    }

    #[test]
    fn empty_sweep_reports_no_hit_for_both_shapes() {
        let mut app = shape_cast_app();
        run_physics_frame(&mut app);

        app.world_mut()
            .resource_mut::<Messages<ShapeCastAction>>()
            .write(ShapeCastAction::SelectScenario(ShapeCastScenario::Empty));
        run_physics_frame(&mut app);
        assert!(app.world().resource::<ShapeCastDemoResult>().hit.is_none());

        for shape in [ShapeCastKind::Sphere, ShapeCastKind::Capsule] {
            app.world_mut()
                .resource_mut::<Messages<ShapeCastAction>>()
                .write(ShapeCastAction::SelectShape(shape));
            run_physics_frame(&mut app);
            assert!(app.world().resource::<ShapeCastDemoResult>().hit.is_none());
        }
    }

    #[test]
    fn controls_change_shape_and_scenario_without_restarting_the_station() {
        let mut app = shape_cast_app();
        run_physics_frame(&mut app);

        app.world_mut()
            .resource_mut::<Messages<ShapeCastAction>>()
            .write(ShapeCastAction::SelectShape(ShapeCastKind::Capsule));
        app.world_mut()
            .resource_mut::<Messages<ShapeCastAction>>()
            .write(ShapeCastAction::SelectScenario(ShapeCastScenario::Empty));
        run_physics_frame(&mut app);

        let state = app.world().resource::<ShapeCastDemoState>();
        assert_eq!(state.shape, ShapeCastKind::Capsule);
        assert_eq!(state.scenario, ShapeCastScenario::Empty);
        assert!(app.world().resource::<ShapeCastDemoResult>().hit.is_none());
    }
}
