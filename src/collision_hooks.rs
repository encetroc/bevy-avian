use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};

use crate::collision_layers::SandboxLayer;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const GHOST_COLOR: Color = Color::srgb(0.35, 0.65, 1.0);
const ACTIVE_GHOST_COLOR: Color = Color::srgb(1.0, 0.55, 0.2);
const ORDINARY_COLOR: Color = Color::srgb(0.25, 0.8, 0.35);

const GHOST_START: Vec3 = Vec3::new(-5.5, 0.7, 4.0);
const ORDINARY_POSITION: Vec3 = Vec3::new(0.0, 0.7, 4.0);
const GHOST_VELOCITY: Vec3 = Vec3::new(4.0, 0.0, 0.0);

/// State used by the collision-hook demonstration.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CollisionHookDemoState {
    /// When active, the ghost is allowed to collide with the ordinary body.
    pub activated: bool,
}

/// Headless-friendly controls exposed by the collision-hook station.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionHookDemoAction {
    ToggleActivation,
    ResetGhost,
}

#[derive(Component)]
struct CollisionHookDemoPanel;

#[derive(Component)]
struct CollisionHookDemoStatus;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum CollisionHookDemoControl {
    ToggleActivation,
    ResetGhost,
}

/// Marks the ghost collider whose pair is filtered by [`SandboxCollisionHooks`].
#[derive(Component)]
struct HookDemoGhost;

/// Marks the ordinary collider used only by the hook demonstration.
#[derive(Component)]
struct HookDemoOrdinary;

/// Owns the isolated custom collision-filtering demonstration.
pub struct CollisionHookDemoPlugin;

