use avian3d::prelude::*;
use bevy::prelude::*;

use crate::stations::StationObject;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

const RAMP_ANGLE: f32 = 17.0_f32.to_radians();
const RAMP_LENGTH: f32 = 1.8;
const RAMP_WIDTH: f32 = 1.05;
const RAMP_THICKNESS: f32 = 0.12;
const RUNOUT_LENGTH: f32 = 2.2;
const RUNOUT_WIDTH: f32 = RAMP_WIDTH;
const RUNOUT_THICKNESS: f32 = 0.12;
const RUNOUT_TOP: f32 = 0.12;
const RAMP_CENTER_Z: f32 = -6.0;
const OBJECT_SIZE: f32 = 0.36;
const OBJECT_HALF_SIZE: f32 = OBJECT_SIZE * 0.5;
const RELEASE_SPEED: f32 = 1.5;
const OBJECT_START_ALONG_RAMP: f32 = 0.66;

/// The three materials compared by station B.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrictionLaneKind {
    Ice,
    Wood,
    Rubber,
}

impl FrictionLaneKind {
    pub const ALL: [Self; 3] = [Self::Ice, Self::Wood, Self::Rubber];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ice => "ICE",
            Self::Wood => "WOOD",
            Self::Rubber => "RUBBER",
        }
    }

    pub const fn coefficient(self) -> f32 {
        match self {
            Self::Ice => 0.05,
            Self::Wood => 0.35,
            Self::Rubber => 0.80,
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Ice => Color::srgb(0.35, 0.82, 1.0),
            Self::Wood => Color::srgb(0.76, 0.43, 0.18),
            Self::Rubber => Color::srgb(0.88, 0.2, 0.22),
        }
    }

    const fn lane_x(self) -> f32 {
        match self {
            Self::Ice => -1.35,
            Self::Wood => 0.0,
            Self::Rubber => 1.35,
        }
    }
}

/// Marks one static surface section in a friction lane.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrictionLaneSurface {
    pub kind: FrictionLaneKind,
    pub ramp: bool,
}

/// Marks the identical moving body released onto a friction lane.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrictionLabObject {
    pub lane: FrictionLaneKind,
}

/// Requests a station-wide friction demonstration action.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrictionLabAction {
    Release,
    Reset,
}

/// Owns station B's ice, wood, and rubber ramps and test objects.
pub struct FrictionStationPlugin;

impl Plugin for FrictionStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FrictionLabAction>()
            .add_systems(Startup, (spawn_friction_lab, spawn_friction_station_ui))
            .add_systems(
                Update,
                (
                    queue_friction_station_actions,
                    update_friction_button_colors,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                draw_friction_labels.run_if(resource_exists::<GizmoConfigStore>),
            )
            .add_systems(PostUpdate, reset_friction_lab)
            .add_systems(FixedUpdate, release_friction_objects);
    }
}

fn ramp_center_y() -> f32 {
    RUNOUT_TOP + RAMP_ANGLE.sin() * RAMP_LENGTH * 0.5 - RAMP_ANGLE.cos() * RAMP_THICKNESS * 0.5
}

fn ramp_rotation() -> Quat {
    // Positive local Z is the high end. The negative X rotation makes the
    // ramp descend toward negative Z, away from station B's pad.
    Quat::from_rotation_x(-RAMP_ANGLE)
}

fn ramp_surface_position(x: f32, along_ramp: f32) -> Vec3 {
    let rotation = ramp_rotation();
    Vec3::new(x, ramp_center_y(), RAMP_CENTER_Z)
        + rotation * Vec3::new(0.0, RAMP_THICKNESS * 0.5, along_ramp)
        + rotation * Vec3::Y * OBJECT_HALF_SIZE
}

