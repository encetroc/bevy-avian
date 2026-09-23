use avian3d::{
    dynamics::{
        joints::EntityConstraint,
        solver::{
            joint_graph::JointGraphPlugin,
            solver_body::{SolverBody, SolverBodyInertia},
            xpbd::*,
        },
    },
    math::*,
    prelude::*,
};
use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    prelude::*,
};

use crate::stations::StationObject;

const ANCHOR_POSITION: Vec3 = Vec3::new(0.0, 1.0, 18.0);
const BODY_POSITION: Vec3 = Vec3::new(2.0, 1.0, 18.0);
const REST_DISTANCE: f32 = 2.0;
const BODY_RADIUS: f32 = 0.3;

/// Keeps the demo body's center a fixed distance from the static anchor.
/// This deliberately small XPBD constraint demonstrates Avian's user-constraint API.
#[derive(Component)]
#[require(CenterDistanceConstraintSolverData)]
struct CenterDistanceConstraint {
    entity1: Entity,
    entity2: Entity,
    rest_distance: Scalar,
    compliance: Scalar,
}

#[derive(Component, Default)]
struct CenterDistanceConstraintSolverData {
    center_difference: Vector,
}

impl XpbdConstraintSolverData for CenterDistanceConstraintSolverData {}

impl CenterDistanceConstraint {
    const fn new(entity1: Entity, entity2: Entity, rest_distance: Scalar) -> Self {
        Self {
            entity1,
            entity2,
            rest_distance,
            compliance: 0.0,
        }
    }
}

impl EntityConstraint<2> for CenterDistanceConstraint {
    fn entities(&self) -> [Entity; 2] {
        [self.entity1, self.entity2]
    }
}

impl XpbdConstraint<2> for CenterDistanceConstraint {
    type SolverData = CenterDistanceConstraintSolverData;

    fn prepare(
        &mut self,
        bodies: [&RigidBodyQueryReadOnlyItem; 2],
        solver_data: &mut CenterDistanceConstraintSolverData,
    ) {
        let [body1, body2] = bodies;
        solver_data.center_difference = (body2.position.0 - body1.position.0)
            + (body2.rotation * body2.center_of_mass.0 - body1.rotation * body1.center_of_mass.0);
    }

    fn solve(
        &mut self,
        bodies: [&mut SolverBody; 2],
        inertias: [&SolverBodyInertia; 2],
        solver_data: &mut CenterDistanceConstraintSolverData,
        dt: Scalar,
    ) {
        let [body1, body2] = bodies;
        let [inertia1, inertia2] = inertias;
        let center_difference =
            body2.delta_position - body1.delta_position + solver_data.center_difference;
        let distance = center_difference.length();
        let error = distance - self.rest_distance;
        if distance <= 0.0 || error == 0.0 {
            return;
        }

        let normal = -center_difference / distance;
        let inverse_mass1 = inertia1.effective_inv_mass();
        let inverse_mass2 = inertia2.effective_inv_mass();
        let inverse_angular_inertia1 = inertia1.effective_inv_angular_inertia();
        let inverse_angular_inertia2 = inertia2.effective_inv_angular_inertia();
        let anchor1 = Vector::ZERO;
        let anchor2 = Vector::ZERO;
        let weight1 = PositionConstraint::compute_generalized_inverse_mass(
            self,
            inverse_mass1.max_element(),
            inverse_angular_inertia1,
            anchor1,
            normal,
        );
        let weight2 = PositionConstraint::compute_generalized_inverse_mass(
            self,
            inverse_mass2.max_element(),
            inverse_angular_inertia2,
            anchor2,
            normal,
        );
        let delta_lagrange =
            compute_lagrange_update(0.0, error, &[weight1, weight2], self.compliance, dt);
        self.apply_positional_impulse(
            body1,
            body2,
            inertia1,
            inertia2,
            delta_lagrange * normal,
            anchor1,
            anchor2,
        );
    }
}

