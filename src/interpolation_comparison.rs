use avian3d::prelude::*;
use bevy::prelude::*;

const MODE_KEY: KeyCode = KeyCode::KeyV;
const COMPARISON_START: Vec3 = Vec3::new(-8.0, 2.0, 8.0);
const REFERENCE_START: Vec3 = Vec3::new(-8.0, 2.0, 9.0);
const BODY_SPEED: f32 = 15.0;

/// Rendering-only easing selection for the fast-moving comparison body.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TransformDisplayMode {
    #[default]
    None,
    Interpolation,
    Extrapolation,
}

impl TransformDisplayMode {
    const fn next(self) -> Self {
        match self {
            Self::None => Self::Interpolation,
            Self::Interpolation => Self::Extrapolation,
            Self::Extrapolation => Self::None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Interpolation => "INTERPOLATION",
            Self::Extrapolation => "EXTRAPOLATION",
        }
    }
}

#[derive(Component)]
struct InterpolationComparisonBody;

#[derive(Component)]
struct InterpolationReferenceBody;

#[derive(Component)]
struct InterpolationComparisonStatus;

/// Demonstrates fast rigid-body motion with a raw reference and runtime-selectable easing.
pub struct InterpolationComparisonPlugin;

impl Plugin for InterpolationComparisonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransformDisplayMode>()
            .add_systems(Startup, (spawn_comparison_bodies, spawn_comparison_help))
            .add_systems(
                Update,
                (
                    handle_mode_input,
                    apply_display_mode,
                    update_comparison_status,
                )
                    .chain(),
            );
    }
}

fn spawn_comparison_bodies(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Sphere::new(0.35)));
    let comparison_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.95, 0.48, 0.12)));
    let reference_material = materials
        .as_mut()
        .map(|materials| materials.add(Color::srgb(0.25, 0.8, 0.95)));

    for (name, marker, position, material) in [
        (
            "Interpolation Comparison Body",
            true,
            COMPARISON_START,
            comparison_material.as_ref(),
        ),
        (
            "Interpolation Raw Reference Body",
            false,
            REFERENCE_START,
            reference_material.as_ref(),
        ),
    ] {
        let mut body = commands.spawn((
            RigidBody::Dynamic,
            Collider::sphere(0.35),
            GravityScale(0.0),
            SleepingDisabled,
            LinearVelocity(Vec3::X * BODY_SPEED),
            Transform::from_translation(position),
            Name::new(name),
        ));
        // Keep the two lanes visually and physically independent.
        if marker {
            body.insert(InterpolationComparisonBody);
        } else {
            body.insert(InterpolationReferenceBody);
        }
        if let (Some(mesh), Some(material)) = (mesh.as_ref(), material) {
            body.insert((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
        }
    }
}

fn spawn_comparison_help(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            bottom: px(16),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.85)),
        Text::new("FAST MOTION: BLUE = RAW REFERENCE  |  ORANGE = SELECTED MODE\nV: CYCLE NONE / INTERPOLATION / EXTRAPOLATION"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        InterpolationComparisonStatus,
    ));
}

fn handle_mode_input(world: &mut World) {
    let advance = world
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(MODE_KEY);
    if advance {
        let next = world.resource::<TransformDisplayMode>().next();
        *world.resource_mut::<TransformDisplayMode>() = next;
    }
}

fn apply_display_mode(
    mut commands: Commands,
    mode: Res<TransformDisplayMode>,
    bodies: Query<
        (
            Entity,
            Option<&TransformInterpolation>,
            Option<&TransformExtrapolation>,
        ),
        With<InterpolationComparisonBody>,
    >,
) {
    if !mode.is_changed() {
        return;
    }

    for (entity, interpolation, extrapolation) in &bodies {
        let mut body = commands.entity(entity);
        if interpolation.is_some() {
            body.remove::<TransformInterpolation>();
        }
        if extrapolation.is_some() {
            body.remove::<TransformExtrapolation>();
        }
        match *mode {
            TransformDisplayMode::None => {}
            TransformDisplayMode::Interpolation => {
                body.insert(TransformInterpolation);
            }
            TransformDisplayMode::Extrapolation => {
                body.insert(TransformExtrapolation);
            }
        }
    }
}