fn spawn_friction_lab(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_length(1.0)));

    for kind in FrictionLaneKind::ALL {
        let lane_x = kind.lane_x();
        let friction = Friction::new(kind.coefficient()).with_combine_rule(CoefficientCombine::Max);
        let color = materials
            .as_mut()
            .map(|materials| materials.add(kind.color()));

        let ramp_transform =
            Transform::from_translation(Vec3::new(lane_x, ramp_center_y(), RAMP_CENTER_Z))
                .with_rotation(ramp_rotation());
        let mut ramp = commands.spawn((
            FrictionLaneSurface { kind, ramp: true },
            RigidBody::Static,
            Collider::cuboid(RAMP_WIDTH, RAMP_THICKNESS, RAMP_LENGTH),
            friction,
            ramp_transform,
            Name::new(format!("Friction Lane - {} Ramp", kind.label())),
        ));
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), color.as_ref()) {
            ramp.insert((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                ramp_transform.with_scale(Vec3::new(RAMP_WIDTH, RAMP_THICKNESS, RAMP_LENGTH)),
            ));
        }

        let runout_center_z = RAMP_CENTER_Z - RAMP_LENGTH * 0.5 - RUNOUT_LENGTH * 0.5;
        let runout_transform =
            Transform::from_xyz(lane_x, RUNOUT_TOP - RUNOUT_THICKNESS * 0.5, runout_center_z);
        let mut runout = commands.spawn((
            FrictionLaneSurface { kind, ramp: false },
            RigidBody::Static,
            Collider::cuboid(RUNOUT_WIDTH, RUNOUT_THICKNESS, RUNOUT_LENGTH),
            friction,
            runout_transform,
            Name::new(format!("Friction Lane - {} Runout", kind.label())),
        ));
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), color.as_ref()) {
            runout.insert((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                runout_transform.with_scale(Vec3::new(
                    RUNOUT_WIDTH,
                    RUNOUT_THICKNESS,
                    RUNOUT_LENGTH,
                )),
            ));
        }

        let initial_transform =
            Transform::from_translation(ramp_surface_position(lane_x, OBJECT_START_ALONG_RAMP));
        let object_color = materials
            .as_mut()
            .map(|materials| materials.add(kind.color().with_luminance(0.8)));
        let mut object = commands.spawn((
            FrictionLabObject { lane: kind },
            StationObject::new('B', initial_transform),
            RigidBody::Dynamic,
            Collider::cuboid(OBJECT_SIZE, OBJECT_SIZE, OBJECT_SIZE),
            // Max combines this zero-friction object with the lane material,
            // making the lane's coefficient the effective contact value.
            Friction::ZERO.with_combine_rule(CoefficientCombine::Max),
            Restitution::new(0.0),
            LinearDamping(0.0),
            AngularDamping(0.0),
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            SleepingDisabled,
            initial_transform,
            Name::new(format!("Friction Test Object - {}", kind.label())),
        ));
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), object_color.as_ref()) {
            object.insert((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(initial_transform.translation)
                    .with_scale(Vec3::splat(OBJECT_SIZE)),
            ));
        }
    }
}

#[derive(Component)]
struct FrictionStationPanel;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum FrictionStationControl {
    Release,
    Reset,
}

