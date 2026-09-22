use avian3d::prelude::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default(), SandboxPlugin))
        .run();
}

/// Owns the initial world for the physics sandbox.
///
/// The sandbox starts with only the presentation entities needed to view a 3D
/// world. Physics geometry and interactive bodies are added by later features.
struct SandboxPlugin;

impl Plugin for SandboxPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.08)))
            .add_systems(Startup, spawn_sandbox_scene);
    }
}

fn spawn_sandbox_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 14.0).looking_at(Vec3::ZERO, Dir3::Y),
        Name::new("Sandbox Camera"),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 5_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-1.0, -2.0, -1.5), Dir3::Y),
        Name::new("Sandbox Light"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avian_plugins_initialize_physics_resources() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsPlugins::default()));

        assert!(app.world().contains_resource::<Gravity>());
        assert!(app.world().contains_resource::<Time<Physics>>());
    }

    #[test]
    fn sandbox_plugin_spawns_the_initial_3d_scene() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SandboxPlugin));
        app.update();

        let world = app.world_mut();
        let mut cameras = world.query_filtered::<Entity, With<Camera3d>>();
        assert_eq!(cameras.iter(world).count(), 1);

        let mut lights = world.query_filtered::<Entity, With<DirectionalLight>>();
        assert_eq!(lights.iter(world).count(), 1);
    }
}
