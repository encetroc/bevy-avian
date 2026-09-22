use avian3d::prelude::*;
use bevy::prelude::*;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// Standard gravity vectors exposed by the global gravity controls.
pub const EARTH_GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
pub const MOON_GRAVITY: Vec3 = Vec3::new(0.0, -1.62, 0.0);
pub const ZERO_GRAVITY: Vec3 = Vec3::ZERO;
pub const REVERSE_GRAVITY: Vec3 = Vec3::new(0.0, 9.81, 0.0);

/// The named global gravity configurations available from the control panel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GravityPreset {
    Earth,
    Moon,
    ZeroG,
    Reverse,
}

impl GravityPreset {
    const ALL: [Self; 4] = [Self::Earth, Self::Moon, Self::ZeroG, Self::Reverse];

    pub const fn gravity(self) -> Vec3 {
        match self {
            Self::Earth => EARTH_GRAVITY,
            Self::Moon => MOON_GRAVITY,
            Self::ZeroG => ZERO_GRAVITY,
            Self::Reverse => REVERSE_GRAVITY,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Earth => "Earth",
            Self::Moon => "Moon",
            Self::ZeroG => "Zero G",
            Self::Reverse => "Reverse",
        }
    }
}

/// Which component of the global gravity vector is being edited.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GravityAxis {
    X,
    Y,
    Z,
}

impl GravityAxis {
    const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    const fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    const fn value(self, gravity: Vec3) -> f32 {
        match self {
            Self::X => gravity.x,
            Self::Y => gravity.y,
            Self::Z => gravity.z,
        }
    }
}

/// Direction for one editable gravity-axis step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GravityAdjustment {
    Decrease,
    Increase,
}

/// A runtime request emitted by the global gravity controls.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum GravityAction {
    SetPreset(GravityPreset),
    AdjustAxis { axis: GravityAxis, delta: f32 },
}

/// Owns the global gravity resource and its runtime controls.
pub struct GravityControlsPlugin;

impl Plugin for GravityControlsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(EARTH_GRAVITY))
            .add_message::<GravityAction>()
            .add_systems(Startup, spawn_gravity_controls_ui)
            .add_systems(
                Update,
                (
                    queue_gravity_actions,
                    apply_gravity_actions,
                    update_gravity_controls_ui,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
struct GravityPanel;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
enum GravityControl {
    Preset(GravityPreset),
    Axis {
        axis: GravityAxis,
        adjustment: GravityAdjustment,
    },
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct GravityValueText(GravityAxis);

fn spawn_gravity_controls_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(16.0),
                width: px(400.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            GravityPanel,
            Name::new("Global Gravity Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GLOBAL GRAVITY"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Acceleration applied to dynamic bodies"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::bottom(px(6.0)),
                    ..default()
                },
            ));

            for axis in GravityAxis::ALL {
                spawn_axis_row(parent, axis);
            }

            parent.spawn((
                Text::new("PRESETS"),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                Node {
                    margin: UiRect::top(px(6.0)),
                    ..default()
                },
            ));
            parent
                .spawn(Node {
                    width: percent(100.0),
                    height: px(26.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for preset in GravityPreset::ALL {
                        spawn_gravity_button(row, preset.label(), GravityControl::Preset(preset));
                    }
                });
        });
}

fn spawn_axis_row(parent: &mut ChildSpawnerCommands, axis: GravityAxis) {
    parent
        .spawn((
            Node {
                width: percent(100.0),
                height: px(26.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new(format!("Gravity {} axis controls", axis.label())),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{} axis", axis.label())),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                Node {
                    width: px(90.0),
                    ..default()
                },
            ));
            spawn_gravity_button(
                row,
                "−",
                GravityControl::Axis {
                    axis,
                    adjustment: GravityAdjustment::Decrease,
                },
            );
            row.spawn((
                Text::new("0.00"),
                TextFont::from_font_size(13.0),
                TextColor(PANEL_TEXT),
                GravityValueText(axis),
                Node {
                    width: px(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ));
            spawn_gravity_button(
                row,
                "+",
                GravityControl::Axis {
                    axis,
                    adjustment: GravityAdjustment::Increase,
                },
            );
        });
}

fn spawn_gravity_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: GravityControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(68.0),
                height: px(22.0),
                margin: UiRect::horizontal(px(2.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Gravity control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(11.0),
            TextColor(PANEL_TEXT),
        ));
}

fn queue_gravity_actions(
    controls: Query<(&Interaction, &GravityControl), Changed<Interaction>>,
    mut actions: MessageWriter<GravityAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *control {
            GravityControl::Preset(preset) => actions.write(GravityAction::SetPreset(preset)),
            GravityControl::Axis { axis, adjustment } => actions.write(GravityAction::AdjustAxis {
                axis,
                delta: match adjustment {
                    GravityAdjustment::Decrease => -0.5,
                    GravityAdjustment::Increase => 0.5,
                },
            }),
        };
    }
}

