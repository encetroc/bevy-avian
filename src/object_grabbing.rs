use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};

use crate::cursor_hover::{
    CursorRay, GrabDistanceSettings, HoverReachability, HoverState, is_within_grab_distance,
};
use crate::player::Player;

/// Spring parameters used while a dynamic body is being dragged.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SpringGrabSettings {
    /// Restoring force per metre of displacement.
    pub stiffness: f32,
    /// Force opposing the body's current velocity.
    pub damping: f32,
    /// Safety limit that keeps a fast cursor from producing an unstable force.
    pub max_force: f32,
}

impl Default for SpringGrabSettings {
    fn default() -> Self {
        Self {
            stiffness: 90.0,
            damping: 18.0,
            max_force: 1_200.0,
        }
    }
}

/// The world-space target and offset for the active grab, if any.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct GrabState {
    pub grab: Option<GrabTarget>,
}

/// Tunable settings for converting grab-target movement into a throw.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ThrowSettings {
    /// Multiplier applied to the recent grab-target velocity on throw.
    pub strength: f32,
}

impl Default for ThrowSettings {
    fn default() -> Self {
        Self { strength: 1.0 }
    }
}

/// The target tracked by the spring while a body is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrabTarget {
    pub entity: Entity,
    pub target: Vec3,
    pub distance: f32,
    /// The latest world-space velocity of the spring target.
    pub target_velocity: Vec3,
    cursor_offset: Vec3,
}

/// Marks a body whose force is currently owned by the grabbing system.
#[derive(Component, Debug)]
struct SpringGrabbed;

/// Owns mouse-button grabbing and the force-based spring that moves grabbed bodies.
pub struct ObjectGrabbingPlugin;

impl Plugin for ObjectGrabbingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorRay>()
            .init_resource::<HoverState>()
            .init_resource::<GrabDistanceSettings>()
            .init_resource::<SpringGrabSettings>()
            .init_resource::<ThrowSettings>()
            .init_resource::<GrabState>()
            .add_systems(
                Update,
                handle_grab_input.after(crate::cursor_hover::update_hover_reachability),
            )
            .add_systems(FixedUpdate, apply_spring_force);
    }
}

#[derive(SystemParam)]
struct GrabInput<'w, 's> {
    mouse: Res<'w, ButtonInput<MouseButton>>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    cursor_ray: Res<'w, CursorRay>,
    hover: Res<'w, HoverState>,
    settings: Res<'w, GrabDistanceSettings>,
    throw_settings: Res<'w, ThrowSettings>,
    time: Res<'w, Time>,
    players: Query<'w, 's, &'static Position, With<Player>>,
    bodies: Query<
        'w,
        's,
        (
            &'static Position,
            &'static RigidBody,
            Option<&'static LinearVelocity>,
        ),
    >,
}