impl PositionConstraint for CenterDistanceConstraint {}
impl AngularConstraint for CenterDistanceConstraint {}

impl MapEntities for CenterDistanceConstraint {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.entity1 = entity_mapper.get_mapped(self.entity1);
        self.entity2 = entity_mapper.get_mapped(self.entity2);
    }
}

#[derive(Component)]
struct CustomConstraintDemo;

#[derive(Component)]
struct CustomConstraintBody;

/// Runtime enable state and the entities used to reinstall the optional constraint.
#[derive(Resource, Default)]
struct CustomConstraintState {
    constraint_entity: Option<Entity>,
    anchor: Option<Entity>,
    body: Option<Entity>,
    enabled: bool,
}

#[derive(Message, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CustomConstraintAction {
    pub toggle: bool,
}

/// Adds a self-contained custom XPBD distance-constraint demo in the free-sandbox area.
pub struct CustomConstraintStationPlugin;

impl Plugin for CustomConstraintStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(JointGraphPlugin::<CenterDistanceConstraint>::default())
            .init_resource::<CustomConstraintState>()
            .add_message::<CustomConstraintAction>()
            .add_systems(Startup, spawn_custom_constraint_demo)
            .add_systems(
                Update,
                (queue_custom_constraint_action, toggle_custom_constraint).chain(),
            )
            .add_systems(
                PhysicsSchedule,
                prepare_xpbd_joint::<CenterDistanceConstraint>
                    .in_set(SolverSystems::PreSubstep)
                    .ambiguous_with_all(),
            );

        app.get_schedule_mut(SubstepSchedule)
            .expect("PhysicsPlugins installs the Avian substep schedule")
            .add_systems(
                solve_xpbd_joint::<CenterDistanceConstraint>
                    .in_set(XpbdSolverSystems::SolveUserConstraints),
            );
    }
}

fn spawn_custom_constraint_demo(
    mut commands: Commands,
    mut state: ResMut<CustomConstraintState>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let anchor = commands
        .spawn((
            RigidBody::Static,
            Collider::sphere(BODY_RADIUS),
            Transform::from_translation(ANCHOR_POSITION),
            Name::new("Custom Constraint Anchor"),
        ))
        .id();
    let body_transform = Transform::from_translation(BODY_POSITION);
    let body = commands
        .spawn((
            CustomConstraintBody,
            StationObject::new('J', body_transform)
                .with_velocities(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO),
            RigidBody::Dynamic,
            Collider::sphere(BODY_RADIUS),
            body_transform,
            LinearVelocity(Vec3::new(0.0, 0.0, 3.0)),
            Name::new("Custom Constraint Demo Body"),
        ))
        .id();
    let constraint_entity = commands
        .spawn((
            CenterDistanceConstraint::new(anchor, body, REST_DISTANCE),
            CustomConstraintDemo,
            Name::new("Custom Center-Distance Constraint"),
        ))
        .id();

    if let (Some(meshes), Some(materials)) = (meshes.as_mut(), materials.as_mut()) {
        let mesh = meshes.add(Sphere::new(BODY_RADIUS));
        let anchor_material = materials.add(Color::srgb(0.25, 0.85, 0.55));
        let body_material = materials.add(Color::srgb(0.95, 0.42, 0.18));
        commands
            .entity(anchor)
            .insert((Mesh3d(mesh.clone()), MeshMaterial3d(anchor_material)));
        commands
            .entity(body)
            .insert((Mesh3d(mesh), MeshMaterial3d(body_material)));
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(420.0),
                top: px(16.0),
                width: px(360.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.05, 0.08, 0.94)),
            Name::new("Custom Constraint Demo Help"),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("J — CUSTOM CONSTRAINT"),
                TextFont::from_font_size(16.0),
            ));
            panel.spawn((
                Text::new("Purpose: keep the orange body's center 2 m from the green anchor. Toggle the constraint to compare constrained orbiting with unconstrained travel."),
                TextFont::from_font_size(12.0),
            ));
            panel
                .spawn((
                    Button,
                    CustomConstraintToggleButton,
                    Node {
                        width: px(176.0),
                        height: px(26.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.18, 0.28)),
                    Name::new("Toggle custom constraint"),
                ))
                .with_child((
                    Text::new("Toggle constraint"),
                    TextFont::from_font_size(12.0),
                ));
        });

    state.constraint_entity = Some(constraint_entity);
    state.anchor = Some(anchor);
    state.body = Some(body);
    state.enabled = true;
}

