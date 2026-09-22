use avian3d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

/// The color used for Avian's collider wireframes when debug rendering is on.
const COLLIDER_DEBUG_COLOR: Color = Color::srgb(1.0, 0.35, 0.1);
const JOINT_ANCHOR_DEBUG_COLOR: Color = Color::srgb(1.0, 0.2, 0.75);
const JOINT_SEPARATION_DEBUG_COLOR: Color = Color::srgb(1.0, 0.1, 0.1);

/// Runtime state for the collider debug rendering toggle.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ColliderDebugSettings {
    /// Whether Avian collider wireframes are currently visible.
    pub enabled: bool,
}

/// Owns Avian's collider debug renderer and its F1 keyboard toggle.
pub struct ColliderDebugPlugin;

impl Plugin for ColliderDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsDebugPlugin)
            .insert_resource(ColliderDebugSettings::default())
            // Keep the normal meshes visible while using Avian's actual collider
            // shapes for the debug overlay. The negative bias keeps wireframes
            // readable when they overlap the visible meshes.
            .insert_gizmo_config(
                PhysicsGizmos::none(),
                GizmoConfig {
                    depth_bias: -0.01,
                    ..default()
                },
            )
            .add_systems(PreUpdate, toggle_collider_debug.after(InputSystems));
    }
}

fn toggle_collider_debug(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ColliderDebugSettings>,
    mut gizmo_store: ResMut<GizmoConfigStore>,
) {
    if !keyboard_input.just_pressed(KeyCode::F1) {
        return;
    }

    settings.enabled = !settings.enabled;
    let (_, physics_gizmos) = gizmo_store.config_mut::<PhysicsGizmos>();
    physics_gizmos.collider_color = settings.enabled.then_some(COLLIDER_DEBUG_COLOR);
    physics_gizmos.joint_anchor_color = settings.enabled.then_some(JOINT_ANCHOR_DEBUG_COLOR);
    physics_gizmos.joint_separation_color =
        settings.enabled.then_some(JOINT_SEPARATION_DEBUG_COLOR);
}

#[cfg(test)]
mod tests {
    use bevy::{
        gizmos::GizmoPlugin,
        input::{
            ButtonState, InputPlugin,
            keyboard::{Key, KeyboardInput},
        },
        mesh::MeshPlugin,
    };

    use super::*;

    fn debug_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            GizmoPlugin,
            PhysicsPlugins::default(),
            ColliderDebugPlugin,
        ));
        app.finish();
        app
    }

    fn send_f1(app: &mut App, state: ButtonState) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::F1,
                logical_key: Key::F1,
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn physics_gizmos(app: &App) -> &PhysicsGizmos {
        app.world()
            .resource::<GizmoConfigStore>()
            .config::<PhysicsGizmos>()
            .1
    }

    fn collider_color(app: &App) -> Option<Color> {
        physics_gizmos(app).collider_color
    }

    #[test]
    fn collider_debug_starts_disabled_without_hiding_meshes() {
        let mut app = debug_app();
        app.update();

        assert_eq!(
            app.world().resource::<ColliderDebugSettings>(),
            &ColliderDebugSettings::default()
        );
        assert_eq!(collider_color(&app), None);
        let gizmos = physics_gizmos(&app);
        assert_eq!(gizmos.joint_anchor_color, None);
        assert_eq!(gizmos.joint_separation_color, None);
        assert!(!gizmos.hide_meshes);
    }

    #[test]
    fn f1_toggles_avian_collider_wireframes_repeatedly() {
        let mut app = debug_app();
        app.update();

        send_f1(&mut app, ButtonState::Pressed);
        assert!(app.world().resource::<ColliderDebugSettings>().enabled);
        let gizmos = physics_gizmos(&app);
        assert_eq!(gizmos.collider_color, Some(COLLIDER_DEBUG_COLOR));
        assert_eq!(gizmos.joint_anchor_color, Some(JOINT_ANCHOR_DEBUG_COLOR));
        assert_eq!(
            gizmos.joint_separation_color,
            Some(JOINT_SEPARATION_DEBUG_COLOR)
        );

        send_f1(&mut app, ButtonState::Released);
        send_f1(&mut app, ButtonState::Pressed);
        assert!(!app.world().resource::<ColliderDebugSettings>().enabled);
        let gizmos = physics_gizmos(&app);
        assert_eq!(gizmos.collider_color, None);
        assert_eq!(gizmos.joint_anchor_color, None);
        assert_eq!(gizmos.joint_separation_color, None);

        send_f1(&mut app, ButtonState::Released);
        send_f1(&mut app, ButtonState::Pressed);
        assert!(app.world().resource::<ColliderDebugSettings>().enabled);
        let gizmos = physics_gizmos(&app);
        assert_eq!(gizmos.collider_color, Some(COLLIDER_DEBUG_COLOR));
        assert_eq!(gizmos.joint_anchor_color, Some(JOINT_ANCHOR_DEBUG_COLOR));
        assert_eq!(
            gizmos.joint_separation_color,
            Some(JOINT_SEPARATION_DEBUG_COLOR)
        );
    }
}