fn apply_gravity_actions(mut actions: MessageReader<GravityAction>, mut gravity: ResMut<Gravity>) {
    for action in actions.read().copied() {
        match action {
            GravityAction::SetPreset(preset) => gravity.0 = preset.gravity(),
            GravityAction::AdjustAxis { axis, delta } => match axis {
                GravityAxis::X => gravity.0.x += delta,
                GravityAxis::Y => gravity.0.y += delta,
                GravityAxis::Z => gravity.0.z += delta,
            },
        }
    }
}

fn update_gravity_controls_ui(
    gravity: Res<Gravity>,
    mut values: Query<(&GravityValueText, &mut Text)>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<GravityControl>>,
) {
    for (axis, mut text) in &mut values {
        text.0 = format!("{:.2}", axis.0.value(gravity.0));
    }
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn gravity_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            GravityControlsPlugin,
        ))
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

    fn request(app: &mut App, action: GravityAction) {
        app.world_mut()
            .resource_mut::<Messages<GravityAction>>()
            .write(action);
        app.update();
    }

    fn spawn_test_body(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                Position(Vec3::new(0.0, 10.0, 0.0)),
                Transform::from_xyz(0.0, 10.0, 0.0),
            ))
            .id()
    }

    fn velocity(app: &App, body: Entity) -> Vec3 {
        app.world()
            .entity(body)
            .get::<LinearVelocity>()
            .expect("test body velocity")
            .0
    }

    #[test]
    fn plugin_starts_with_earth_gravity_and_all_runtime_controls() {
        let mut app = gravity_app();
        app.update();

        assert_eq!(app.world().resource::<Gravity>().0, EARTH_GRAVITY);
        let world = app.world_mut();
        let mut controls = world.query::<&GravityControl>();
        assert_eq!(
            controls.iter(world).count(),
            3 * 2 + GravityPreset::ALL.len()
        );
        let mut values = world.query::<&GravityValueText>();
        assert_eq!(values.iter(world).count(), GravityAxis::ALL.len());
    }

    #[test]
    fn every_preset_changes_gravity_and_takes_effect_on_the_next_physics_steps() {
        for (preset, direction) in [
            (GravityPreset::Earth, -1.0),
            (GravityPreset::Moon, -1.0),
            (GravityPreset::ZeroG, 0.0),
            (GravityPreset::Reverse, 1.0),
        ] {
            let mut app = gravity_app();
            let body = spawn_test_body(&mut app);
            request(&mut app, GravityAction::SetPreset(preset));
            run_steps(&mut app, 30);

            assert_eq!(app.world().resource::<Gravity>().0, preset.gravity());
            let y_velocity = velocity(&app, body).y;
            if direction == 0.0 {
                assert!(y_velocity.abs() < 0.01, "zero-G body moved at {y_velocity}");
            } else {
                assert!(
                    y_velocity.signum() == direction,
                    "{preset:?} produced velocity {y_velocity}"
                );
            }
        }
    }

    #[test]
    fn each_gravity_axis_can_be_edited_without_changing_the_other_axes() {
        let mut app = gravity_app();
        app.update();
        request(
            &mut app,
            GravityAction::AdjustAxis {
                axis: GravityAxis::X,
                delta: 3.0,
            },
        );
        request(
            &mut app,
            GravityAction::AdjustAxis {
                axis: GravityAxis::Z,
                delta: -2.0,
            },
        );

        assert_eq!(
            app.world().resource::<Gravity>().0,
            Vec3::new(3.0, -9.81, -2.0)
        );
    }

    #[test]
    fn editing_an_axis_produces_directional_motion_immediately() {
        let mut app = gravity_app();
        let body = spawn_test_body(&mut app);
        request(&mut app, GravityAction::SetPreset(GravityPreset::ZeroG));
        request(
            &mut app,
            GravityAction::AdjustAxis {
                axis: GravityAxis::X,
                delta: 4.0,
            },
        );
        let before = app.world().entity(body).get::<Position>().unwrap().0;
        run_steps(&mut app, 30);
        let after = app.world().entity(body).get::<Position>().unwrap().0;

        assert!(
            after.x > before.x + 0.1,
            "body did not move along +X: {before:?} -> {after:?}"
        );
        assert!(
            (after.y - before.y).abs() < 0.01,
            "axis edit changed Y motion: {before:?} -> {after:?}"
        );
    }

    #[test]
    fn clicking_an_axis_control_updates_the_global_resource() {
        let mut app = gravity_app();
        app.update();
        let control = {
            let world = app.world_mut();
            let mut controls = world.query::<(Entity, &GravityControl)>();
            controls
                .iter(world)
                .find_map(|(entity, control)| {
                    matches!(
                        control,
                        GravityControl::Axis {
                            axis: GravityAxis::X,
                            adjustment: GravityAdjustment::Increase
                        }
                    )
                    .then_some(entity)
                })
                .expect("positive X gravity control")
        };

        *app.world_mut()
            .entity_mut(control)
            .get_mut::<Interaction>()
            .expect("button interaction") = Interaction::Pressed;
        app.update();

        assert_eq!(
            app.world().resource::<Gravity>().0,
            Vec3::new(0.5, -9.81, 0.0)
        );
    }
}