#[derive(Component)]
struct CustomConstraintToggleButton;

fn queue_custom_constraint_action(
    buttons: Query<&Interaction, (Changed<Interaction>, With<CustomConstraintToggleButton>)>,
    mut actions: MessageWriter<CustomConstraintAction>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        actions.write(CustomConstraintAction { toggle: true });
    }
}

fn toggle_custom_constraint(
    mut actions: MessageReader<CustomConstraintAction>,
    mut state: ResMut<CustomConstraintState>,
    mut commands: Commands,
) {
    let toggle_count = actions.read().filter(|action| action.toggle).count();
    if toggle_count % 2 == 0 {
        return;
    }
    let (Some(constraint_entity), Some(anchor), Some(body)) =
        (state.constraint_entity, state.anchor, state.body)
    else {
        return;
    };

    state.enabled = !state.enabled;
    if state.enabled {
        commands
            .entity(constraint_entity)
            .insert(CenterDistanceConstraint::new(anchor, body, REST_DISTANCE));
    } else {
        commands
            .entity(constraint_entity)
            .remove::<CenterDistanceConstraint>();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CustomConstraintStationPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn body_and_anchor(app: &mut App) -> (Entity, Entity) {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<Entity, With<CustomConstraintBody>>();
        let body = bodies.iter(world).next().expect("custom constraint body");
        let mut anchors =
            world.query_filtered::<Entity, (With<RigidBody>, Without<CustomConstraintBody>)>();
        let anchor = anchors
            .iter(world)
            .next()
            .expect("custom constraint anchor");
        (body, anchor)
    }

    fn distance(app: &App, body: Entity, anchor: Entity) -> f32 {
        let body_position = app.world().entity(body).get::<Position>().unwrap().0;
        let anchor_position = app.world().entity(anchor).get::<Position>().unwrap().0;
        body_position.distance(anchor_position)
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn toggle(app: &mut App) {
        app.world_mut()
            .write_message(CustomConstraintAction { toggle: true });
        app.update();
    }

    #[test]
    fn enabled_constraint_keeps_body_center_at_configured_distance() {
        let mut app = app();
        app.update();
        let (body, anchor) = body_and_anchor(&mut app);

        run_steps(&mut app, 180);

        let measured = distance(&app, body, anchor);
        assert!(
            (measured - REST_DISTANCE).abs() < 0.08,
            "custom constraint drifted from {REST_DISTANCE} m: {measured}"
        );
    }

    #[test]
    fn disabling_releases_body_and_reenabling_restores_the_distance_constraint() {
        for _ in 0..3 {
            let mut app = app();
            app.update();
            let (body, anchor) = body_and_anchor(&mut app);
            run_steps(&mut app, 30);
            assert!(app.world().resource::<CustomConstraintState>().enabled);

            toggle(&mut app);
            assert!(!app.world().resource::<CustomConstraintState>().enabled);
            run_steps(&mut app, 30);
            let unconstrained_distance = distance(&app, body, anchor);
            assert!(
                unconstrained_distance > REST_DISTANCE + 0.25,
                "disabled body remained constrained at {unconstrained_distance} m"
            );

            toggle(&mut app);
            assert!(app.world().resource::<CustomConstraintState>().enabled);
            run_steps(&mut app, 180);
            let constrained_distance = distance(&app, body, anchor);
            assert!(
                (constrained_distance - REST_DISTANCE).abs() < 0.08,
                "reenabled constraint did not restore its purpose: {constrained_distance} m"
            );
        }
    }
}