impl Plugin for CollisionHookDemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionHookDemoState>()
            .add_message::<CollisionHookDemoAction>()
            .add_systems(
                Startup,
                (spawn_collision_hook_demo, spawn_collision_hook_ui),
            )
            .add_systems(
                Update,
                (
                    queue_collision_hook_actions,
                    apply_collision_hook_actions,
                    update_collision_hook_ui,
                    draw_collision_hook_labels.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

/// The collision hook used by the sandbox's hook demonstration.
///
/// All pairs are accepted except the marked ghost/ordinary pair while the
/// demonstration is inactive. The marker queries keep this rule local to the
/// demonstration rather than changing collision behavior for other entities.
#[derive(SystemParam)]
pub struct SandboxCollisionHooks<'w, 's> {
    ghosts: Query<'w, 's, (), With<HookDemoGhost>>,
    ordinary: Query<'w, 's, (), With<HookDemoOrdinary>>,
    state: Res<'w, CollisionHookDemoState>,
}

impl CollisionHooks for SandboxCollisionHooks<'_, '_> {
    fn filter_pairs(&self, collider1: Entity, collider2: Entity, _commands: &mut Commands) -> bool {
        let is_demo_pair = (self.ghosts.get(collider1).is_ok()
            && self.ordinary.get(collider2).is_ok())
            || (self.ghosts.get(collider2).is_ok() && self.ordinary.get(collider1).is_ok());

        !is_demo_pair || self.state.activated
    }
}

fn demo_layers(membership: SandboxLayer, filter: SandboxLayer) -> CollisionLayers {
    CollisionLayers::new(membership, filter)
}

fn spawn_collision_hook_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let ghost_material = materials
        .as_mut()
        .map(|materials| materials.add(GHOST_COLOR));
    let ordinary_material = materials
        .as_mut()
        .map(|materials| materials.add(ORDINARY_COLOR));

    let ordinary = commands
        .spawn((
            HookDemoOrdinary,
            RigidBody::Static,
            Collider::cuboid(0.55, 0.7, 0.55),
            demo_layers(SandboxLayer::Objects, SandboxLayer::Ghost),
            Transform::from_translation(ORDINARY_POSITION),
            Name::new("Hook Demo Ordinary Body"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (mesh.as_ref(), ordinary_material) {
        commands.entity(ordinary).insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(ORDINARY_POSITION).with_scale(Vec3::new(1.1, 1.4, 1.1)),
        ));
    }

    let ghost = commands
        .spawn((
            HookDemoGhost,
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            demo_layers(SandboxLayer::Ghost, SandboxLayer::Objects),
            ActiveCollisionHooks::FILTER_PAIRS,
            GravityScale(0.0),
            SleepingDisabled,
            LinearVelocity(GHOST_VELOCITY),
            Transform::from_translation(GHOST_START),
            Name::new("Hook Demo Ghost"),
        ))
        .id();
    if let (Some(mesh), Some(material)) = (mesh, ghost_material) {
        commands.entity(ghost).insert((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(GHOST_START).with_scale(Vec3::splat(1.0)),
        ));
    }
}

fn spawn_collision_hook_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                bottom: px(16.0),
                width: px(390.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            CollisionHookDemoPanel,
            Name::new("Collision Hook Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("COLLISION HOOKS"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("The ghost passes through the ordinary body until activated."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Hook: INACTIVE — GHOST passes through"),
                TextFont::from_font_size(12.0),
                TextColor(GHOST_COLOR),
                CollisionHookDemoStatus,
            ));
            parent
                .spawn((
                    Button,
                    CollisionHookDemoControl::ToggleActivation,
                    Node {
                        width: px(220.0),
                        height: px(24.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Toggle collision hook activation"),
                ))
                .with_child((
                    Text::new("toggle hook activation"),
                    TextFont::from_font_size(10.0),
                    TextColor(PANEL_TEXT),
                ));
            parent
                .spawn((
                    Button,
                    CollisionHookDemoControl::ResetGhost,
                    Node {
                        width: px(120.0),
                        height: px(24.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Reset collision hook ghost"),
                ))
                .with_child((
                    Text::new("reset ghost"),
                    TextFont::from_font_size(10.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_collision_hook_actions(
    controls: Query<(&Interaction, &CollisionHookDemoControl), Changed<Interaction>>,
    mut actions: MessageWriter<CollisionHookDemoAction>,
) {
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match control {
            CollisionHookDemoControl::ToggleActivation => CollisionHookDemoAction::ToggleActivation,
            CollisionHookDemoControl::ResetGhost => CollisionHookDemoAction::ResetGhost,
        });
    }
}

fn apply_collision_hook_actions(
    mut actions: MessageReader<CollisionHookDemoAction>,
    mut state: ResMut<CollisionHookDemoState>,
    mut ghost: Query<(&mut Position, &mut Transform, &mut LinearVelocity), With<HookDemoGhost>>,
) {
    for action in actions.read() {
        match action {
            CollisionHookDemoAction::ToggleActivation => {
                state.activated = !state.activated;
                reset_ghost(&mut ghost);
            }
            CollisionHookDemoAction::ResetGhost => reset_ghost(&mut ghost),
        }
    }
}

fn reset_ghost(
    ghost: &mut Query<(&mut Position, &mut Transform, &mut LinearVelocity), With<HookDemoGhost>>,
) {
    for (mut position, mut transform, mut velocity) in ghost.iter_mut() {
        position.0 = GHOST_START;
        transform.translation = GHOST_START;
        velocity.0 = GHOST_VELOCITY;
    }
}

fn update_collision_hook_ui(
    state: Res<CollisionHookDemoState>,
    mut status: Query<&mut Text, With<CollisionHookDemoStatus>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<CollisionHookDemoControl>>,
) {
    if let Ok(mut status) = status.single_mut() {
        status.0 = if state.activated {
            "Hook: ACTIVE — GHOST collides".to_owned()
        } else {
            "Hook: INACTIVE — GHOST passes through".to_owned()
        };
    }

    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn draw_collision_hook_labels(
    mut gizmos: Gizmos,
    state: Res<CollisionHookDemoState>,
    ghosts: Query<&Transform, With<HookDemoGhost>>,
    ordinary: Query<&Transform, With<HookDemoOrdinary>>,
) {
    for transform in &ghosts {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 1.1, Quat::IDENTITY),
            "HOOK GHOST",
            0.22,
            Vec2::ZERO,
            if state.activated {
                ACTIVE_GHOST_COLOR
            } else {
                GHOST_COLOR
            },
        );
    }
    for transform in &ordinary {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 1.1, Quat::IDENTITY),
            "HOOK ORDINARY",
            0.22,
            Vec2::ZERO,
            ORDINARY_COLOR,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    #[derive(Component)]
    struct RegularBody;

    fn hook_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default().with_collision_hooks::<SandboxCollisionHooks>(),
            CollisionHookDemoPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
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

    fn ghost_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut ghosts = world.query_filtered::<Entity, With<HookDemoGhost>>();
        ghosts.iter(world).next().expect("hook demo ghost")
    }

    fn ghost_x(app: &mut App) -> f32 {
        let ghost = ghost_entity(app);
        app.world()
            .entity(ghost)
            .get::<Position>()
            .expect("ghost position")
            .0
            .x
    }

    #[test]
    fn inactive_hook_lets_the_ghost_pass_through() {
        let mut app = hook_app();
        app.update();
        run_steps(&mut app, 150);

        assert!(!app.world().resource::<CollisionHookDemoState>().activated);
        assert!(
            ghost_x(&mut app) > 2.0,
            "ghost should pass through the body"
        );
    }

    #[test]
    fn activating_hook_changes_the_ghost_collision_result() {
        let mut app = hook_app();
        app.update();
        run_steps(&mut app, 150);
        assert!(ghost_x(&mut app) > 2.0);

        app.world_mut()
            .resource_mut::<Messages<CollisionHookDemoAction>>()
            .write(CollisionHookDemoAction::ToggleActivation);
        app.update();
        run_steps(&mut app, 150);

        assert!(app.world().resource::<CollisionHookDemoState>().activated);
        assert!(
            ghost_x(&mut app) < -0.4,
            "active hook should stop the ghost at the ordinary body"
        );
    }

    #[test]
    fn ordinary_pairs_are_not_filtered_by_the_demo_hook() {
        let mut app = hook_app();
        app.update();

        let wall = app
            .world_mut()
            .spawn((
                RigidBody::Static,
                Collider::cuboid(0.55, 0.7, 0.55),
                Transform::from_xyz(0.0, 0.7, 1.0),
                Name::new("Regular Wall"),
            ))
            .id();
        let body = app
            .world_mut()
            .spawn((
                RegularBody,
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                GravityScale(0.0),
                SleepingDisabled,
                LinearVelocity(GHOST_VELOCITY),
                Transform::from_xyz(-5.5, 0.7, 1.0),
                Name::new("Regular Body"),
            ))
            .id();
        assert!(app.world().entity(wall).contains::<Collider>());

        run_steps(&mut app, 150);

        assert!(
            app.world().entity(body).get::<Position>().unwrap().0.x < -0.4,
            "ordinary body should still collide with an ordinary wall"
        );
    }
}
