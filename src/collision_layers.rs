use avian3d::prelude::*;
use bevy::prelude::*;

use crate::player::Player;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const RED: Color = Color::srgb(0.95, 0.2, 0.18);
const BLUE: Color = Color::srgb(0.2, 0.42, 0.98);
const GOLD: Color = Color::srgb(0.98, 0.72, 0.22);
const GREEN: Color = Color::srgb(0.25, 0.8, 0.35);
const CYAN: Color = Color::srgba(0.15, 0.8, 1.0, 0.26);

const DEFAULT_BIT: u32 = 1 << 0;
const PLAYER_BIT: u32 = 1 << 1;
const WORLD_BIT: u32 = 1 << 2;
const OBJECTS_BIT: u32 = 1 << 3;
const SENSORS_BIT: u32 = 1 << 4;
const PROJECTILES_BIT: u32 = 1 << 5;
const PLAYER_FILTER: u32 = DEFAULT_BIT | WORLD_BIT | OBJECTS_BIT | SENSORS_BIT | PROJECTILES_BIT;
const WORLD_FILTER: u32 = DEFAULT_BIT | PLAYER_BIT | OBJECTS_BIT | PROJECTILES_BIT;
const OBJECTS_FILTER: u32 =
    DEFAULT_BIT | PLAYER_BIT | WORLD_BIT | OBJECTS_BIT | SENSORS_BIT | PROJECTILES_BIT;
const SENSOR_FILTER: u32 = DEFAULT_BIT | PLAYER_BIT | OBJECTS_BIT | PROJECTILES_BIT;
const PROJECTILE_FILTER: u32 = DEFAULT_BIT | PLAYER_BIT | WORLD_BIT | OBJECTS_BIT | SENSORS_BIT;
const GHOST_FILTER: u32 = DEFAULT_BIT | PLAYER_BIT | OBJECTS_BIT | PROJECTILES_BIT;

/// Collision categories used by the sandbox's filtering demonstrations.
///
/// `Default` is Avian's reserved fallback layer. The six variants after it
/// are the categories exposed by the station.
#[derive(PhysicsLayer, Default, Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxLayer {
    #[default]
    Default,
    Player,
    World,
    Objects,
    Sensors,
    Projectiles,
    Ghost,
}

impl SandboxLayer {
    /// The six categories that are shown by the demonstration station.
    pub const DEMONSTRATED: [Self; 6] = [
        Self::Player,
        Self::World,
        Self::Objects,
        Self::Sensors,
        Self::Projectiles,
        Self::Ghost,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "DEFAULT",
            Self::Player => "PLAYER",
            Self::World => "WORLD",
            Self::Objects => "OBJECTS",
            Self::Sensors => "SENSORS",
            Self::Projectiles => "PROJECTILES",
            Self::Ghost => "GHOST",
        }
    }
}

/// Returns the normal membership and filter configuration for one category.
pub fn layers_for(category: SandboxLayer) -> CollisionLayers {
    let filters = match category {
        SandboxLayer::Default => u32::MAX,
        SandboxLayer::Player => PLAYER_FILTER,
        SandboxLayer::World => WORLD_FILTER,
        SandboxLayer::Objects => OBJECTS_FILTER,
        SandboxLayer::Sensors => SENSOR_FILTER,
        SandboxLayer::Projectiles => PROJECTILE_FILTER,
        SandboxLayer::Ghost => GHOST_FILTER,
    };
    CollisionLayers::from_bits(category.to_bits(), filters)
}

/// Returns the projectile configuration, optionally allowing it to hit ghosts.
pub fn projectile_layers(collide_with_ghost: bool) -> CollisionLayers {
    let mut layers = layers_for(SandboxLayer::Projectiles);
    if collide_with_ghost {
        layers.filters |= SandboxLayer::Ghost;
    }
    layers
}

/// One labelled object in the collision-layer laboratory.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollisionLayerDemoObject {
    pub category: SandboxLayer,
    pub label: &'static str,
}

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CollisionLayerDemoState {
    pub projectile_collides_with_ghost: bool,
}

/// Headless-friendly controls exposed by the collision-layer station.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionLayerAction {
    ToggleProjectileGhost,
}