fn handle_grab_input(input: GrabInput, mut state: ResMut<GrabState>, mut commands: Commands) {
    if input.mouse.just_pressed(MouseButton::Left) {
        release_grab(&mut state, &mut commands);

        let Some(hover) = input.hover.object.as_ref() else {
            return;
        };
        let Some(ray) = input.cursor_ray.ray else {
            return;
        };
        let Ok((position, body, _)) = input.bodies.get(hover.entity) else {
            return;
        };
        if *body != RigidBody::Dynamic || hover.reachability != HoverReachability::Reachable {
            return;
        }

        let target_on_ray = ray.origin + ray.direction * hover.distance;
        let cursor_offset = position.0 - target_on_ray;
        state.grab = Some(GrabTarget {
            entity: hover.entity,
            target: position.0,
            distance: hover.distance,
            target_velocity: Vec3::ZERO,
            cursor_offset,
        });
        commands
            .entity(hover.entity)
            .insert((SpringGrabbed, ConstantForce::default()));
    }

    let Some(active_grab) = state.grab else {
        return;
    };

    if let Some(player) = input.players.iter().next() {
        let Ok((position, _, _)) = input.bodies.get(active_grab.entity) else {
            release_grab(&mut state, &mut commands);
            return;
        };
        if !is_within_grab_distance(player.0, position.0, input.settings.max_distance) {
            release_grab(&mut state, &mut commands);
            return;
        }
    }

    if !input.mouse.pressed(MouseButton::Left) {
        release_grab(&mut state, &mut commands);
        return;
    }

    let delta_secs = input.time.delta_secs();
    let Some(grab) = state.grab.as_mut() else {
        return;
    };
    if let Some(ray) = input.cursor_ray.ray {
        let target_on_ray = ray.origin + ray.direction * grab.distance;
        let next_target = target_on_ray + grab.cursor_offset;
        grab.target_velocity = if delta_secs.is_finite() && delta_secs > 0.0 {
            (next_target - grab.target) / delta_secs
        } else {
            Vec3::ZERO
        };
        grab.target = next_target;
    } else {
        grab.target_velocity = Vec3::ZERO;
    }

    let throw_requested = input.mouse.just_pressed(MouseButton::Right)
        || input.keyboard.just_pressed(KeyCode::Space)
        // ButtonInput::press is a useful headless seam and represents a held
        // Space action even when no raw keyboard message was delivered.
        || input.keyboard.pressed(KeyCode::Space);
    if !throw_requested {
        return;
    }

    let target_velocity = grab.target_velocity;
    let current_velocity = input
        .bodies
        .get(active_grab.entity)
        .ok()
        .and_then(|(_, _, velocity)| velocity.map(|velocity| velocity.0))
        .unwrap_or(Vec3::ZERO);
    let throw_strength = input.throw_settings.strength;
    let throw_velocity = if throw_strength.is_finite() {
        target_velocity * throw_strength.max(0.0)
    } else {
        Vec3::ZERO
    };

    // Add the target's recent motion to the body's existing spring momentum,
    // then remove the spring so the body remains a normal dynamic collider.
    commands
        .entity(active_grab.entity)
        .insert(LinearVelocity(current_velocity + throw_velocity));
    release_grab(&mut state, &mut commands);
}

fn release_grab(state: &mut GrabState, commands: &mut Commands) {
    let Some(grab) = state.grab.take() else {
        return;
    };

    let mut entity = commands.entity(grab.entity);
    entity.remove::<SpringGrabbed>();
    entity.remove::<ConstantForce>();
}

