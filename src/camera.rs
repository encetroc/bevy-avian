use bevy::prelude::*;

/// The fixed world-space offset used by the isometric follow camera.
///
/// The camera keeps this height and viewing direction while its horizontal
/// position follows the target. Keeping the vertical component independent of
/// the target prevents physics bodies from pulling the camera up or down.
pub const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 23.0, 25.0);

/// Exponential decay rate for camera positional following.
pub const CAMERA_DECAY_RATE: f32 = 6.0;

/// Marks the one gameplay entity the sandbox camera should follow.
#[derive(Component)]
pub struct CameraFollowTarget;

/// Marks the camera owned by [`CameraFollowPlugin`].
#[derive(Component)]
pub struct FixedFollowCamera;

/// Configuration for the fixed-orientation follow camera.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CameraFollowSettings {
    /// Horizontal offset from the target and the fixed camera height.
    pub offset: Vec3,
    /// Exponential decay rate used for smooth positional movement.
    pub decay_rate: f32,
}

impl Default for CameraFollowSettings {
    fn default() -> Self {
        Self {
            offset: CAMERA_OFFSET,
            decay_rate: CAMERA_DECAY_RATE,
        }
    }
}

/// Owns the sandbox's fixed-orientation, smooth-follow camera.
pub struct CameraFollowPlugin;

impl Plugin for CameraFollowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraFollowSettings>()
            .add_systems(Startup, spawn_follow_camera)
            .add_systems(Update, follow_camera);
    }
}

fn spawn_follow_camera(mut commands: Commands, settings: Res<CameraFollowSettings>) {
    commands.spawn((
        Camera3d::default(),
        FixedFollowCamera,
        Transform::from_translation(settings.offset).looking_at(Vec3::ZERO, Dir3::Y),
        Name::new("Sandbox Camera"),
    ));
}

fn follow_camera(
    mut camera: Single<&mut Transform, With<FixedFollowCamera>>,
    target: Option<Single<&Transform, (With<CameraFollowTarget>, Without<FixedFollowCamera>)>>,
    settings: Res<CameraFollowSettings>,
    time: Res<Time>,
) {
    let Some(target) = target else {
        return;
    };

    let target = target.translation;
    let desired_position = Vec3::new(
        target.x + settings.offset.x,
        settings.offset.y,
        target.z + settings.offset.z,
    );

    // Only translation changes here. The rotation established at spawn is
    // deliberately preserved so normal player movement cannot rotate the view.
    camera
        .translation
        .smooth_nudge(&desired_position, settings.decay_rate, time.delta_secs());
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    const FRAME_TIME: f32 = 1.0 / 60.0;

    fn camera_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, CameraFollowPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                FRAME_TIME,
            )));
        app.finish();
        app
    }

    fn camera_transform(app: &mut App) -> Transform {
        let world = app.world_mut();
        let mut cameras = world.query_filtered::<&Transform, With<FixedFollowCamera>>();
        *cameras.iter(world).next().expect("fixed follow camera")
    }

    #[test]
    fn camera_starts_in_a_tilted_isometric_pose() {
        let mut app = camera_app();
        app.update();

        let camera = camera_transform(&mut app);
        assert_eq!(camera.translation, CAMERA_OFFSET);
        assert!(camera.translation.y > 0.0);
        assert!(camera.translation.z > 0.0);

        let expected_forward = (Vec3::ZERO - CAMERA_OFFSET).normalize();
        let actual_forward = camera.rotation * -Vec3::Z;
        assert!(actual_forward.distance(expected_forward) < 1e-5);
    }

    #[test]
    fn camera_smoothly_follows_horizontal_target_movement() {
        let mut app = camera_app();
        let target = app
            .world_mut()
            .spawn((CameraFollowTarget, Transform::default()))
            .id();
        app.update();

        let initial = camera_transform(&mut app);
        app.world_mut()
            .entity_mut(target)
            .get_mut::<Transform>()
            .expect("target transform")
            .translation = Vec3::new(8.0, 9.0, -6.0);

        app.update();
        let in_flight = camera_transform(&mut app);
        assert!(in_flight.translation.x > initial.translation.x);
        assert!(in_flight.translation.x < 8.0 + CAMERA_OFFSET.x);
        assert!(in_flight.translation.z < initial.translation.z);
        assert!(in_flight.translation.z > -6.0 + CAMERA_OFFSET.z);
        assert_eq!(in_flight.translation.y, CAMERA_OFFSET.y);

        for _ in 0..240 {
            app.update();
        }
        let settled = camera_transform(&mut app);
        let expected = Vec3::new(8.0, CAMERA_OFFSET.y, -6.0 + CAMERA_OFFSET.z);
        assert!(settled.translation.distance(expected) < 0.01);
    }

    #[test]
    fn target_vertical_motion_does_not_change_camera_height() {
        let mut app = camera_app();
        let target = app
            .world_mut()
            .spawn((CameraFollowTarget, Transform::from_xyz(2.0, 50.0, -3.0)))
            .id();
        app.update();

        for _ in 0..10 {
            app.update();
        }
        let camera = camera_transform(&mut app);
        assert_eq!(camera.translation.y, CAMERA_OFFSET.y);
        assert!(camera.translation.x > 0.0);
        assert!(camera.translation.z < CAMERA_OFFSET.z);

        app.world_mut()
            .entity_mut(target)
            .get_mut::<Transform>()
            .expect("target transform")
            .translation
            .y = -50.0;
        app.update();

        assert_eq!(camera_transform(&mut app).translation.y, CAMERA_OFFSET.y);
    }

    #[test]
    fn camera_orientation_remains_fixed_while_target_moves() {
        let mut app = camera_app();
        let target = app
            .world_mut()
            .spawn((CameraFollowTarget, Transform::default()))
            .id();
        app.update();
        let initial_rotation = camera_transform(&mut app).rotation;

        for position in [
            Vec3::new(-7.0, 0.0, -8.0),
            Vec3::new(6.0, 0.0, -1.0),
            Vec3::new(4.0, 0.0, 7.0),
        ] {
            app.world_mut()
                .entity_mut(target)
                .get_mut::<Transform>()
                .expect("target transform")
                .translation = position;
            app.update();
            assert_eq!(camera_transform(&mut app).rotation, initial_rotation);
        }
    }
}
