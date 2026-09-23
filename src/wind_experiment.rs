use avian3d::prelude::*;
use bevy::prelude::*;

use crate::gameplay_materials::{GameplayMaterial, WindResponse};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const INITIAL_WIND_STRENGTH: f32 = 8.0;
const STRENGTH_STEP: f32 = 2.0;
const TUNNEL_CENTER: Vec3 = Vec3::new(0.0, 0.0, 20.0);
const OBJECT_SPACING: f32 = 0.9;

/// Marks a dynamic body as eligible to receive the wind experiment's force.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WindAffected;

/// Runtime wind state shared by the experiment controls and force system.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct WindSettings {
    pub enabled: bool,
    pub direction: Vec3,
    pub strength: f32,
}

impl Default for WindSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            direction: Vec3::X,
            strength: INITIAL_WIND_STRENGTH,
        }
    }
}

/// A request from the wind panel to change one part of the wind state.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum WindAction {
    Toggle,
    AdjustStrength(f32),
    SetDirection(Vec3),
}

/// Builds the wind tunnel, test bodies, runtime controls, and Avian force system.
pub struct WindExperimentPlugin;

impl Plugin for WindExperimentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindSettings>()
            .add_message::<WindAction>()
            .add_systems(Startup, (spawn_wind_tunnel, spawn_wind_controls))
            .add_systems(
                Update,
                (queue_wind_actions, apply_wind_actions, update_wind_status).chain(),
            )
            .add_systems(FixedUpdate, apply_wind_force);
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
enum WindControl {
    Toggle,
    Strength(f32),
    Direction(Vec3),
}

fn spawn_wind_tunnel(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let sections = [
        (
            "Wind Tunnel Floor",
            Vec3::new(0.0, -0.15, 20.0),
            Vec3::new(12.0, 0.3, 4.0),
        ),
        (
            "Wind Tunnel Left Wall",
            Vec3::new(0.0, 1.5, 18.0),
            Vec3::new(12.0, 3.0, 0.15),
        ),
        (
            "Wind Tunnel Right Wall",
            Vec3::new(0.0, 1.5, 22.0),
            Vec3::new(12.0, 3.0, 0.15),
        ),
        (
            "Wind Tunnel Back",
            Vec3::new(-6.0, 1.5, 20.0),
            Vec3::new(0.15, 3.0, 4.0),
        ),
    ];
    let tunnel_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.22, 0.3, 0.38)));
    for (name, position, size) in sections {
        let transform = Transform::from_translation(position);
        let mut entity = commands.spawn((
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            transform,
            Name::new(name),
        ));
        if let (Some(meshes), Some(material)) = (meshes.as_deref_mut(), tunnel_material.as_ref()) {
            entity.insert((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(material.clone()),
            ));
        }
    }

    let presets = [
        ("Paper", None, 2.0, Color::srgb(0.92, 0.88, 0.68)),
        (
            "Wood",
            Some(GameplayMaterial::Wood),
            0.0,
            Color::srgb(0.58, 0.32, 0.16),
        ),
        (
            "Ceramic",
            Some(GameplayMaterial::Ceramic),
            0.0,
            Color::srgb(0.3, 0.72, 0.84),
        ),
        (
            "Stone",
            Some(GameplayMaterial::Stone),
            0.0,
            Color::srgb(0.48, 0.5, 0.54),
        ),
    ];
    let meshes = meshes
        .as_mut()
        .map(|meshes| meshes.add(Cuboid::from_size(Vec3::splat(0.65))));
    for (index, (label, material, paper_factor, color)) in presets.into_iter().enumerate() {
        let z = index as f32 * OBJECT_SPACING - 1.5 * OBJECT_SPACING;
        let position = TUNNEL_CENTER + Vec3::new(-4.5, 0.55, z);
        // Spawn each preset separately so the material-defined wind response is applied normally.
        let mut entity = commands.spawn((
            WindAffected,
            RigidBody::Dynamic,
            Collider::cuboid(0.65, 0.65, 0.65),
            Transform::from_translation(position),
            Name::new(format!("Wind Test - {label}")),
        ));
        if let Some(material) = material {
            entity.insert(material);
        } else {
            entity.insert(WindResponse(paper_factor));
        }
        if let (Some(mesh), Some(materials)) = (meshes.as_ref(), materials.as_mut()) {
            entity.insert((Mesh3d(mesh.clone()), MeshMaterial3d(materials.add(color))));
        }
    }
}

