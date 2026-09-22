use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::StationObject;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

const SURFACE_WIDTH: f32 = 1.0;
const SURFACE_DEPTH: f32 = 1.15;
const SURFACE_THICKNESS: f32 = 0.12;
const SURFACE_CENTER_Y: f32 = 0.06;
const SURFACE_Z: f32 = -3.7;
const BALL_RADIUS: f32 = 0.24;
const BALL_START_Y: f32 = 4.5;

/// The three bounce surfaces compared by station B.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RestitutionSurfaceKind {
    Low,
    Medium,
    High,
}

impl RestitutionSurfaceKind {
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        }
    }

    pub const fn coefficient(self) -> f32 {
        match self {
            Self::Low => 0.10,
            Self::Medium => 0.50,
            Self::High => 0.90,
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Low => Color::srgb(0.45, 0.48, 0.54),
            Self::Medium => Color::srgb(0.76, 0.43, 0.18),
            Self::High => Color::srgb(0.88, 0.2, 0.22),
        }
    }

    const fn lane_x(self) -> f32 {
        match self {
            Self::Low => -1.35,
            Self::Medium => 0.0,
            Self::High => 1.35,
        }
    }
}

/// Marks one static surface in the restitution comparison.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestitutionSurface {
    pub kind: RestitutionSurfaceKind,
}

/// Marks the identical ball dropped onto one restitution surface.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestitutionLabObject {
    pub surface: RestitutionSurfaceKind,
}

/// Requests a repeatable restitution demonstration action.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestitutionLabAction {
    Drop,
    Reset,
}

/// Owns station B's low, medium, and high restitution surfaces and matching balls.
pub struct RestitutionStationPlugin;

impl Plugin for RestitutionStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RestitutionLabAction>()
            .add_systems(
                Startup,
                (spawn_restitution_lab, spawn_restitution_station_ui),
            )
            .add_systems(
                Update,
                (
                    queue_restitution_station_actions,
                    update_restitution_button_colors,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                draw_restitution_labels.run_if(resource_exists::<GizmoConfigStore>),
            )
            .add_systems(PostUpdate, reset_restitution_lab);
    }
}

fn surface_transform(kind: RestitutionSurfaceKind) -> Transform {
    Transform::from_xyz(kind.lane_x(), SURFACE_CENTER_Y, SURFACE_Z)
}

fn ball_transform(kind: RestitutionSurfaceKind) -> Transform {
    Transform::from_xyz(kind.lane_x(), BALL_START_Y, SURFACE_Z)
}

fn spawn_restitution_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    for kind in RestitutionSurfaceKind::ALL {
        let surface_transform = surface_transform(kind);
        let surface_restitution =
            Restitution::new(kind.coefficient()).with_combine_rule(CoefficientCombine::Max);
        let surface_material = materials
            .as_mut()
            .map(|materials| materials.add(kind.color()));
        let mut surface = commands.spawn((
            RestitutionSurface { kind },
            RigidBody::Static,
            Collider::cuboid(SURFACE_WIDTH, SURFACE_THICKNESS, SURFACE_DEPTH),
            // Max makes the surface coefficient win over the zero-restitution ball.
            surface_restitution,
            surface_transform,
            Name::new(format!("Restitution Surface - {}", kind.label())),
        ));
        if let (Some(meshes), Some(material)) = (meshes.as_mut(), surface_material.as_ref()) {
            surface.insert((
                Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(
                    SURFACE_WIDTH,
                    SURFACE_THICKNESS,
                    SURFACE_DEPTH,
                )))),
                MeshMaterial3d(material.clone()),
            ));
        }

        let ball_transform = ball_transform(kind);
        let ball_material = materials
            .as_mut()
            .map(|materials| materials.add(kind.color().with_luminance(0.8)));
        let mut ball = commands.spawn((
            RestitutionLabObject { surface: kind },
            StationObject::new('B', ball_transform),
            RigidBody::Dynamic,
            Collider::sphere(BALL_RADIUS),
            // The identical balls contribute no restitution or friction of their own.
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Max),
            Friction::ZERO.with_combine_rule(CoefficientCombine::Max),
            LinearDamping(0.0),
            AngularDamping(0.0),
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            SleepingDisabled,
            ball_transform,
            Name::new(format!("Restitution Test Ball - {}", kind.label())),
        ));
        if let (Some(meshes), Some(material)) = (meshes.as_mut(), ball_material.as_ref()) {
            ball.insert((
                Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
                MeshMaterial3d(material.clone()),
            ));
        }
    }
}

