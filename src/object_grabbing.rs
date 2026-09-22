use avian3d::prelude::*;
use bevy::prelude::*;

use crate::cursor_hover::{CursorRay, HoverState};

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

/// The target tracked by the spring while a body is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrabTarget {
    pub entity: Entity,
    pub target: Vec3,
    pub distance: f32,
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
            .init_resource::<SpringGrabSettings>()
            .init_resource::<GrabState>()
            .add_systems(
                Update,
                handle_grab_input.after(crate::cursor_hover::update_hover_state),
            )
            .add_systems(FixedUpdate, apply_spring_force);
    }
}

fn handle_grab_input(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_ray: Res<CursorRay>,
    hover: Res<HoverState>,
    bodies: Query<(&Position, &RigidBody)>,
    mut state: ResMut<GrabState>,
    mut commands: Commands,
) {
    if mouse.just_pressed(MouseButton::Left) {
        release_grab(&mut state, &mut commands);

        let Some(hover) = hover.object.as_ref() else {
            return;
        };
        let Some(ray) = cursor_ray.ray else {
            return;
        };
        let Ok((position, body)) = bodies.get(hover.entity) else {
            return;
        };
        if *body != RigidBody::Dynamic {
            return;
        }

        let target_on_ray = ray.origin + ray.direction * hover.distance;
        let cursor_offset = position.0 - target_on_ray;
        state.grab = Some(GrabTarget {
            entity: hover.entity,
            target: position.0,
            distance: hover.distance,
            cursor_offset,
        });
        commands
            .entity(hover.entity)
            .insert((SpringGrabbed, ConstantForce::default()));
    }

    let Some(grab) = state.grab.as_mut() else {
        return;
    };

    if !mouse.pressed(MouseButton::Left) {
        release_grab(&mut state, &mut commands);
        return;
    }

    if let Some(ray) = cursor_ray.ray {
        let target_on_ray = ray.origin + ray.direction * grab.distance;
        grab.target = target_on_ray + grab.cursor_offset;
    }
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
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.finish();
        app
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

    fn send_mouse_button(app: &mut App, state: ButtonState) {
        app.world_mut()
            .resource_mut::<Messages<MouseButtonInput>>()
            .write(MouseButtonInput {
                button: MouseButton::Left,
                state,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
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
