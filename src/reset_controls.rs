use avian3d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

use crate::{
    counted_body_spawning::{CountedBody, SelectedStressMode, SpawnedBodyCount, StressObjectKind},
    joint_creation::{JointCreationState, RuntimeCreatedJoint},
    object_composition::{ComposedObject, CompositionJoint, CompositionState},
    object_inspector::SelectionState,
    object_spawn_palette::SpawnedSandboxObject,
    player::{PLAYER_START_POSITION, Player},
    stations::{ResetStation, Station, StationObject},
};

const STATION_CODES: [char; 10] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J'];

/// Owns the global keyboard shortcuts for resetting sandbox experiments.
pub struct ResetControlsPlugin;

impl Plugin for ResetControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, handle_reset_keys.after(InputSystems))
            .add_systems(Update, reset_station_spawned_objects);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the full-reset command coordinates state owned by existing sandbox plugins"
)]
#[expect(
    clippy::type_complexity,
    reason = "the player query pair separates read-only station selection from reset mutation"
)]
fn handle_reset_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    stations: Query<(&Station, &Transform), Without<Player>>,
    mut resets: MessageWriter<ResetStation>,
    mut commands: Commands,
    created_joints: Query<Entity, Or<(With<RuntimeCreatedJoint>, With<CompositionJoint>)>>,
    composed_objects: Query<Entity, With<ComposedObject>>,
    selection: Option<ResMut<SelectionState>>,
    joint_creation: Option<ResMut<JointCreationState>>,
    composition: Option<ResMut<CompositionState>>,
    gravity: Option<ResMut<Gravity>>,
    physics_time: Option<ResMut<Time<Physics>>>,
    substeps: Option<ResMut<SubstepCount>>,
    spawned_body_count: Option<ResMut<SpawnedBodyCount>>,
    selected_stress_mode: Option<ResMut<SelectedStressMode>>,
    mut player_transforms: ParamSet<(
        Query<&Transform, With<Player>>,
        Query<
            (
                &mut Transform,
                Option<&mut Position>,
                Option<&mut LinearVelocity>,
            ),
            With<Player>,
        >,
    )>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }

    let full_reset = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if full_reset {
        for code in STATION_CODES {
            resets.write(ResetStation { code });
        }
        if let Some(mut count) = spawned_body_count {
            count.0 = 0;
        }
        if let Some(mut mode) = selected_stress_mode {
            mode.0 = StressObjectKind::default();
        }
        for entity in &created_joints {
            commands.entity(entity).despawn();
        }
        for entity in &composed_objects {
            commands.entity(entity).remove::<ComposedObject>();
        }
        if let Some(mut selection) = selection {
            selection.entity = None;
        }
        if let Some(mut state) = joint_creation {
            *state = JointCreationState::default();
        }
        if let Some(mut state) = composition {
            *state = CompositionState::default();
        }
        if let Some(mut gravity) = gravity {
            gravity.0 = Vec3::new(0.0, -9.81, 0.0);
        }
        if let Some(mut physics_time) = physics_time {
            physics_time.set_relative_speed(1.0);
            physics_time.unpause();
        }
        if let Some(mut substeps) = substeps {
            substeps.0 = 6;
        }
        for (mut transform, position, velocity) in &mut player_transforms.p1() {
            transform.translation = PLAYER_START_POSITION;
            if let Some(mut position) = position {
                position.0 = PLAYER_START_POSITION;
            }
            if let Some(mut velocity) = velocity {
                velocity.0 = Vec3::ZERO;
            }
        }
    } else {
        let Some(player_position) = player_transforms
            .p0()
            .iter()
            .next()
            .map(|player| player.translation)
        else {
            return;
        };
        let current_station = stations
            .iter()
            .min_by(|(_, left), (_, right)| {
                horizontal_distance_squared(left.translation, player_position).total_cmp(
                    &horizontal_distance_squared(right.translation, player_position),
                )
            })
            .map(|(station, _)| station.code);
        if let Some(code) = current_station {
            resets.write(ResetStation { code });
        }
    }
}

#[expect(
    clippy::type_complexity,
    reason = "the marker filter selects only user-spawned station-owned bodies"
)]
fn reset_station_spawned_objects(
    mut commands: Commands,
    mut resets: MessageReader<ResetStation>,
    spawned: Query<(Entity, &StationObject), Or<(With<SpawnedSandboxObject>, With<CountedBody>)>>,
    spawned_body_count: Option<ResMut<SpawnedBodyCount>>,
    selected_stress_mode: Option<ResMut<SelectedStressMode>>,
) {
    let requested: std::collections::HashSet<_> = resets.read().map(|reset| reset.code).collect();
    if requested.is_empty() {
        return;
    }
    for (entity, object) in &spawned {
        if requested.contains(&object.station) {
            commands.entity(entity).despawn();
        }
    }
    if requested.contains(&'I') {
        if let Some(mut count) = spawned_body_count {
            count.0 = 0;
        }
        if let Some(mut mode) = selected_stress_mode {
            mode.0 = StressObjectKind::default();
        }
    }
}