fn apply_spring_force(
    state: Res<GrabState>,
    settings: Res<SpringGrabSettings>,
    mut grabbed_bodies: Query<
        (&Position, &LinearVelocity, &mut ConstantForce),
        With<SpringGrabbed>,
    >,
) {
    let Some(grab) = state.grab else {
        return;
    };
    let Ok((position, velocity, mut force)) = grabbed_bodies.get_mut(grab.entity) else {
        return;
    };

    let spring_force =
        (grab.target - position.0) * settings.stiffness - velocity.0 * settings.damping;
    force.0 = spring_force.clamp_length_max(settings.max_force);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        input::{ButtonState, InputPlugin, mouse::MouseButtonInput},
        mesh::MeshPlugin,
        time::TimeUpdateStrategy,
    };

    use super::*;
    use crate::cursor_hover::CursorHoverPlugin;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    fn grabbing_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CursorHoverPlugin,
            ObjectGrabbingPlugin,
        ))
        .add_systems(Startup, spawn_test_player)
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(GrabDistanceSettings { max_distance: 10.0 })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
    }

    fn spawn_test_player(mut commands: Commands) {
        commands.spawn((
            Player,
            Position(Vec3::ZERO),
            Transform::from_translation(Vec3::ZERO),
            Name::new("Grab Test Player"),
        ));
    }

    fn player_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<Player>>();
        players.iter(world).next().expect("grab test player")
    }

    fn spawn_body(app: &mut App, position: Vec3, mass: f32) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                Mass(mass),
                Position(position),
                Transform::from_translation(position),
                Name::new("Grab Test Body"),
            ))
            .id()
    }

    fn spawn_wall(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Collider::cuboid(0.5, 3.0, 3.0),
                Position(position),
                Transform::from_translation(position),
                Name::new("Grab Test Wall"),
            ))
            .id()
    }

    fn spawn_floor(app: &mut App) {
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 1.0, 20.0),
            Position(Vec3::new(0.0, -1.0, -5.0)),
            Transform::from_xyz(0.0, -1.0, -5.0),
            Name::new("Grab Test Floor"),
        ));
    }

    fn point_cursor_at(app: &mut App, target: Vec3) {
        let direction = Dir3::new(target.normalize()).expect("test ray direction");
        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::ZERO, direction);
        app.update();
    }

    fn send_mouse_button_for(app: &mut App, button: MouseButton, state: ButtonState) {
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(MouseButtonInput {
                button,
                state,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn send_mouse_button(app: &mut App, state: ButtonState) {
        send_mouse_button_for(app, MouseButton::Left, state);
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn set_max_grab_distance(app: &mut App, max_distance: f32) {
        app.world_mut()
            .resource_mut::<GrabDistanceSettings>()
            .max_distance = max_distance;
    }

    fn position(app: &App, entity: Entity) -> Vec3 {
        app.world()
            .entity(entity)
            .get::<Position>()
            .expect("body position")
            .0
    }

    #[test]
    fn left_click_grabs_only_dynamic_hovered_bodies() {
        let mut app = grabbing_app();
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);

        let grab = app.world().resource::<GrabState>().grab;
        assert_eq!(grab.map(|grab| grab.entity), Some(body));
        assert!(app.world().entity(body).contains::<SpringGrabbed>());
        assert!(app.world().entity(body).contains::<ConstantForce>());
        assert_eq!(
            app.world().entity(body).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
    }

    #[test]
    fn objects_at_the_radius_boundary_can_be_grabbed() {
        let mut app = grabbing_app();
        set_max_grab_distance(&mut app, 5.0);
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));

        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .reachability,
            HoverReachability::Reachable
        );
        send_mouse_button(&mut app, ButtonState::Pressed);

        assert_eq!(
            app.world()
                .resource::<GrabState>()
                .grab
                .map(|grab| grab.entity),
            Some(body)
        );
    }

    #[test]
    fn unreachable_hover_remains_visible_but_cannot_be_grabbed() {
        let mut app = grabbing_app();
        set_max_grab_distance(&mut app, 5.0);
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -6.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -6.0));

        let hover = app.world().resource::<HoverState>().object.clone().unwrap();
        assert_eq!(hover.entity, body);
        assert_eq!(hover.reachability, HoverReachability::Unreachable);
        send_mouse_button(&mut app, ButtonState::Pressed);

        assert!(app.world().resource::<GrabState>().grab.is_none());
        assert!(!app.world().entity(body).contains::<SpringGrabbed>());
    }

    #[test]
    fn moving_the_player_changes_grab_eligibility_and_releases_out_of_range_grabs() {
        let mut app = grabbing_app();
        set_max_grab_distance(&mut app, 5.0);
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);
        assert_eq!(
            app.world()
                .resource::<GrabState>()
                .grab
                .map(|grab| grab.entity),
            Some(body)
        );

        let player = player_entity(&mut app);
        app.world_mut()
            .entity_mut(player)
            .get_mut::<Position>()
            .unwrap()
            .0 = Vec3::new(0.0, 0.0, 0.01);
        app.update();

        assert_eq!(
            app.world()
                .resource::<HoverState>()
                .object
                .as_ref()
                .unwrap()
                .reachability,
            HoverReachability::Unreachable
        );
        assert!(app.world().resource::<GrabState>().grab.is_none());
        assert!(!app.world().entity(body).contains::<SpringGrabbed>());
    }

    #[test]
    fn target_motion_pulls_without_teleporting_and_heavy_bodies_lag_more() {
        let mut light_app = grabbing_app();
        let light = spawn_body(&mut light_app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut light_app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut light_app, ButtonState::Pressed);
        light_app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::ZERO,
            Dir3::new(Vec3::new(1.0, 0.0, -5.0).normalize()).unwrap(),
        );
        light_app.update();
        let light_after_one_frame = position(&light_app, light);
        run_steps(&mut light_app, 30);
        let light_after_motion = position(&light_app, light);

        let mut heavy_app = grabbing_app();
        let heavy = spawn_body(&mut heavy_app, Vec3::new(0.0, 0.0, -5.0), 10.0);
        point_cursor_at(&mut heavy_app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut heavy_app, ButtonState::Pressed);
        heavy_app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::ZERO,
            Dir3::new(Vec3::new(1.0, 0.0, -5.0).normalize()).unwrap(),
        );
        heavy_app.update();
        let heavy_after_one_frame = position(&heavy_app, heavy);
        run_steps(&mut heavy_app, 30);
        let heavy_after_motion = position(&heavy_app, heavy);

        assert!(
            light_after_one_frame.distance(Vec3::new(0.0, 0.0, -5.0)) < 0.1,
            "light body teleported: {light_after_one_frame:?}"
        );
        assert!(
            heavy_after_one_frame.distance(Vec3::new(0.0, 0.0, -5.0)) < 0.1,
            "heavy body teleported: {heavy_after_one_frame:?}"
        );
        assert!(
            light_after_motion.x > 0.2,
            "light body did not follow target"
        );
        assert!(
            heavy_after_motion.x < light_after_motion.x,
            "heavy body did not lag more"
        );
    }

    #[test]
    fn release_removes_spring_but_preserves_natural_velocity() {
        let mut app = grabbing_app();
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);
        app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::new(0.0, 0.0, 0.0),
            Dir3::new(Vec3::new(1.0, 0.0, -1.0).normalize()).unwrap(),
        );
        run_steps(&mut app, 30);
        let velocity_before_release = app.world().entity(body).get::<LinearVelocity>().unwrap().0;
        assert!(velocity_before_release.x > 0.1);

        send_mouse_button(&mut app, ButtonState::Released);
        assert!(app.world().resource::<GrabState>().grab.is_none());
        assert!(!app.world().entity(body).contains::<SpringGrabbed>());
        assert!(!app.world().entity(body).contains::<ConstantForce>());

        run_steps(&mut app, 1);
        let velocity_after_release = app.world().entity(body).get::<LinearVelocity>().unwrap().0;
        assert!(velocity_after_release.x > 0.0);
    }

    fn throw_body_with_target_motion(strength: f32, target: Vec3) -> (Vec3, Vec3) {
        let mut app = grabbing_app();
        app.world_mut().resource_mut::<ThrowSettings>().strength = strength;
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);

        app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::ZERO,
            Dir3::new(target.normalize()).expect("throw target direction"),
        );
        app.update();
        let target_velocity = app
            .world()
            .resource::<GrabState>()
            .grab
            .expect("active grab before throw")
            .target_velocity;

        send_mouse_button_for(&mut app, MouseButton::Right, ButtonState::Pressed);
        let velocity = app.world().entity(body).get::<LinearVelocity>().unwrap().0;
        assert!(app.world().resource::<GrabState>().grab.is_none());
        assert!(!app.world().entity(body).contains::<SpringGrabbed>());
        assert!(!app.world().entity(body).contains::<ConstantForce>());
        (target_velocity, velocity)
    }

    #[test]
    fn right_click_throw_releases_the_body_and_preserves_target_momentum() {
        let (target_velocity, throw_velocity) =
            throw_body_with_target_motion(1.0, Vec3::new(1.0, 0.0, -5.0));

        assert!(
            target_velocity.x > 1.0,
            "target did not move: {target_velocity:?}"
        );
        assert!(
            throw_velocity.x > 1.0,
            "throw did not preserve forward momentum: {throw_velocity:?} / {target_velocity:?}"
        );
    }

    #[test]
    fn configured_throw_strength_scales_low_and_high_target_speeds() {
        let (low_target_velocity, low_throw_velocity) =
            throw_body_with_target_motion(0.5, Vec3::new(0.5, 0.0, -5.0));
        let (high_target_velocity, high_throw_velocity) =
            throw_body_with_target_motion(2.0, Vec3::new(2.0, 0.0, -5.0));

        assert!(low_target_velocity.x > 0.0);
        assert!(high_target_velocity.x > low_target_velocity.x);
        assert!(high_throw_velocity.x > low_throw_velocity.x);
        assert!(
            high_throw_velocity.x - low_throw_velocity.x > 0.1,
            "throw strength and target speed had no measurable effect: {low_throw_velocity:?} / {high_throw_velocity:?}"
        );
    }

    #[test]
    fn thrown_body_keeps_colliding_with_a_wall_after_release() {
        let mut app = grabbing_app();
        app.world_mut().resource_mut::<ThrowSettings>().strength = 0.05;
        spawn_floor(&mut app);
        let body = spawn_body(&mut app, Vec3::new(-3.0, 0.0, -5.0), 2.0);
        let wall = spawn_wall(&mut app, Vec3::new(-1.0, 0.0, -5.0));
        point_cursor_at(&mut app, Vec3::new(-3.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);
        app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::ZERO,
            Dir3::new(Vec3::new(10.0, 0.0, -5.0).normalize()).unwrap(),
        );
        app.update();
        send_mouse_button_for(&mut app, MouseButton::Right, ButtonState::Pressed);

        assert!(app.world().resource::<GrabState>().grab.is_none());
        assert!(!app.world().entity(body).contains::<ConstantForce>());

        let mut collided = false;
        for _ in 0..180 {
            app.update();
            if app.world().resource::<ContactGraph>().contains(body, wall) {
                collided = true;
                break;
            }
        }

        assert!(
            collided,
            "thrown body did not collide after release: {:?}",
            position(&app, body)
        );
        assert_eq!(
            app.world().entity(body).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
    }

    #[test]
    fn space_throw_action_releases_the_body() {
        let mut app = grabbing_app();
        let body = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 1.0);
        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();

        assert!(app.world().resource::<GrabState>().grab.is_none());
        assert!(!app.world().entity(body).contains::<SpringGrabbed>());
    }

    #[test]
    fn grabbed_body_pushes_into_another_dynamic_body() {
        let mut app = grabbing_app();
        spawn_floor(&mut app);
        let grabbed = spawn_body(&mut app, Vec3::new(-3.0, 0.0, -5.0), 2.0);
        let obstacle = spawn_body(&mut app, Vec3::new(0.0, 0.0, -5.0), 8.0);
        point_cursor_at(&mut app, Vec3::new(-3.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);
        assert_eq!(
            app.world()
                .resource::<GrabState>()
                .grab
                .map(|grab| grab.entity),
            Some(grabbed)
        );
        app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::ZERO,
            Dir3::new(Vec3::new(3.0, 0.0, -5.0).normalize()).unwrap(),
        );

        let mut collided = false;
        for _ in 0..180 {
            app.update();
            if app
                .world()
                .resource::<ContactGraph>()
                .contains(grabbed, obstacle)
            {
                collided = true;
                break;
            }
        }

        assert!(
            collided,
            "grabbed body did not collide with another dynamic body: {:?} / {:?}",
            position(&app, grabbed),
            position(&app, obstacle)
        );
        assert_eq!(
            app.world().entity(grabbed).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
    }

    #[test]
    fn grabbed_body_keeps_colliding_with_a_static_wall() {
        let mut app = grabbing_app();
        let body = spawn_body(&mut app, Vec3::new(-3.0, 0.0, -5.0), 2.0);
        let wall = spawn_wall(&mut app, Vec3::new(0.0, 0.0, -5.0));
        point_cursor_at(&mut app, Vec3::new(-3.0, 0.0, -5.0));
        send_mouse_button(&mut app, ButtonState::Pressed);
        app.world_mut().resource_mut::<CursorRay>().set(
            Vec3::ZERO,
            Dir3::new(Vec3::new(1.0, 0.0, -5.0).normalize()).unwrap(),
        );

        run_steps(&mut app, 180);

        let body_position = position(&app, body);
        assert!(
            body_position.x < -0.45,
            "grabbed body passed through wall: {body_position:?}"
        );
        assert!(app.world().resource::<ContactGraph>().contains(body, wall));
        assert_eq!(
            app.world().entity(body).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
    }
}