fn spawn_friction_station_ui(mut commands: Commands) {
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
            FrictionStationPanel,
            Name::new("Friction Station Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("B — FRICTION LABORATORY"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Release identical sliders together on parallel ramps. Lower friction should carry an object farther down the runout."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for kind in FrictionLaneKind::ALL {
                parent.spawn((
                    Text::new(format!("{:<6} friction coefficient  {:.2}", kind.label(), kind.coefficient())),
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
                    FrictionStationControl::Release,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Friction station release"),
                ))
                .with_child((
                    Text::new("release objects"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
            parent
                .spawn((
                    Button,
                    FrictionStationControl::Reset,
                    Node {
                        width: px(120.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Friction station reset"),
                ))
                .with_child((
                    Text::new("reset station B"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_friction_station_actions(
    controls: Query<(&Interaction, &FrictionStationControl), Changed<Interaction>>,
    mut actions: MessageWriter<FrictionLabAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match control {
            FrictionStationControl::Release => FrictionLabAction::Release,
            FrictionStationControl::Reset => FrictionLabAction::Reset,
        });
    }
}

fn update_friction_button_colors(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<FrictionStationControl>>,
) {
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn reset_friction_lab(
    mut actions: MessageReader<FrictionLabAction>,
    mut objects: Query<
        (
            &StationObject,
            &mut Transform,
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut AngularVelocity,
        ),
        With<FrictionLabObject>,
    >,
) {
    if !actions
        .read()
        .any(|action| *action == FrictionLabAction::Reset)
    {
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

fn release_friction_objects(
    mut actions: MessageReader<FrictionLabAction>,
    mut objects: Query<&mut LinearVelocity, With<FrictionLabObject>>,
) {
    if !actions
        .read()
        .any(|action| *action == FrictionLabAction::Release)
    {
        return;
    }

    for mut velocity in &mut objects {
        velocity.0 = Vec3::NEG_Z * RELEASE_SPEED;
    }
}

fn draw_friction_labels(
    mut gizmos: Gizmos,
    surfaces: Query<(&FrictionLaneSurface, &Transform)>,
    objects: Query<(&FrictionLabObject, &Transform)>,
) {
    for (surface, transform) in &surfaces {
        if !surface.ramp {
            continue;
        }
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 0.36, Quat::IDENTITY),
            &format!(
                "{}\nμ {:.2}",
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
            Isometry3d::new(transform.translation + Vec3::Y * 0.28, Quat::IDENTITY),
            object.lane.label(),
            0.28,
            Vec2::ZERO,
            object.lane.color(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::stations::StationLayoutPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn friction_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            FrictionStationPlugin,
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

    fn release(app: &mut App) {
        app.world_mut()
            .resource_mut::<Messages<FrictionLabAction>>()
            .write(FrictionLabAction::Release);
        app.update();
    }

    fn positions(app: &mut App) -> HashMap<FrictionLaneKind, (Vec3, Vec3)> {
        let world = app.world_mut();
        let mut objects = world.query::<(&FrictionLabObject, &Position, &Transform)>();
        objects
            .iter(world)
            .map(|(object, position, transform)| (object.lane, (position.0, transform.translation)))
            .collect()
    }

    #[test]
    fn station_spawns_labelled_ice_wood_and_rubber_lanes_with_distinct_values() {
        let mut app = friction_app();
        app.update();

        let world = app.world_mut();
        let mut surfaces = world.query::<(&FrictionLaneSurface, &Friction, &Name)>();
        let surfaces: Vec<_> = surfaces.iter(world).collect();
        assert_eq!(surfaces.len(), FrictionLaneKind::ALL.len() * 2);

        for kind in FrictionLaneKind::ALL {
            let matching: Vec<_> = surfaces
                .iter()
                .filter(|(surface, _, _)| surface.kind == kind)
                .collect();
            assert_eq!(
                matching.len(),
                2,
                "missing ramp/runout for {}",
                kind.label()
            );
            for (_, friction, name) in matching {
                assert_eq!(friction.dynamic_coefficient, kind.coefficient());
                assert_eq!(friction.static_coefficient, kind.coefficient());
                assert!(name.as_str().contains(kind.label()));
            }
        }

        let mut objects = world.query::<(&FrictionLabObject, &Name)>();
        let objects: Vec<_> = objects.iter(world).collect();
        assert_eq!(objects.len(), FrictionLaneKind::ALL.len());
        for (object, name) in objects {
            assert!(name.as_str().contains(object.lane.label()));
        }
    }

    #[test]
    fn each_lane_has_an_inclined_ramp_and_a_flat_runout() {
        let mut app = friction_app();
        app.update();

        let world = app.world_mut();
        let mut surfaces = world.query::<(&FrictionLaneSurface, &Transform)>();
        let surfaces: Vec<_> = surfaces.iter(world).collect();
        for kind in FrictionLaneKind::ALL {
            let ramp = surfaces
                .iter()
                .find(|(surface, _)| surface.kind == kind && surface.ramp)
                .map(|(_, transform)| transform)
                .expect("ramp");
            let runout = surfaces
                .iter()
                .find(|(surface, _)| surface.kind == kind && !surface.ramp)
                .map(|(_, transform)| transform)
                .expect("runout");
            assert!(ramp.rotation != Quat::IDENTITY);
            assert_eq!(runout.rotation, Quat::IDENTITY);
        }
    }

    #[test]
    fn simultaneously_released_matching_objects_travel_in_friction_order() {
        let mut app = friction_app();
        app.update();
        let initial = positions(&mut app);

        release(&mut app);
        run_steps(&mut app, 70);

        let final_positions = positions(&mut app);
        let distance = |kind: FrictionLaneKind| initial[&kind].0.z - final_positions[&kind].0.z;
        let ice = distance(FrictionLaneKind::Ice);
        let wood = distance(FrictionLaneKind::Wood);
        let rubber = distance(FrictionLaneKind::Rubber);
        assert!(
            ice > wood + 0.1,
            "ice {ice} was not farther than wood {wood}"
        );
        assert!(
            wood > rubber + 0.1,
            "wood {wood} was not farther than rubber {rubber}"
        );
    }

    #[test]
    fn repeated_release_runs_have_the_same_distance_ordering() {
        let mut orderings = Vec::new();
        for _ in 0..3 {
            let mut app = friction_app();
            app.update();
            let initial = positions(&mut app);
            release(&mut app);
            run_steps(&mut app, 70);
            let final_positions = positions(&mut app);
            let mut distances: Vec<_> = FrictionLaneKind::ALL
                .into_iter()
                .map(|kind| (kind, initial[&kind].0.z - final_positions[&kind].0.z))
                .collect();
            distances.sort_by(|(_, a), (_, b)| b.total_cmp(a));
            orderings.push(
                distances
                    .into_iter()
                    .map(|(kind, _)| kind)
                    .collect::<Vec<_>>(),
            );
        }

        assert!(orderings.iter().all(|order| order
            == &vec![
                FrictionLaneKind::Ice,
                FrictionLaneKind::Wood,
                FrictionLaneKind::Rubber,
            ]));
        assert!(orderings.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn reset_returns_objects_to_the_same_release_positions() {
        let mut app = friction_app();
        app.update();
        let initial = positions(&mut app);
        release(&mut app);
        run_steps(&mut app, 30);

        app.world_mut()
            .resource_mut::<Messages<FrictionLabAction>>()
            .write(FrictionLabAction::Reset);
        app.update();

        let reset = positions(&mut app);
        for kind in FrictionLaneKind::ALL {
            assert!(reset[&kind].0.distance(initial[&kind].0) < 0.001);
            let entity = {
                let world = app.world_mut();
                let mut objects = world.query::<(Entity, &FrictionLabObject)>();
                objects
                    .iter(world)
                    .find_map(|(entity, object)| (object.lane == kind).then_some(entity))
                    .expect("friction object")
            };
            assert_eq!(
                app.world()
                    .entity(entity)
                    .get::<LinearVelocity>()
                    .unwrap()
                    .0,
                Vec3::ZERO
            );
        }
    }
}