#[derive(Component)]
struct CollisionLayerDemoPanel;

#[derive(Component)]
struct CollisionLayerDemoStatus;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum CollisionLayerDemoControl {
    ToggleProjectileGhost,
}

#[derive(Component)]
struct CollisionLayerDemoProjectile;

/// Owns the collision-layer categories and the red/blue wall demonstration.
pub struct CollisionLayerDemoPlugin;

impl Plugin for CollisionLayerDemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionLayerDemoState>()
            .add_message::<CollisionLayerAction>()
            .add_systems(
                Startup,
                (spawn_collision_layer_demo, spawn_collision_layer_ui),
            )
            .add_systems(
                Update,
                (
                    queue_collision_layer_actions,
                    apply_collision_layer_actions,
                    update_collision_layer_ui,
                    draw_collision_layer_labels.run_if(resource_exists::<GizmoConfigStore>),
                )
                    .chain(),
            );
    }
}

const RED_WALL_POSITION: Vec3 = Vec3::new(0.0, 1.0, 8.0);
const BLUE_WALL_POSITION: Vec3 = Vec3::new(0.0, 1.0, 9.0);
const RED_PROJECTILE_POSITION: Vec3 = Vec3::new(-3.0, 1.0, 8.0);
const BLUE_PROJECTILE_POSITION: Vec3 = Vec3::new(-3.0, 1.0, 9.0);

fn spawn_collision_layer_demo(
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let mesh = meshes.as_mut().map(|meshes| meshes.add(Cuboid::default()));
    let material = |materials: &mut Option<ResMut<Assets<StandardMaterial>>>, color: Color| {
        materials.as_mut().map(|materials| materials.add(color))
    };

    spawn_demo_object(
        &mut commands,
        "Layer Demo Player",
        "PLAYER",
        SandboxLayer::Player,
        RigidBody::Static,
        Collider::cuboid(0.7, 1.2, 0.7),
        Transform::from_xyz(-2.7, 0.7, 7.2),
        mesh.as_ref(),
        material(&mut materials, GOLD),
        false,
    );
    spawn_demo_object(
        &mut commands,
        "Red World Wall",
        "WORLD — RED WALL",
        SandboxLayer::World,
        RigidBody::Static,
        Collider::cuboid(0.25, 1.8, 0.42),
        Transform::from_translation(RED_WALL_POSITION),
        mesh.as_ref(),
        material(&mut materials, RED),
        false,
    );
    spawn_demo_object(
        &mut commands,
        "Objects Sample Crate",
        "OBJECTS",
        SandboxLayer::Objects,
        RigidBody::Dynamic,
        Collider::cuboid(0.7, 0.7, 0.7),
        Transform::from_xyz(2.5, 0.7, 7.2),
        mesh.as_ref(),
        material(&mut materials, GREEN),
        false,
    );
    spawn_demo_object(
        &mut commands,
        "Sensors Sample Volume",
        "SENSORS",
        SandboxLayer::Sensors,
        RigidBody::Static,
        Collider::cuboid(0.65, 1.0, 0.65),
        Transform::from_xyz(2.5, 1.0, 8.5),
        mesh.as_ref(),
        material(&mut materials, CYAN),
        true,
    );
    spawn_demo_object(
        &mut commands,
        "Blue Ghost Wall",
        "GHOST — BLUE WALL",
        SandboxLayer::Ghost,
        RigidBody::Static,
        Collider::cuboid(0.25, 1.8, 0.42),
        Transform::from_translation(BLUE_WALL_POSITION),
        mesh.as_ref(),
        material(&mut materials, BLUE),
        false,
    );

    for (name, position, label) in [
        (
            "Red Projectile",
            RED_PROJECTILE_POSITION,
            "PROJECTILES — RED LANE",
        ),
        (
            "Blue Projectile",
            BLUE_PROJECTILE_POSITION,
            "PROJECTILES — BLUE LANE",
        ),
    ] {
        let entity = spawn_demo_object(
            &mut commands,
            name,
            label,
            SandboxLayer::Projectiles,
            RigidBody::Dynamic,
            Collider::cuboid(0.45, 0.45, 0.45),
            Transform::from_translation(position),
            mesh.as_ref(),
            material(&mut materials, GOLD),
            false,
        );
        commands.entity(entity).insert((
            CollisionLayerDemoProjectile,
            GravityScale(0.0),
            SleepingDisabled,
            LinearVelocity(Vec3::new(3.0, 0.0, 0.0)),
        ));
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "each demonstration object declares its complete physics and presentation setup"
)]
fn spawn_demo_object(
    commands: &mut Commands,
    name: &'static str,
    label: &'static str,
    category: SandboxLayer,
    body: RigidBody,
    collider: Collider,
    transform: Transform,
    mesh: Option<&Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
    sensor: bool,
) -> Entity {
    let mut entity = commands.spawn((
        CollisionLayerDemoObject { category, label },
        if category == SandboxLayer::Projectiles {
            projectile_layers(false)
        } else {
            layers_for(category)
        },
        body,
        collider,
        CollisionEventsEnabled,
        transform,
        Name::new(name),
    ));
    if let (Some(mesh), Some(material)) = (mesh, material) {
        entity.insert((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(transform.translation).with_scale(Vec3::splat(1.0)),
        ));
    }
    if sensor {
        entity.insert(Sensor);
    }
    entity.id()
}