fn horizontal_distance_squared(left: Vec3, right: Vec3) -> f32 {
    let delta = left - right;
    delta.x * delta.x + delta.z * delta.z
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
    use crate::{
        collision_layers::{SandboxLayer, layers_for},
        stations::{StationLayoutPlugin, StationObject},
    };

    fn reset_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            StationLayoutPlugin,
            ResetControlsPlugin,
        ))
        .init_resource::<SelectionState>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn key(app: &mut App, key: KeyCode, state: ButtonState) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
    }

    fn press_reset(app: &mut App, shift: bool) {
        if shift {
            key(app, KeyCode::ShiftLeft, ButtonState::Pressed);
        }
        key(app, KeyCode::KeyR, ButtonState::Pressed);
        app.update();
        key(app, KeyCode::KeyR, ButtonState::Released);
        if shift {
            key(app, KeyCode::ShiftLeft, ButtonState::Released);
        }
        app.update();
    }

    fn station_object(app: &mut App, code: char, initial: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                StationObject::new(code, Transform::from_translation(initial)),
                Transform::from_translation(initial),
                Position(initial),
                LinearVelocity::default(),
                AngularVelocity::default(),
            ))
            .id()
    }

    #[test]
    fn r_resets_only_the_nearest_station() {
        let mut app = reset_app();
        app.update();
        let a = station_object(&mut app, 'A', Vec3::new(-6.0, 1.0, -6.0));
        let b = station_object(&mut app, 'B', Vec3::new(0.0, 1.0, -6.0));
        let spawned_a = app
            .world_mut()
            .spawn((
                SpawnedSandboxObject {
                    preset: crate::object_spawn_palette::SpawnPreset::Cube,
                },
                StationObject::new('A', Transform::IDENTITY),
            ))
            .id();
        let spawned_j = app
            .world_mut()
            .spawn((
                SpawnedSandboxObject {
                    preset: crate::object_spawn_palette::SpawnPreset::Ball,
                },
                StationObject::new('J', Transform::IDENTITY),
            ))
            .id();
        app.world_mut()
            .spawn((Player, Transform::from_xyz(-6.0, 1.0, -6.0)));
        app.world_mut()
            .entity_mut(a)
            .insert(Transform::from_xyz(2.0, 4.0, 3.0));
        app.world_mut()
            .entity_mut(a)
            .insert(Position(Vec3::new(2.0, 4.0, 3.0)));
        app.world_mut()
            .entity_mut(b)
            .insert(Transform::from_xyz(3.0, 4.0, 3.0));
        press_reset(&mut app, false);

        assert_eq!(
            app.world()
                .entity(a)
                .get::<Transform>()
                .unwrap()
                .translation,
            Vec3::new(-6.0, 1.0, -6.0)
        );
        assert_eq!(
            app.world()
                .entity(b)
                .get::<Transform>()
                .unwrap()
                .translation,
            Vec3::new(3.0, 4.0, 3.0)
        );
        assert!(app.world().get_entity(spawned_a).is_err());
        assert!(app.world().get_entity(spawned_j).is_ok());
    }

    #[test]
    fn shift_r_resets_all_stations_and_clears_spawned_objects_and_global_settings() {
        let mut app = reset_app();
        app.update();
        let a = station_object(&mut app, 'A', Vec3::new(-6.0, 1.0, -6.0));
        let b = station_object(&mut app, 'B', Vec3::new(0.0, 1.0, -6.0));
        for entity in [a, b] {
            app.world_mut()
                .entity_mut(entity)
                .insert(Transform::from_xyz(3.0, 4.0, 3.0));
        }
        let runtime_joint = app.world_mut().spawn(RuntimeCreatedJoint).id();
        let composition_joint = app.world_mut().spawn(CompositionJoint).id();
        let composed_body = app.world_mut().spawn(ComposedObject).id();
        let spawned = app
            .world_mut()
            .spawn((
                SpawnedSandboxObject {
                    preset: crate::object_spawn_palette::SpawnPreset::Cube,
                },
                StationObject::new('J', Transform::IDENTITY),
                RigidBody::Dynamic,
                layers_for(SandboxLayer::Objects),
            ))
            .id();
        app.world_mut().resource_mut::<Gravity>().0 = Vec3::ZERO;
        app.world_mut()
            .resource_mut::<Time<Physics>>()
            .set_relative_speed(2.0);
        app.world_mut().resource_mut::<SubstepCount>().0 = 2;

        press_reset(&mut app, true);

        assert_eq!(
            app.world()
                .entity(a)
                .get::<Transform>()
                .unwrap()
                .translation,
            Vec3::new(-6.0, 1.0, -6.0)
        );
        assert_eq!(
            app.world()
                .entity(b)
                .get::<Transform>()
                .unwrap()
                .translation,
            Vec3::new(0.0, 1.0, -6.0)
        );
        assert!(app.world().get_entity(spawned).is_err());
        assert!(app.world().get_entity(runtime_joint).is_err());
        assert!(app.world().get_entity(composition_joint).is_err());
        assert!(
            !app.world()
                .entity(composed_body)
                .contains::<ComposedObject>()
        );
        assert_eq!(
            app.world().resource::<Gravity>().0,
            Vec3::new(0.0, -9.81, 0.0)
        );
        assert_eq!(
            app.world().resource::<Time<Physics>>().relative_speed(),
            1.0
        );
        assert_eq!(app.world().resource::<SubstepCount>().0, 6);
    }
}
