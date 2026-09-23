use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    collision_layers::{SandboxLayer, layers_for},
    cursor_hover::CursorRay,
    player::Player,
};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);

/// The six reusable objects available from the free-sandbox developer palette.
#[derive(Component, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpawnPreset {
    Cube,
    Ball,
    Plank,
    Barrel,
    HeavyBlock,
    BouncyBall,
}

impl SpawnPreset {
    pub const ALL: [Self; 6] = [
        Self::Cube,
        Self::Ball,
        Self::Plank,
        Self::Barrel,
        Self::HeavyBlock,
        Self::BouncyBall,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Cube => "Cube",
            Self::Ball => "Ball",
            Self::Plank => "Plank",
            Self::Barrel => "Barrel",
            Self::HeavyBlock => "Heavy Block",
            Self::BouncyBall => "Bouncy Ball",
        }
    }

    const fn dimensions(self) -> Vec3 {
        match self {
            Self::Cube | Self::HeavyBlock => Vec3::splat(0.8),
            Self::Ball | Self::BouncyBall => Vec3::splat(0.9),
            Self::Plank => Vec3::new(1.8, 0.35, 0.7),
            Self::Barrel => Vec3::new(0.9, 1.2, 0.9),
        }
    }

    const fn color(self) -> Color {
        match self {
            Self::Cube => Color::srgb(0.22, 0.66, 0.95),
            Self::Ball => Color::srgb(0.95, 0.34, 0.27),
            Self::Plank => Color::srgb(0.66, 0.4, 0.2),
            Self::Barrel => Color::srgb(0.28, 0.72, 0.38),
            Self::HeavyBlock => Color::srgb(0.48, 0.52, 0.6),
            Self::BouncyBall => Color::srgb(0.88, 0.24, 0.78),
        }
    }

    const fn density(self) -> f32 {
        match self {
            Self::HeavyBlock => 4.0,
            _ => 0.75,
        }
    }

    const fn restitution(self) -> f32 {
        match self {
            Self::BouncyBall => 0.9,
            _ => 0.0,
        }
    }

    fn collider(self) -> Collider {
        let dimensions = self.dimensions();
        match self {
            Self::Ball | Self::BouncyBall => Collider::sphere(dimensions.x * 0.5),
            Self::Barrel => Collider::cylinder(dimensions.x * 0.5, dimensions.y),
            Self::Cube | Self::Plank | Self::HeavyBlock => {
                Collider::cuboid(dimensions.x, dimensions.y, dimensions.z)
            }
        }
    }

    fn mesh(self) -> Mesh {
        let dimensions = self.dimensions();
        match self {
            Self::Ball | Self::BouncyBall => Sphere::new(dimensions.x * 0.5).into(),
            Self::Barrel => Cylinder::new(dimensions.x * 0.5, dimensions.y).into(),
            Self::Cube | Self::Plank | Self::HeavyBlock => Cuboid::from_size(dimensions).into(),
        }
    }

    const fn half_height(self) -> f32 {
        self.dimensions().y * 0.5
    }
}

/// Marks an object created by the free-sandbox spawn palette.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpawnedSandboxObject {
    pub preset: SpawnPreset,
}

/// Whether the Tab developer palette is currently open.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SpawnPaletteState {
    pub open: bool,
}

/// Owns the Tab palette and the six free-sandbox object presets.
pub struct ObjectSpawnPalettePlugin;

impl Plugin for ObjectSpawnPalettePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnPaletteState>()
            .add_systems(Startup, spawn_palette_ui)
            .add_systems(
                Update,
                (
                    toggle_palette,
                    spawn_from_controls,
                    update_palette_visibility,
                )
                    .chain(),
            );
    }
}

#[derive(Component, Clone, Copy)]
struct SpawnPresetButton(SpawnPreset);

#[derive(Component)]
struct SpawnPalettePanel;