fn spawn_collision_layer_ui(mut commands: Commands) {
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
            CollisionLayerDemoPanel,
            Name::new("Collision Layer Controls"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("COLLISION LAYERS"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new(
                    SandboxLayer::DEMONSTRATED
                        .iter()
                        .map(|category| category.label())
                        .collect::<Vec<_>>()
                        .join(" · "),
                ),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Red WORLD wall blocks projectiles; blue GHOST wall is pass-through."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Projectile → GHOST: PASS THROUGH"),
                TextFont::from_font_size(12.0),
                TextColor(BLUE),
                CollisionLayerDemoStatus,
            ));
            parent
                .spawn((
                    Button,
                    CollisionLayerDemoControl::ToggleProjectileGhost,
                    Node {
                        width: px(220.0),
                        height: px(24.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Toggle projectile ghost mask"),
                ))
                .with_child((
                    Text::new("toggle PROJECTILES ↔ GHOST"),
                    TextFont::from_font_size(10.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_collision_layer_actions(
    controls: Query<(&Interaction, &CollisionLayerDemoControl), Changed<Interaction>>,
    mut actions: MessageWriter<CollisionLayerAction>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            match control {
                CollisionLayerDemoControl::ToggleProjectileGhost => {
                    actions.write(CollisionLayerAction::ToggleProjectileGhost);
                }
            }
        }
    }
}

fn apply_collision_layer_actions(
    mut actions: MessageReader<CollisionLayerAction>,
    mut state: ResMut<CollisionLayerDemoState>,
    projectiles: Query<Entity, With<CollisionLayerDemoProjectile>>,
    mut commands: Commands,
) {
    for action in actions.read() {
        match action {
            CollisionLayerAction::ToggleProjectileGhost => {
                state.projectile_collides_with_ghost = !state.projectile_collides_with_ghost;
                let layers = projectile_layers(state.projectile_collides_with_ghost);
                for entity in &projectiles {
                    commands.entity(entity).insert(layers);
                }
            }
        }
    }
}

fn update_collision_layer_ui(
    state: Res<CollisionLayerDemoState>,
    mut status: Query<&mut Text, With<CollisionLayerDemoStatus>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<CollisionLayerDemoControl>>,
) {
    if let Ok(mut status) = status.single_mut() {
        status.0 = if state.projectile_collides_with_ghost {
            "Projectile → GHOST: COLLIDE".to_owned()
        } else {
            "Projectile → GHOST: PASS THROUGH".to_owned()
        };
    }

    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn draw_collision_layer_labels(
    mut gizmos: Gizmos,
    demo_objects: Query<(&CollisionLayerDemoObject, &Transform)>,
    players: Query<&Transform, With<Player>>,
) {
    for (object, transform) in &demo_objects {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 1.25, Quat::IDENTITY),
            object.label,
            0.22,
            Vec2::ZERO,
            match object.category {
                SandboxLayer::World => RED,
                SandboxLayer::Ghost => BLUE,
                SandboxLayer::Sensors => CYAN,
                _ => GOLD,
            },
        );
    }
    for transform in &players {
        gizmos.text(
            Isometry3d::new(transform.translation + Vec3::Y * 1.4, Quat::IDENTITY),
            "PLAYER",
            0.2,
            Vec2::ZERO,
            GOLD,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn demo_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CollisionLayerDemoPlugin,
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

    fn projectile_entity(app: &mut App, name: &str) -> Entity {
        let world = app.world_mut();
        let mut projectiles =
            world.query_filtered::<(Entity, &Name), With<CollisionLayerDemoProjectile>>();
        projectiles
            .iter(world)
            .find(|(_, projectile_name)| projectile_name.as_str() == name)
            .map(|(entity, _)| entity)
            .expect("named demo projectile")
    }

    fn projectile_x(app: &mut App, name: &str) -> f32 {
        let entity = projectile_entity(app, name);
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("projectile position")
            .0
            .x
    }

    fn reset_projectile(app: &mut App, name: &str) {
        let entity = projectile_entity(app, name);
        let position = if name == "Blue Projectile" {
            BLUE_PROJECTILE_POSITION
        } else {
            RED_PROJECTILE_POSITION
        };
        app.world_mut().entity_mut(entity).insert((
            Position(position),
            Transform::from_translation(position),
            LinearVelocity(Vec3::new(3.0, 0.0, 0.0)),
        ));
    }

    #[test]
    fn all_six_collision_categories_spawn_with_visible_labels() {
        let mut app = demo_app();
        app.update();

        let world = app.world_mut();
        let mut objects = world.query::<&CollisionLayerDemoObject>();
        let categories: Vec<_> = objects.iter(world).map(|object| object.category).collect();
        for category in SandboxLayer::DEMONSTRATED {
            assert!(categories.contains(&category), "missing {category:?}");
        }
        assert!(objects.iter(world).all(|object| !object.label.is_empty()));
    }

    #[test]
    fn configured_pairs_collide_or_pass_through_as_expected() {
        assert!(layers_for(SandboxLayer::Player).interacts_with(layers_for(SandboxLayer::World)));
        assert!(layers_for(SandboxLayer::Objects).interacts_with(layers_for(SandboxLayer::World)));
        assert!(
            layers_for(SandboxLayer::Projectiles).interacts_with(layers_for(SandboxLayer::World))
        );
        assert!(
            layers_for(SandboxLayer::Projectiles).interacts_with(layers_for(SandboxLayer::Sensors))
        );
        assert!(
            !layers_for(SandboxLayer::Projectiles).interacts_with(layers_for(SandboxLayer::Ghost))
        );
        assert!(projectile_layers(true).interacts_with(layers_for(SandboxLayer::Ghost)));
    }

    #[test]
    fn red_world_wall_blocks_while_blue_ghost_wall_passes_through() {
        let mut app = demo_app();
        app.update();
        run_steps(&mut app, 120);

        assert!(
            projectile_x(&mut app, "Red Projectile") < -0.1,
            "world wall should stop the red projectile"
        );
        assert!(
            projectile_x(&mut app, "Blue Projectile") > 0.8,
            "ghost wall should let the blue projectile pass"
        );
    }

    #[test]
    fn changing_the_projectile_mask_changes_the_ghost_wall_result() {
        let mut app = demo_app();
        app.update();
        run_steps(&mut app, 120);
        assert!(projectile_x(&mut app, "Blue Projectile") > 0.8);

        app.world_mut()
            .resource_mut::<Messages<CollisionLayerAction>>()
            .write(CollisionLayerAction::ToggleProjectileGhost);
        app.update();
        let blue = projectile_entity(&mut app, "Blue Projectile");
        assert_eq!(
            app.world().entity(blue).get::<CollisionLayers>(),
            Some(&projectile_layers(true))
        );

        reset_projectile(&mut app, "Blue Projectile");
        run_steps(&mut app, 120);
        assert!(
            projectile_x(&mut app, "Blue Projectile") < -0.1,
            "adding the ghost mask should make the blue wall block the projectile"
        );
    }
}