fn spawn_wind_controls(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                bottom: px(16.0),
                width: px(300.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            Name::new("Wind Experiment Controls"),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("WIND TUNNEL"),
                TextFont::from_font_size(16.0),
                TextColor(PANEL_TEXT),
            ));
            panel.spawn((
                Text::new("Compare paper, wood, ceramic, and stone response"),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));
            spawn_control_row(
                panel,
                [
                    ("wind on/off", WindControl::Toggle),
                    ("− strength", WindControl::Strength(-STRENGTH_STEP)),
                    ("+ strength", WindControl::Strength(STRENGTH_STEP)),
                ],
            );
            spawn_control_row(
                panel,
                [
                    ("+X", WindControl::Direction(Vec3::X)),
                    ("−X", WindControl::Direction(Vec3::NEG_X)),
                    ("+Z", WindControl::Direction(Vec3::Z)),
                ],
            );
            spawn_control_row(
                panel,
                [
                    ("−Z", WindControl::Direction(Vec3::NEG_Z)),
                    ("+Y", WindControl::Direction(Vec3::Y)),
                    ("−Y", WindControl::Direction(Vec3::NEG_Y)),
                ],
            );
            panel.spawn((
                Text::new("WIND: ON · 8.0 N · +X"),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
                WindStatusText,
            ));
        });
}

#[derive(Component)]
struct WindStatusText;