fn update_comparison_status(
    mode: Res<TransformDisplayMode>,
    mut labels: Query<&mut Text, With<InterpolationComparisonStatus>>,
) {
    if !mode.is_changed() {
        return;
    }
    for mut label in &mut labels {
        **label = format!(
            "FAST MOTION: BLUE = RAW REFERENCE  |  ORANGE = {}\nV: CYCLE NONE / INTERPOLATION / EXTRAPOLATION",
            mode.label()
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;
    use crate::physics_controls::PhysicsControlsPlugin;

    fn comparison_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            InputPlugin,
            PhysicsPlugins::default(),
            PhysicsControlsPlugin,
            InterpolationComparisonPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn entity_with<T: Component>(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut entities = world.query_filtered::<Entity, With<T>>();
        entities.iter(world).next().expect("comparison entity")
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        handle_mode_input(app.world_mut());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        app.update();
    }

    #[test]
    fn spawns_fast_moving_comparison_and_unmodified_reference_bodies() {
        let mut app = comparison_app();
        app.update();

        let comparison = entity_with::<InterpolationComparisonBody>(&mut app);
        let reference = entity_with::<InterpolationReferenceBody>(&mut app);
        for entity in [comparison, reference] {
            let body = app.world().entity(entity);
            assert_eq!(body.get::<RigidBody>(), Some(&RigidBody::Dynamic));
            assert_eq!(
                body.get::<LinearVelocity>().unwrap().0,
                Vec3::X * BODY_SPEED
            );
            assert!(body.contains::<Collider>());
        }
        assert!(
            !app.world()
                .entity(reference)
                .contains::<TransformInterpolation>()
        );
        assert!(
            !app.world()
                .entity(reference)
                .contains::<TransformExtrapolation>()
        );
    }

    #[test]
    fn runtime_mode_changes_only_render_easing_not_physics_state() {
        let mut app = comparison_app();
        app.update();
        let comparison = entity_with::<InterpolationComparisonBody>(&mut app);
        let reference = entity_with::<InterpolationReferenceBody>(&mut app);
        let initial_physics_position = app.world().entity(comparison).get::<Position>().unwrap().0;

        press(&mut app, MODE_KEY);
        assert_eq!(
            *app.world().resource::<TransformDisplayMode>(),
            TransformDisplayMode::Interpolation
        );
        assert!(
            app.world()
                .entity(comparison)
                .contains::<TransformInterpolation>()
        );
        assert!(
            !app.world()
                .entity(comparison)
                .contains::<TransformExtrapolation>()
        );
        assert!(
            !app.world()
                .entity(reference)
                .contains::<TransformInterpolation>()
        );
        assert!(
            app.world()
                .entity(comparison)
                .get::<Position>()
                .unwrap()
                .0
                .x
                > initial_physics_position.x
        );

        press(&mut app, MODE_KEY);
        assert_eq!(
            *app.world().resource::<TransformDisplayMode>(),
            TransformDisplayMode::Extrapolation
        );
        assert!(
            !app.world()
                .entity(comparison)
                .contains::<TransformInterpolation>()
        );
        assert!(
            app.world()
                .entity(comparison)
                .contains::<TransformExtrapolation>()
        );
        assert!(
            !app.world()
                .entity(reference)
                .contains::<TransformExtrapolation>()
        );
    }

    #[test]
    fn changing_render_mode_while_paused_does_not_advance_physics() {
        let mut app = comparison_app();
        app.update();
        let comparison = entity_with::<InterpolationComparisonBody>(&mut app);
        app.world_mut().resource_mut::<Time<Physics>>().pause();
        let paused_at = app.world().entity(comparison).get::<Position>().unwrap().0;

        for _ in 0..3 {
            press(&mut app, MODE_KEY);
            assert_eq!(
                app.world().entity(comparison).get::<Position>().unwrap().0,
                paused_at
            );
        }
        assert!(app.world().resource::<Time<Physics>>().is_paused());
    }
}