#[derive(Component)]
struct RestitutionStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum RestitutionStationControl {
    Drop,
    Reset,
}

fn spawn_restitution_station_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(422.0),
                top: px(16.0),
                width: px(330.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            RestitutionStationPanel,
            Name::new("Restitution Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("B — RESTITUTION LABORATORY"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    "Drop identical balls together to compare rebound heights. Reset before each trial for repeatable results.",
                ),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for kind in RestitutionSurfaceKind::ALL {
                parent.spawn((
                    Text::new(format!(
                        "{:<6} restitution coefficient  {:.2}",
                        kind.label(),
                        kind.coefficient()
                    )),
                    TextFont::from_font_size(13.0),
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
                    RestitutionStationControl::Drop,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Restitution station drop"),
                ))
                .with_child((
                    Text::new("drop balls"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
            parent
                .spawn((
                    Button,
                    RestitutionStationControl::Reset,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Restitution station reset"),
                ))
                .with_child((
                    Text::new("reset station B"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_restitution_station_actions(
    controls: Query<(&Interaction, &RestitutionStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<RestitutionLabAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match control {
            RestitutionStationControl::Drop => RestitutionLabAction::Drop,
            RestitutionStationControl::Reset => RestitutionLabAction::Reset,
        });
    }
}

fn update_restitution_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<RestitutionStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

type RestitutionObjectQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static StationObject,
        &'static mut Transform,
        &'static mut Position,
        &'static mut Rotation,
        &'static mut LinearVelocity,
        &'static mut AngularVelocity,
    ),
    With<RestitutionLabObject>,
>;

fn reset_restitution_lab(
    mut actions: MessageReader<RestitutionLabAction>,
    mut objects: RestitutionObjectQuery,
) {
    if !actions.read().any(|_| true) {
        return;
    }

    for (
        initial,
        mut transform,
        mut position,
        mut rotation,
        mut linear_velocity,
        mut angular_velocity,
    ) in &mut objects
    {
        *transform = initial.initial_transform;
        position.0 = initial.initial_transform.translation;
        rotation.0 = initial.initial_transform.rotation;
        linear_velocity.0 = initial.initial_linear_velocity;
        angular_velocity.0 = initial.initial_angular_velocity;
    }
}

fn draw_restitution_labels(
    mut gizmos: Gizmos,
    surfaces: Query<(&RestitutionSurface, &Transform)>,
    objects: Query<(&RestitutionLabObject, &Transform)>,
) {
    for (surface, transform) in &surfaces {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.42, Quat::IDENTITY),
            &format!(
                "{}\ne {:.2}",
                surface.kind.label(),
                surface.kind.coefficient()
            ),
            0.42,
            Vec2::ZERO,
            surface.kind.color(),
        );
    }

    for (object, transform) in &objects {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.3, Quat::IDENTITY),
            object.surface.label(),
            0.28,
            Vec2::ZERO,
            object.surface.color(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::time::Duration;

    use bevy::{
        gizmos::GizmoPlugin,
        input::{
            ButtonState, InputPlugin,
            keyboard::{Key, KeyboardInput},
        },
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::{
        collider_debug::{ColliderDebugPlugin, ColliderDebugSettings},
        stations::StationLayoutPlugin,
    };

    const PHYSICS_STEP: f32 = 1.0 / 60.0;
    const SURFACE_TOP: f32 = SURFACE_CENTER_Y + SURFACE_THICKNESS * 0.5;
    const BALL_SUPPORT_Y: f32 = SURFACE_TOP + BALL_RADIUS;

    fn restitution_app() -> App {
        restitution_app_with_debug(false)
    }

    fn restitution_app_with_debug(debug: bool) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            RestitutionStationPlugin,
        ));
        if debug {
            app.add_plugins((GizmoPlugin, ColliderDebugPlugin));
        }
        app.insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
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

    fn object_entities(app: &mut App) -> HashMap<RestitutionSurfaceKind, Entity> {
        let world = app.world_mut();
        let mut objects = world.query::<(Entity, &RestitutionLabObject)>();
        objects
            .iter(world)
            .map(|(entity, object)| (object.surface, entity))
            .collect()
    }

    fn position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("restitution ball position")
            .0
    }

    fn drop_balls(app: &mut App) {
        app.world_mut()
            .resource_mut::<Messages<RestitutionLabAction>>()
            .write(RestitutionLabAction::Drop);
        app.update();
    }

    fn enable_collider_debug(app: &mut App) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::F1,
                logical_key: Key::F1,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::F1,
                logical_key: Key::F1,
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn first_bounce_peaks(app: &mut App) -> HashMap<RestitutionSurfaceKind, f32> {
        let entities = object_entities(app);
        let mut contacted = HashSet::new();
        let mut peaks = HashMap::new();

        for _ in 0..180 {
            run_steps(app, 1);
            for kind in RestitutionSurfaceKind::ALL {
                let y = position(app, entities[&kind]).y;
                if !contacted.contains(&kind) {
                    if y <= BALL_SUPPORT_Y + 0.04 {
                        contacted.insert(kind);
                    }
                } else {
                    peaks
                        .entry(kind)
                        .and_modify(|peak: &mut f32| *peak = peak.max(y))
                        .or_insert(y);
                }
            }
        }

        assert_eq!(contacted.len(), RestitutionSurfaceKind::ALL.len());
        assert_eq!(peaks.len(), RestitutionSurfaceKind::ALL.len());
        peaks
    }

    #[test]
    fn station_spawns_labelled_surfaces_and_identical_balls_with_distinct_restitution() {
        let mut app = restitution_app();
        app.update();

        let world = app.world_mut();
        let mut surfaces = world.query::<(&RestitutionSurface, &Restitution, &Name)>();
        let surfaces: Vec<_> = surfaces.iter(world).collect();
        assert_eq!(surfaces.len(), RestitutionSurfaceKind::ALL.len());
        for kind in RestitutionSurfaceKind::ALL {
            let (surface, restitution, name) = surfaces
                .iter()
                .find(|(surface, _, _)| surface.kind == kind)
                .expect("restitution surface");
            assert_eq!(surface.kind, kind);
            assert_eq!(restitution.coefficient, kind.coefficient());
            assert_eq!(restitution.combine_rule, CoefficientCombine::Max);
            assert!(name.as_str().contains(kind.label()));
        }

        let mut balls = world.query::<(
            &RestitutionLabObject,
            &Collider,
            &Restitution,
            &Friction,
            &Name,
        )>();
        let balls: Vec<_> = balls.iter(world).collect();
        assert_eq!(balls.len(), RestitutionSurfaceKind::ALL.len());
        for (ball, collider, restitution, friction, name) in balls {
            let ball_shape = collider.shape().as_ball().expect("sphere collider");
            assert!((ball_shape.radius - BALL_RADIUS).abs() < 0.001);
            assert_eq!(
                *restitution,
                Restitution::ZERO.with_combine_rule(CoefficientCombine::Max)
            );
            assert_eq!(
                *friction,
                Friction::ZERO.with_combine_rule(CoefficientCombine::Max)
            );
            assert!(name.as_str().contains(ball.surface.label()));
        }
    }

    #[test]
    fn matching_balls_are_dropped_from_the_same_height() {
        let mut app = restitution_app();
        app.update();
        let entities = object_entities(&mut app);
        let initial_y: Vec<_> = RestitutionSurfaceKind::ALL
            .into_iter()
            .map(|kind| position(&app, entities[&kind]).y)
            .collect();

        drop_balls(&mut app);

        let dropped_y: Vec<_> = RestitutionSurfaceKind::ALL
            .into_iter()
            .map(|kind| position(&app, entities[&kind]).y)
            .collect();
        assert!(
            initial_y
                .iter()
                .all(|height| (*height - BALL_START_Y).abs() < 0.001)
        );
        assert!(
            dropped_y
                .iter()
                .all(|height| (*height - BALL_START_Y).abs() < 0.001)
        );
        assert!(
            dropped_y
                .windows(2)
                .all(|pair| (pair[0] - pair[1]).abs() < 0.001)
        );
    }

    #[test]
    fn bounce_heights_are_ordered_by_restitution() {
        let mut app = restitution_app();
        app.update();
        drop_balls(&mut app);
        let peaks = first_bounce_peaks(&mut app);

        let low = peaks[&RestitutionSurfaceKind::Low];
        let medium = peaks[&RestitutionSurfaceKind::Medium];
        let high = peaks[&RestitutionSurfaceKind::High];
        assert!(
            low + 0.15 < medium,
            "low bounce {low} was not below medium bounce {medium}"
        );
        assert!(
            medium + 0.5 < high,
            "medium bounce {medium} was not below high bounce {high}"
        );
    }

    #[test]
    fn repeated_drops_produce_the_same_bounce_order_and_heights() {
        let mut trials = Vec::new();
        for _ in 0..3 {
            let mut app = restitution_app();
            app.update();
            drop_balls(&mut app);
            trials.push(first_bounce_peaks(&mut app));
        }

        for peaks in &trials {
            let low = peaks[&RestitutionSurfaceKind::Low];
            let medium = peaks[&RestitutionSurfaceKind::Medium];
            let high = peaks[&RestitutionSurfaceKind::High];
            assert!(low < medium && medium < high);
        }

        for pair in trials.windows(2) {
            for kind in RestitutionSurfaceKind::ALL {
                assert!(
                    (pair[0][&kind] - pair[1][&kind]).abs() < 0.02,
                    "{} bounce was not repeatable: {} vs {}",
                    kind.label(),
                    pair[0][&kind],
                    pair[1][&kind]
                );
            }
        }
    }

    #[test]
    fn reset_returns_all_balls_to_the_same_drop_height() {
        let mut app = restitution_app();
        app.update();
        let entities = object_entities(&mut app);
        drop_balls(&mut app);
        run_steps(&mut app, 30);

        app.world_mut()
            .resource_mut::<Messages<RestitutionLabAction>>()
            .write(RestitutionLabAction::Reset);
        app.update();

        for kind in RestitutionSurfaceKind::ALL {
            let entity = entities[&kind];
            assert!((position(&app, entity).y - BALL_START_Y).abs() < 0.001);
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<LinearVelocity>()
                    .expect("restitution ball velocity")
                    .0,
                Vec3::ZERO
            );
        }
    }

    #[test]
    fn bounce_order_remains_visible_with_avian_collider_debug_enabled() {
        let mut app = restitution_app_with_debug(true);
        app.update();
        enable_collider_debug(&mut app);
        assert!(app.world().resource::<ColliderDebugSettings>().enabled);

        drop_balls(&mut app);
        let peaks = first_bounce_peaks(&mut app);
        assert!(
            peaks[&RestitutionSurfaceKind::Low] < peaks[&RestitutionSurfaceKind::Medium]
                && peaks[&RestitutionSurfaceKind::Medium] < peaks[&RestitutionSurfaceKind::High]
        );
    }
}