fn spawn_control_row(panel: &mut ChildSpawnerCommands, controls: [(&'static str, WindControl); 3]) {
    panel
        .spawn(Node {
            width: percent(100.0),
            height: px(26.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            for (label, control) in controls {
                row.spawn((
                    Button,
                    control,
                    Node {
                        width: px(88.0),
                        height: px(22.0),
                        margin: UiRect::horizontal(px(2.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new(format!("Wind control {label}")),
                ))
                .with_child((
                    Text::new(label),
                    TextFont::from_font_size(10.0),
                    TextColor(PANEL_TEXT),
                ));
            }
        });
}

fn queue_wind_actions(
    controls: Query<(&Interaction, &WindControl), Changed<Interaction>>,
    mut actions: MessageWriter<WindAction>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<WindControl>>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            actions.write(match *control {
                WindControl::Toggle => WindAction::Toggle,
                WindControl::Strength(amount) => WindAction::AdjustStrength(amount),
                WindControl::Direction(direction) => WindAction::SetDirection(direction),
            });
        }
    }
    for (interaction, mut color) in &mut buttons {
        *color = if *interaction == Interaction::Hovered || *interaction == Interaction::Pressed {
            CONTROL_PRESSED.into()
        } else {
            CONTROL_BACKGROUND.into()
        };
    }
}

fn apply_wind_actions(mut actions: MessageReader<WindAction>, mut settings: ResMut<WindSettings>) {
    for action in actions.read().copied() {
        match action {
            WindAction::Toggle => settings.enabled = !settings.enabled,
            WindAction::AdjustStrength(delta) => {
                settings.strength = (settings.strength + delta).max(0.0);
            }
            WindAction::SetDirection(direction) if direction.length_squared() > 0.0 => {
                settings.direction = direction.normalize();
            }
            WindAction::SetDirection(_) => {}
        }
    }
}

fn apply_wind_force(
    settings: Res<WindSettings>,
    mut affected: Query<(&RigidBody, &WindResponse, Forces), With<WindAffected>>,
) {
    if !settings.enabled || settings.strength <= 0.0 {
        return;
    }
    let wind_force = settings.direction * settings.strength;
    for (body, response, mut forces) in &mut affected {
        if *body == RigidBody::Dynamic {
            forces.apply_force(wind_force * response.0.max(0.0));
        }
    }
}

fn update_wind_status(
    settings: Res<WindSettings>,
    mut status: Query<&mut Text, With<WindStatusText>>,
) {
    if !settings.is_changed() {
        return;
    }
    let direction = settings.direction;
    let axis = if direction.x.abs() > 0.5 {
        if direction.x > 0.0 { "+X" } else { "−X" }
    } else if direction.y.abs() > 0.5 {
        if direction.y > 0.0 { "+Y" } else { "−Y" }
    } else if direction.z > 0.0 {
        "+Z"
    } else {
        "−Z"
    };
    for mut text in &mut status {
        **text = format!(
            "WIND: {} · {:.1} N · {axis}",
            if settings.enabled { "ON" } else { "OFF" },
            settings.strength
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use crate::gameplay_materials::GameplayMaterialsPlugin;

    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    fn wind_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            GameplayMaterialsPlugin,
            WindExperimentPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            STEP,
        )));
        app.finish();
        app
    }

    fn run_steps(app: &mut App, count: usize) {
        for _ in 0..count {
            app.update();
        }
    }

    fn velocities(app: &mut App) -> Vec<(String, Vec3)> {
        let world = app.world_mut();
        let mut bodies = world.query::<(&Name, &LinearVelocity)>();
        bodies
            .iter(world)
            .filter(|(name, _)| name.as_str().starts_with("Wind Test - "))
            .map(|(name, velocity)| (name.as_str().to_owned(), velocity.0))
            .collect()
    }

    fn set_wind(app: &mut App, action: WindAction) {
        app.world_mut()
            .resource_mut::<Messages<WindAction>>()
            .write(action);
        app.update();
    }

    #[test]
    fn tunnel_contains_paper_wood_ceramic_and_stone_comparison_bodies() {
        let mut app = wind_app();
        app.update();

        let world = app.world_mut();
        let mut bodies = world.query::<(&Name, &RigidBody, &WindAffected, &WindResponse)>();
        let objects: Vec<_> = bodies
            .iter(world)
            .filter(|(name, ..)| name.as_str().starts_with("Wind Test - "))
            .collect();
        assert_eq!(objects.len(), 4);
        assert!(
            objects
                .iter()
                .all(|(_, body, _, _)| **body == RigidBody::Dynamic)
        );
        assert!(
            objects
                .iter()
                .any(|(name, _, _, _)| name.as_str() == "Wind Test - Paper")
        );
        assert!(
            objects
                .iter()
                .any(|(name, _, _, _)| name.as_str() == "Wind Test - Wood")
        );
        assert!(
            objects
                .iter()
                .any(|(name, _, _, _)| name.as_str() == "Wind Test - Ceramic")
        );
        assert!(
            objects
                .iter()
                .any(|(name, _, _, _)| name.as_str() == "Wind Test - Stone")
        );

        let mut controls = world.query::<&WindControl>();
        assert!(
            controls
                .iter(world)
                .any(|control| *control == WindControl::Toggle)
        );
        assert!(
            controls
                .iter(world)
                .any(|control| matches!(control, WindControl::Strength(_)))
        );
        assert!(
            controls
                .iter(world)
                .any(|control| matches!(control, WindControl::Direction(_)))
        );
    }

    #[test]
    fn material_wind_responses_order_force_effect_and_controls_change_direction_and_strength() {
        let mut app = wind_app();
        app.update();
        run_steps(&mut app, 30);
        let initial = velocities(&mut app);
        let speed = |name: &str| {
            initial
                .iter()
                .find_map(|(object, velocity)| (object == name).then_some(velocity.length()))
                .unwrap()
        };
        let paper_speed = speed("Wind Test - Paper");
        let wood_speed = speed("Wind Test - Wood");
        let ceramic_speed = speed("Wind Test - Ceramic");
        let stone_speed = speed("Wind Test - Stone");
        assert!(paper_speed > wood_speed, "{initial:?}");
        assert!(wood_speed > ceramic_speed, "{initial:?}");
        assert!(ceramic_speed > stone_speed, "{initial:?}");

        set_wind(&mut app, WindAction::SetDirection(Vec3::Y));
        set_wind(&mut app, WindAction::AdjustStrength(6.0));
        run_steps(&mut app, 10);
        let redirected = velocities(&mut app);
        let paper = redirected
            .iter()
            .find(|(name, _)| name == "Wind Test - Paper")
            .unwrap()
            .1;
        assert!(
            paper.y > 1.0,
            "wind did not follow the updated direction/strength: {paper:?}"
        );
        assert_eq!(
            app.world().resource::<WindSettings>().strength,
            INITIAL_WIND_STRENGTH + 6.0
        );
        assert_eq!(app.world().resource::<WindSettings>().direction, Vec3::Y);
    }

    #[test]
    fn disabling_wind_stops_force_and_unmarked_bodies_are_not_affected() {
        let mut app = wind_app();
        app.update();
        set_wind(&mut app, WindAction::Toggle);
        let baseline = velocities(&mut app);
        let unmarked = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                WindResponse(10.0),
                LinearVelocity::ZERO,
                Position(Vec3::new(30.0, 0.0, 0.0)),
            ))
            .id();
        run_steps(&mut app, 30);
        let after_disabled_steps = velocities(&mut app);
        for (name, velocity) in &baseline {
            let later = after_disabled_steps
                .iter()
                .find(|(later_name, _)| later_name == name)
                .unwrap()
                .1;
            assert_eq!(
                *velocity, later,
                "wind continued after disabling for {name}"
            );
        }
        assert_eq!(
            app.world()
                .entity(unmarked)
                .get::<LinearVelocity>()
                .unwrap()
                .0,
            Vec3::ZERO
        );
    }
}