fn spawn_palette_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(16.0),
                width: px(250.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            SpawnPalettePanel,
            Name::new("Object Spawn Palette"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("OBJECT SPAWNER — TAB"),
                TextFont::from_font_size(16.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Choose an object or press its number."),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));
            for (index, preset) in SpawnPreset::ALL.into_iter().enumerate() {
                parent
                    .spawn((
                        Button,
                        SpawnPresetButton(preset),
                        Node {
                            width: percent(100),
                            height: px(27.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(CONTROL_BACKGROUND),
                        Name::new(format!("Spawn {}", preset.label())),
                    ))
                    .with_child((
                        Text::new(format!("{} — {}", index + 1, preset.label())),
                        TextFont::from_font_size(12.0),
                        TextColor(PANEL_TEXT),
                    ));
            }
        });
}

fn toggle_palette(keyboard: Res<ButtonInput<KeyCode>>, mut state: ResMut<SpawnPaletteState>) {
    if keyboard.just_pressed(KeyCode::Tab) {
        state.open = !state.open;
    }
}

fn update_palette_visibility(
    state: Res<SpawnPaletteState>,
    mut panels: Query<&mut Node, With<SpawnPalettePanel>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut panel in &mut panels {
        panel.display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[derive(SystemParam)]
struct SpawnPaletteInput<'w, 's> {
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    state: Res<'w, SpawnPaletteState>,
    buttons:
        Query<'w, 's, (&'static Interaction, &'static SpawnPresetButton), Changed<Interaction>>,
    cursor_ray: Option<Res<'w, CursorRay>>,
    players: Query<'w, 's, &'static Transform, With<Player>>,
    cameras: Query<'w, 's, &'static Transform, With<crate::camera::FixedFollowCamera>>,
}

fn spawn_from_controls(
    mut commands: Commands,
    input: SpawnPaletteInput,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let button_preset = input.buttons.iter().find_map(|(interaction, button)| {
        (*interaction == Interaction::Pressed).then_some(button.0)
    });
    let keyboard_preset = if input.state.open {
        SpawnPreset::ALL
            .into_iter()
            .enumerate()
            .find_map(|(index, preset)| {
                let key = match index {
                    0 => KeyCode::Digit1,
                    1 => KeyCode::Digit2,
                    2 => KeyCode::Digit3,
                    3 => KeyCode::Digit4,
                    4 => KeyCode::Digit5,
                    _ => KeyCode::Digit6,
                };
                input.keyboard.just_pressed(key).then_some(preset)
            })
    } else {
        None
    };
    let Some(preset) = button_preset.or(keyboard_preset) else {
        return;
    };

    let position = spawn_position(
        preset,
        input.cursor_ray.as_deref(),
        &input.players,
        &input.cameras,
    );
    let mesh = meshes.as_mut().map(|meshes| meshes.add(preset.mesh()));
    let material = materials
        .as_mut()
        .map(|materials| materials.add(preset.color()));
    let mut entity = commands.spawn((
        SpawnedSandboxObject { preset },
        RigidBody::Dynamic,
        layers_for(SandboxLayer::Objects),
        preset.collider(),
        ColliderDensity(preset.density()),
        Restitution::new(preset.restitution()),
        Transform::from_translation(position),
        Name::new(format!("Spawned {}", preset.label())),
    ));
    if let (Some(mesh), Some(material)) = (mesh, material) {
        entity.insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}

fn spawn_position(
    preset: SpawnPreset,
    cursor_ray: Option<&CursorRay>,
    players: &Query<&Transform, With<Player>>,
    cameras: &Query<&Transform, With<crate::camera::FixedFollowCamera>>,
) -> Vec3 {
    let floor_position = cursor_ray
        .and_then(|cursor| cursor.ray)
        .and_then(|ray| {
            let direction_y = ray.direction.y;
            if direction_y.abs() < f32::EPSILON {
                return None;
            }
            let distance = -ray.origin.y / direction_y;
            (distance >= 0.0).then(|| ray.origin + ray.direction * distance)
        })
        .or_else(|| {
            let player = players.iter().next()?;
            let camera_forward = cameras
                .iter()
                .next()
                .map(|camera| camera.rotation * -Vec3::Z)
                .unwrap_or(Vec3::Z);
            let horizontal_forward =
                Vec3::new(camera_forward.x, 0.0, camera_forward.z).normalize_or_zero();
            let forward = if horizontal_forward == Vec3::ZERO {
                Vec3::Z
            } else {
                horizontal_forward
            };
            Some(player.translation + forward * 2.0)
        })
        .unwrap_or(Vec3::ZERO);

    floor_position + Vec3::Y * (preset.half_height() + 0.03)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::{ButtonState, InputPlugin, keyboard::KeyboardInput},
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::camera::FixedFollowCamera;

    fn palette_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            ObjectSpawnPalettePlugin,
        ))
        .init_resource::<CursorRay>()
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn release_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn spawned(app: &mut App) -> Vec<Entity> {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<SpawnedSandboxObject>>();
        query.iter(world).collect()
    }

    #[test]
    fn tab_opens_and_closes_palette() {
        let mut app = palette_app();
        app.update();
        let panel = {
            let world = app.world_mut();
            let mut panels = world.query_filtered::<Entity, With<SpawnPalettePanel>>();
            panels.single(world).unwrap()
        };
        assert_eq!(
            app.world().entity(panel).get::<Node>().unwrap().display,
            Display::None
        );

        press_key(&mut app, KeyCode::Tab);
        assert!(app.world().resource::<SpawnPaletteState>().open);
        assert_eq!(
            app.world().entity(panel).get::<Node>().unwrap().display,
            Display::Flex
        );

        release_key(&mut app, KeyCode::Tab);
        press_key(&mut app, KeyCode::Tab);
        assert!(!app.world().resource::<SpawnPaletteState>().open);
        assert_eq!(
            app.world().entity(panel).get::<Node>().unwrap().display,
            Display::None
        );
    }

    #[test]
    fn every_hotkey_spawns_a_visible_dynamic_body_with_the_matching_defaults() {
        let mut app = palette_app();
        app.world_mut()
            .spawn((Player, Transform::from_xyz(0.0, 1.0, 0.0)));
        app.world_mut()
            .spawn((FixedFollowCamera, Transform::from_rotation(Quat::IDENTITY)));
        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::new(2.0, 10.0, 3.0), Dir3::NEG_Y);
        app.update();
        press_key(&mut app, KeyCode::Tab);
        release_key(&mut app, KeyCode::Tab);

        for (index, (preset, key)) in SpawnPreset::ALL
            .into_iter()
            .zip([
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
                KeyCode::Digit5,
                KeyCode::Digit6,
            ])
            .enumerate()
        {
            let target_x = 2.0 + index as f32 * 3.0;
            app.world_mut()
                .resource_mut::<CursorRay>()
                .set(Vec3::new(target_x, 10.0, 3.0), Dir3::NEG_Y);
            press_key(&mut app, key);
            release_key(&mut app, key);
            let entity = spawned(&mut app)
                .into_iter()
                .find(|entity| {
                    app.world()
                        .entity(*entity)
                        .get::<SpawnedSandboxObject>()
                        .is_some_and(|spawned| spawned.preset == preset)
                })
                .expect("preset should spawn");
            let body = app.world().entity(entity);
            assert_eq!(body.get::<RigidBody>(), Some(&RigidBody::Dynamic));
            assert_eq!(
                body.get::<ColliderDensity>(),
                Some(&ColliderDensity(preset.density()))
            );
            assert_eq!(
                body.get::<Restitution>()
                    .expect("preset restitution")
                    .coefficient,
                preset.restitution()
            );
            assert!(body.contains::<Mesh3d>());
            assert!(body.contains::<MeshMaterial3d<StandardMaterial>>());
            let transform_position = body.get::<Transform>().unwrap().translation;
            assert!(
                transform_position
                    .abs_diff_eq(Vec3::new(target_x, preset.half_height() + 0.03, 3.0), 0.003,),
                "{preset:?} spawned at {transform_position:?}"
            );
            let collider = body.get::<Collider>().unwrap();
            match preset {
                SpawnPreset::Ball | SpawnPreset::BouncyBall => {
                    assert!(collider.shape().as_ball().is_some())
                }
                SpawnPreset::Barrel => assert!(collider.shape().as_cylinder().is_some()),
                SpawnPreset::Cube | SpawnPreset::Plank | SpawnPreset::HeavyBlock => {
                    assert!(collider.shape().as_cuboid().is_some())
                }
            }
            let material_handle = body.get::<MeshMaterial3d<StandardMaterial>>().unwrap();
            assert_eq!(
                app.world()
                    .resource::<Assets<StandardMaterial>>()
                    .get(&material_handle.0)
                    .unwrap()
                    .base_color,
                preset.color()
            );
        }
        assert_eq!(spawned(&mut app).len(), SpawnPreset::ALL.len());
    }

    #[test]
    fn unavailable_cursor_uses_a_position_near_the_player() {
        let mut app = palette_app();
        app.world_mut()
            .spawn((Player, Transform::from_xyz(1.0, 1.0, 2.0)));
        app.update();
        press_key(&mut app, KeyCode::Tab);
        release_key(&mut app, KeyCode::Tab);
        press_key(&mut app, KeyCode::Digit1);

        let entity = spawned(&mut app)[0];
        let position = app
            .world()
            .entity(entity)
            .get::<Position>()
            .expect("spawned position")
            .0;
        assert!(position.abs_diff_eq(Vec3::new(1.0, 1.43, 4.0), 1e-5));
    }
}
