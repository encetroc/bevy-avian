use avian3d::{parry::shape::TypedShape, prelude::*};
use bevy::prelude::*;

use crate::cursor_hover::{CursorRay, HoverState, update_hover_reachability};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// The physics body currently selected by the development inspector.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionState {
    /// The selected body or joint entity, or `None` when the last click hit empty space.
    pub entity: Option<Entity>,
}

/// A readable snapshot of the selected joint's configuration and stress state.
#[derive(Clone, Debug, PartialEq)]
pub struct JointInspectorSnapshot {
    pub entity: Entity,
    pub name: String,
    pub joint_type: JointType,
    pub body1: Entity,
    pub body2: Entity,
    pub body1_name: String,
    pub body2_name: String,
    pub anchor1: Option<Vec3>,
    pub anchor2: Option<Vec3>,
    pub compliance: JointComplianceInfo,
    pub damping: JointDampingInfo,
    pub stress: Option<JointStressInfo>,
    pub enabled: bool,
}

/// The Avian joint type represented by an inspected joint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JointType {
    Fixed,
    Distance,
    Revolute,
    Prismatic,
    Spherical,
}

/// Compliance values exposed by a joint inspector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JointComplianceInfo {
    pub point: Option<f32>,
    pub alignment: Option<f32>,
    pub angle: Option<f32>,
    pub limit: Option<f32>,
    pub swing: Option<f32>,
    pub twist: Option<f32>,
}

/// Damping values exposed by a joint inspector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JointDampingInfo {
    pub linear: f32,
    pub angular: f32,
    pub present: bool,
}

/// Force, torque, and motor stress reported by Avian for an inspected joint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JointStressInfo {
    pub force: Vec3,
    pub torque: Vec3,
    pub motor_force: f32,
}

struct JointDetails {
    joint_type: JointType,
    bodies: [Entity; 2],
    anchors: [Option<Vec3>; 2],
    compliance: JointComplianceInfo,
}

/// A live contact snapshot for one contact manifold involving the selected body.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactInspectorSnapshot {
    pub other_entity: Entity,
    pub other_name: String,
    pub position: Vec3,
    pub normal: Vec3,
    pub relative_speed: f32,
    pub impulse: f32,
}

/// A readable snapshot of the selected body's physics and current simulation state.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorSnapshot {
    pub entity: Entity,
    pub name: String,
    pub body: RigidBody,
    pub collider: ColliderInfo,
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub mass: Option<f32>,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub locked_axes: u8,
    pub dominance: i8,
    pub friction: Option<FrictionInfo>,
    pub restitution: Option<f32>,
    pub sleeping: bool,
    pub sleeping_disabled: bool,
    pub contacts: Vec<ContactInspectorSnapshot>,
}

/// Collider details shown by the inspector shell.
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderInfo {
    pub shape: String,
    pub scale: Vec3,
}

/// Physics material coefficients shown by the inspector shell.
#[derive(Clone, Debug, PartialEq)]
pub struct FrictionInfo {
    pub dynamic: f32,
    pub static_coefficient: f32,
}

/// The inspector's current contents. `None` means that its panel is cleared.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct InspectorState {
    pub object: Option<InspectorSnapshot>,
    pub joint: Option<JointInspectorSnapshot>,
}

/// Owns click selection and the development-only selected-object inspector.
pub struct ObjectInspectorPlugin;

impl Plugin for ObjectInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionState>()
            .init_resource::<InspectorState>()
            .add_systems(Startup, spawn_inspector_ui)
            .add_systems(
                Update,
                (
                    select_hovered_object.after(update_hover_reachability),
                    select_hovered_joint,
                    apply_inspector_edits,
                    apply_joint_inspector_edits,
                    refresh_inspector,
                    update_inspector_ui,
                )
                    .chain(),
            );
    }
}

/// Selects the body under the cursor on a left click. Empty-space clicks clear
/// the selection, while hovering alone never changes the inspected object.
fn select_hovered_object(
    mouse: Res<ButtonInput<MouseButton>>,
    hover: Res<HoverState>,
    controls: Query<&Interaction, With<InspectorControl>>,
    mut selection: ResMut<SelectionState>,
) {
    if !mouse.just_pressed(MouseButton::Left)
        || controls
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    selection.set_if_neq(SelectionState {
        entity: hover.object.as_ref().map(|object| object.entity),
    });
}

/// Selects the joint whose visible anchor line is under the cursor. Joints do
/// not have colliders, so selection uses the same cursor ray as body hover and
/// a small world-space distance threshold around each joint's anchors.
fn select_hovered_joint(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_ray: Res<CursorRay>,
    controls: Query<&Interaction, With<InspectorControl>>,
    mut selection: ResMut<SelectionState>,
    joints: JointSelectionQuery<'_, '_>,
    bodies: Query<&Transform>,
) {
    if !mouse.just_pressed(MouseButton::Left)
        || controls
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    let Some(ray) = cursor_ray.ray else {
        return;
    };
    let Some((joint, _)) = closest_joint_to_ray(ray, &joints, &bodies) else {
        return;
    };
    selection.set_if_neq(SelectionState {
        entity: Some(joint),
    });
}

const JOINT_PICK_DISTANCE: f32 = 0.3;

type JointSelectionQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static FixedJoint>,
        Option<&'static DistanceJoint>,
        Option<&'static RevoluteJoint>,
        Option<&'static PrismaticJoint>,
        Option<&'static SphericalJoint>,
    ),
>;

type JointInspectorQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Name>,
        Option<&'static FixedJoint>,
        Option<&'static DistanceJoint>,
        Option<&'static RevoluteJoint>,
        Option<&'static PrismaticJoint>,
        Option<&'static SphericalJoint>,
        Option<&'static JointDamping>,
        Option<&'static JointForces>,
        Has<JointDisabled>,
    ),
>;

fn closest_joint_to_ray(
    ray: Ray3d,
    joints: &JointSelectionQuery<'_, '_>,
    body_transforms: &Query<&Transform>,
) -> Option<(Entity, f32)> {
    let mut closest = None;
    for (entity, fixed, distance, revolute, prismatic, spherical) in joints.iter() {
        let Some(details) = joint_details(fixed, distance, revolute, prismatic, spherical) else {
            continue;
        };
        let [body1, body2] = details.bodies;
        let [Some(local_anchor1), Some(local_anchor2)] = details.anchors else {
            continue;
        };
        let Ok([transform1, transform2]) = body_transforms.get_many([body1, body2]) else {
            continue;
        };
        let anchor1 = transform1.translation + transform1.rotation * local_anchor1;
        let anchor2 = transform2.translation + transform2.rotation * local_anchor2;
        let Some((ray_distance, segment_distance)) =
            ray_segment_distance(ray.origin, ray.direction, anchor1, anchor2)
        else {
            continue;
        };
        if segment_distance > JOINT_PICK_DISTANCE
            || closest.is_some_and(|(_, current)| ray_distance >= current)
        {
            continue;
        }
        closest = Some((entity, ray_distance));
    }
    closest
}

fn ray_segment_distance(
    origin: Vec3,
    direction: Dir3,
    start: Vec3,
    end: Vec3,
) -> Option<(f32, f32)> {
    let direction = direction.as_vec3();
    let segment = end - start;
    let segment_length_squared = segment.length_squared();
    let (ray_distance, segment_factor) = if segment_length_squared <= f32::EPSILON {
        ((start - origin).dot(direction), 0.0)
    } else {
        let ray_to_start = start - origin;
        let direction_segment = direction.dot(segment);
        let ray_segment = direction.dot(ray_to_start);
        let segment_start = segment.dot(ray_to_start);
        let denominator = segment_length_squared - direction_segment * direction_segment;
        if denominator.abs() <= f32::EPSILON {
            (ray_segment, 0.0)
        } else {
            let ray_distance = (segment_length_squared * ray_segment
                - direction_segment * segment_start)
                / denominator;
            let segment_factor =
                (segment_start - ray_distance * direction_segment) / segment_length_squared;
            (ray_distance, segment_factor)
        }
    };
    let segment_factor = segment_factor.clamp(0.0, 1.0);
    let ray_distance = ray_distance.max(0.0);
    let ray_point = origin + direction * ray_distance;
    let segment_point = start + segment * segment_factor;
    Some((ray_distance, ray_point.distance(segment_point)))
}

/// Applies one-shot button edits directly to the selected body's Avian
/// components. Commands are used only when an optional component is not yet
/// present; existing components are changed immediately so the next physics
/// step observes the new value.
#[allow(clippy::type_complexity)]
fn apply_inspector_edits(
    selection: Res<SelectionState>,
    mut commands: Commands,
    controls: Query<(&Interaction, &InspectorControl), Changed<Interaction>>,
    mut properties: Query<(
        Option<&mut Mass>,
        Option<&ComputedMass>,
        Option<&mut GravityScale>,
        Option<&mut LinearDamping>,
        Option<&mut AngularDamping>,
        Option<&mut LockedAxes>,
        Option<&mut Dominance>,
        Option<&mut LinearVelocity>,
        Option<&mut AngularVelocity>,
    )>,
    bodies: Query<&RigidBody>,
) {
    let Some(entity) = selection.entity else {
        return;
    };
    let Ok((
        mut mass,
        computed_mass,
        mut gravity_scale,
        mut linear_damping,
        mut angular_damping,
        mut locked_axes,
        mut dominance,
        mut linear_velocity,
        mut angular_velocity,
    )) = properties.get_mut(entity)
    else {
        return;
    };

    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *control {
            InspectorControl::Scalar { property, step } => {
                let delta = scalar_step(property) * step_multiplier(step);
                match property {
                    ScalarProperty::Mass => {
                        let current = mass
                            .as_ref()
                            .map(|value| value.0)
                            .or_else(|| computed_mass.map(|value| value.value()))
                            .unwrap_or(1.0);
                        let value = (current + delta).max(0.01);
                        if let Some(mass) = mass.as_deref_mut() {
                            mass.0 = value;
                        } else {
                            commands.entity(entity).insert(Mass(value));
                        }
                    }
                    ScalarProperty::GravityScale => {
                        let current = gravity_scale.as_ref().map(|value| value.0).unwrap_or(1.0);
                        let value = current + delta;
                        if let Some(scale) = gravity_scale.as_deref_mut() {
                            scale.0 = value;
                        } else {
                            commands.entity(entity).insert(GravityScale(value));
                        }
                    }
                    ScalarProperty::LinearDamping => {
                        let current = linear_damping.as_ref().map(|value| value.0).unwrap_or(0.0);
                        let value = (current + delta).max(0.0);
                        if let Some(damping) = linear_damping.as_deref_mut() {
                            damping.0 = value;
                        } else {
                            commands.entity(entity).insert(LinearDamping(value));
                        }
                    }
                    ScalarProperty::AngularDamping => {
                        let current = angular_damping.as_ref().map(|value| value.0).unwrap_or(0.0);
                        let value = (current + delta).max(0.0);
                        if let Some(damping) = angular_damping.as_deref_mut() {
                            damping.0 = value;
                        } else {
                            commands.entity(entity).insert(AngularDamping(value));
                        }
                    }
                    ScalarProperty::Dominance => {
                        let current = dominance.as_ref().map(|value| value.0).unwrap_or(0);
                        let value = (current + delta as i8).clamp(-127, 127);
                        if let Some(dominance) = dominance.as_deref_mut() {
                            dominance.0 = value;
                        } else {
                            commands.entity(entity).insert(Dominance(value));
                        }
                    }
                }
            }
            InspectorControl::ToggleAxisLock { kind, axis } => {
                let current = locked_axes.as_deref().copied().unwrap_or_default();
                let updated = toggle_axis_lock(current, kind, axis);
                if let Some(locked_axes) = locked_axes.as_deref_mut() {
                    *locked_axes = updated;
                } else {
                    commands.entity(entity).insert(updated);
                }
            }
            InspectorControl::Velocity { kind, axis, step } => {
                let delta = 1.0 * step_multiplier(step);
                let velocity = match kind {
                    VelocityKind::Linear => linear_velocity
                        .as_deref_mut()
                        .map(|velocity| &mut velocity.0),
                    VelocityKind::Angular => angular_velocity
                        .as_deref_mut()
                        .map(|velocity| &mut velocity.0),
                };
                if let Some(velocity) = velocity {
                    change_axis(velocity, axis, delta);
                } else if kind == VelocityKind::Linear {
                    let mut value = Vec3::ZERO;
                    change_axis(&mut value, axis, delta);
                    commands.entity(entity).insert(LinearVelocity(value));
                } else {
                    let mut value = Vec3::ZERO;
                    change_axis(&mut value, axis, delta);
                    commands.entity(entity).insert(AngularVelocity(value));
                }
            }
            InspectorControl::CycleBody => {
                let Some(body) = bodies.get(entity).ok() else {
                    continue;
                };
                let next_body = match body {
                    RigidBody::Dynamic => RigidBody::Kinematic,
                    RigidBody::Kinematic => RigidBody::Static,
                    RigidBody::Static => RigidBody::Dynamic,
                };
                commands.entity(entity).insert(next_body);
            }
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_joint_inspector_edits(
    mut selection: ResMut<SelectionState>,
    mut commands: Commands,
    controls: Query<(&Interaction, &InspectorControl), Changed<Interaction>>,
    mut joints: Query<(
        Option<&mut FixedJoint>,
        Option<&mut DistanceJoint>,
        Option<&mut RevoluteJoint>,
        Option<&mut PrismaticJoint>,
        Option<&mut SphericalJoint>,
        Option<&mut JointDamping>,
        Option<&JointDisabled>,
    )>,
) {
    let Some(entity) = selection.entity else {
        return;
    };
    let Ok((
        mut fixed,
        mut distance,
        mut revolute,
        mut prismatic,
        mut spherical,
        mut damping,
        disabled,
    )) = joints.get_mut(entity)
    else {
        return;
    };
    if fixed.is_none()
        && distance.is_none()
        && revolute.is_none()
        && prismatic.is_none()
        && spherical.is_none()
    {
        return;
    }

    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *control {
            InspectorControl::ToggleJointEnabled => {
                if disabled.is_some() {
                    commands.entity(entity).remove::<JointDisabled>();
                } else {
                    commands.entity(entity).insert(JointDisabled);
                }
            }
            InspectorControl::DeleteJoint => {
                commands.entity(entity).despawn();
                selection.entity = None;
                return;
            }
            InspectorControl::JointAnchor { body, axis, step } => {
                let delta = 0.1 * step_multiplier(step);
                if let Some(joint) = fixed.as_deref_mut() {
                    let anchor = if body == JointBody::First {
                        &mut joint.frame1.anchor
                    } else {
                        &mut joint.frame2.anchor
                    };
                    update_joint_anchor(anchor, axis, delta);
                }
                if let Some(joint) = distance.as_deref_mut() {
                    let anchor = if body == JointBody::First {
                        &mut joint.anchor1
                    } else {
                        &mut joint.anchor2
                    };
                    update_joint_anchor(anchor, axis, delta);
                }
                if let Some(joint) = revolute.as_deref_mut() {
                    let anchor = if body == JointBody::First {
                        &mut joint.frame1.anchor
                    } else {
                        &mut joint.frame2.anchor
                    };
                    update_joint_anchor(anchor, axis, delta);
                }
                if let Some(joint) = prismatic.as_deref_mut() {
                    let anchor = if body == JointBody::First {
                        &mut joint.frame1.anchor
                    } else {
                        &mut joint.frame2.anchor
                    };
                    update_joint_anchor(anchor, axis, delta);
                }
                if let Some(joint) = spherical.as_deref_mut() {
                    let anchor = if body == JointBody::First {
                        &mut joint.frame1.anchor
                    } else {
                        &mut joint.frame2.anchor
                    };
                    update_joint_anchor(anchor, axis, delta);
                }
            }
            InspectorControl::JointCompliance { property, step } => {
                let delta = 0.001 * step_multiplier(step);
                if let Some(joint) = fixed.as_deref_mut() {
                    match property {
                        JointComplianceProperty::Point => {
                            joint.point_compliance = (joint.point_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Angle => {
                            joint.angle_compliance = (joint.angle_compliance + delta).max(0.0)
                        }
                        _ => {}
                    }
                }
                if let Some(joint) = distance.as_deref_mut()
                    && property == JointComplianceProperty::Point
                {
                    joint.compliance = (joint.compliance + delta).max(0.0);
                }
                if let Some(joint) = revolute.as_deref_mut() {
                    match property {
                        JointComplianceProperty::Point => {
                            joint.point_compliance = (joint.point_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Alignment => {
                            joint.align_compliance = (joint.align_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Limit => {
                            joint.limit_compliance = (joint.limit_compliance + delta).max(0.0)
                        }
                        _ => {}
                    }
                }
                if let Some(joint) = prismatic.as_deref_mut() {
                    match property {
                        JointComplianceProperty::Alignment => {
                            joint.align_compliance = (joint.align_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Angle => {
                            joint.angle_compliance = (joint.angle_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Limit => {
                            joint.limit_compliance = (joint.limit_compliance + delta).max(0.0)
                        }
                        _ => {}
                    }
                }
                if let Some(joint) = spherical.as_deref_mut() {
                    match property {
                        JointComplianceProperty::Point => {
                            joint.point_compliance = (joint.point_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Swing => {
                            joint.swing_compliance = (joint.swing_compliance + delta).max(0.0)
                        }
                        JointComplianceProperty::Twist => {
                            joint.twist_compliance = (joint.twist_compliance + delta).max(0.0)
                        }
                        _ => {}
                    }
                }
            }
            InspectorControl::JointDamping { kind, step } => {
                let delta = 0.1 * step_multiplier(step);
                if let Some(damping) = damping.as_deref_mut() {
                    match kind {
                        JointDampingKind::Linear => {
                            damping.linear = (damping.linear + delta).max(0.0)
                        }
                        JointDampingKind::Angular => {
                            damping.angular = (damping.angular + delta).max(0.0)
                        }
                    }
                } else {
                    let mut value = JointDamping::default();
                    match kind {
                        JointDampingKind::Linear => value.linear = delta.max(0.0),
                        JointDampingKind::Angular => value.angular = delta.max(0.0),
                    }
                    commands.entity(entity).insert(value);
                }
            }
            _ => {}
        }
    }
}

fn update_joint_anchor(anchor: &mut JointAnchor, axis: Axis, delta: f32) {
    let JointAnchor::Local(value) = anchor else {
        return;
    };
    change_axis(value, axis, delta);
}

fn scalar_step(property: ScalarProperty) -> f32 {
    match property {
        ScalarProperty::Mass => 0.5,
        ScalarProperty::GravityScale => 0.1,
        ScalarProperty::LinearDamping | ScalarProperty::AngularDamping => 0.1,
        ScalarProperty::Dominance => 1.0,
    }
}

fn step_multiplier(step: StepDirection) -> f32 {
    match step {
        StepDirection::Down => -1.0,
        StepDirection::Up => 1.0,
    }
}

fn change_axis(value: &mut Vec3, axis: Axis, delta: f32) {
    match axis {
        Axis::X => value.x += delta,
        Axis::Y => value.y += delta,
        Axis::Z => value.z += delta,
    }
}

/// Copies the selected entity's live Avian components into a stable, UI-ready
/// snapshot. The snapshot is refreshed every frame so velocity and sleeping
/// information remains useful while a body is moving or being grabbed.
#[allow(clippy::type_complexity)]
fn refresh_inspector(
    mut selection: ResMut<SelectionState>,
    mut inspector: ResMut<InspectorState>,
    bodies: Query<(
        Option<&Name>,
        &RigidBody,
        &Collider,
        Option<&Position>,
        Option<&Rotation>,
        Option<&Transform>,
        Option<&LinearVelocity>,
        Option<&AngularVelocity>,
    )>,
    body_details: Query<(
        Option<&Mass>,
        Option<&ComputedMass>,
        Option<&GravityScale>,
        Option<&LinearDamping>,
        Option<&AngularDamping>,
        Option<&LockedAxes>,
        Option<&Dominance>,
        Option<&Friction>,
        Option<&Restitution>,
        Has<Sleeping>,
        Has<SleepingDisabled>,
    )>,
    joint_names: Query<&Name>,
    joints: JointInspectorQuery<'_, '_>,
    contact_graph: Res<ContactGraph>,
) {
    let selected = selection.entity;
    let next_object = selected.and_then(|entity| {
        let (
            name,
            body,
            collider,
            position,
            rotation,
            transform,
            linear_velocity,
            angular_velocity,
        ) = bodies.get(entity).ok()?;
        let (
            mass,
            computed_mass,
            gravity_scale,
            linear_damping,
            angular_damping,
            locked_axes,
            dominance,
            friction,
            restitution,
            sleeping,
            sleeping_disabled,
        ) = body_details.get(entity).ok()?;

        let position = position
            .map(|position| position.0)
            .or_else(|| transform.map(|transform| transform.translation))
            .unwrap_or(Vec3::ZERO);
        let rotation = rotation
            .map(|rotation| rotation.0)
            .or_else(|| transform.map(|transform| transform.rotation))
            .unwrap_or(Quat::IDENTITY);
        let mass = mass.map(|mass| mass.0).or_else(|| {
            body.is_dynamic()
                .then(|| computed_mass.map(|computed_mass| computed_mass.value()))
                .flatten()
        });

        Some(InspectorSnapshot {
            entity,
            name: name
                .map(|name| name.as_str().to_owned())
                .unwrap_or_else(|| format!("Physics Entity {entity:?}")),
            body: *body,
            collider: ColliderInfo {
                shape: collider_shape_name(collider),
                scale: collider.scale(),
            },
            position,
            rotation,
            linear_velocity: linear_velocity
                .map(|velocity| velocity.0)
                .unwrap_or(Vec3::ZERO),
            angular_velocity: angular_velocity
                .map(|velocity| velocity.0)
                .unwrap_or(Vec3::ZERO),
            mass,
            gravity_scale: gravity_scale.map(|scale| scale.0).unwrap_or(1.0),
            linear_damping: linear_damping.map(|damping| damping.0).unwrap_or(0.0),
            angular_damping: angular_damping.map(|damping| damping.0).unwrap_or(0.0),
            locked_axes: locked_axes.map_or(0, LockedAxes::to_bits),
            dominance: dominance.map(|dominance| dominance.0).unwrap_or(0),
            friction: friction.map(|friction| FrictionInfo {
                dynamic: friction.dynamic_coefficient,
                static_coefficient: friction.static_coefficient,
            }),
            restitution: restitution.map(|restitution| restitution.coefficient),
            sleeping,
            sleeping_disabled,
            contacts: contact_snapshots(entity, &contact_graph, &joint_names),
        })
    });

    let next_joint = selected.and_then(|entity| {
        let (name, fixed, distance, revolute, prismatic, spherical, damping, forces, disabled) =
            joints.get(entity).ok()?;
        let details = joint_details(fixed, distance, revolute, prismatic, spherical)?;
        let [body1, body2] = details.bodies;
        let stress = forces.map(|forces| JointStressInfo {
            force: forces.force(),
            torque: forces.torque(),
            motor_force: forces.motor_force(),
        });

        Some(JointInspectorSnapshot {
            entity,
            name: name
                .map(|name| name.as_str().to_owned())
                .unwrap_or_else(|| format!("Physics Joint {entity:?}")),
            joint_type: details.joint_type,
            body1,
            body2,
            body1_name: joint_entity_name(&joint_names, body1),
            body2_name: joint_entity_name(&joint_names, body2),
            anchor1: details.anchors[0],
            anchor2: details.anchors[1],
            compliance: details.compliance,
            damping: damping.map_or(JointDampingInfo::default(), |damping| JointDampingInfo {
                linear: damping.linear,
                angular: damping.angular,
                present: true,
            }),
            stress,
            enabled: !disabled,
        })
    });

    if next_object.is_none() && next_joint.is_none() && selected.is_some() {
        selection.entity = None;
    }
    inspector.set_if_neq(InspectorState {
        object: next_object,
        joint: next_joint,
    });
}

fn joint_entity_name(names: &Query<&Name>, entity: Entity) -> String {
    names
        .get(entity)
        .map(|name| name.as_str().to_owned())
        .unwrap_or_else(|_| format!("Physics Entity {entity:?}"))
}

fn contact_snapshots(
    selected: Entity,
    contact_graph: &ContactGraph,
    names: &Query<&Name>,
) -> Vec<ContactInspectorSnapshot> {
    contact_graph
        .iter_active_touching()
        .chain(contact_graph.iter_sleeping_touching())
        .filter_map(|pair| {
            let selected_is_first = if pair.collider1 == selected || pair.body1 == Some(selected) {
                true
            } else if pair.collider2 == selected || pair.body2 == Some(selected) {
                false
            } else {
                return None;
            };
            let other_entity = if selected_is_first {
                pair.body2.unwrap_or(pair.collider2)
            } else {
                pair.body1.unwrap_or(pair.collider1)
            };
            let (manifold, contact) = pair
                .manifolds
                .iter()
                .filter_map(|manifold| {
                    manifold
                        .find_deepest_contact()
                        .map(|contact| (manifold, contact))
                })
                .max_by(|(_, first), (_, second)| {
                    first
                        .penetration
                        .partial_cmp(&second.penetration)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;
            let normal = if selected_is_first {
                manifold.normal
            } else {
                -manifold.normal
            };
            let relative_speed = if selected_is_first {
                contact.normal_speed
            } else {
                -contact.normal_speed
            };
            Some(ContactInspectorSnapshot {
                other_entity,
                other_name: joint_entity_name(names, other_entity),
                position: contact.point,
                normal,
                relative_speed,
                impulse: contact.normal_impulse.abs(),
            })
        })
        .collect()
}

fn joint_details(
    fixed: Option<&FixedJoint>,
    distance: Option<&DistanceJoint>,
    revolute: Option<&RevoluteJoint>,
    prismatic: Option<&PrismaticJoint>,
    spherical: Option<&SphericalJoint>,
) -> Option<JointDetails> {
    if let Some(joint) = fixed {
        return Some(JointDetails {
            joint_type: JointType::Fixed,
            bodies: [joint.body1, joint.body2],
            anchors: [joint.local_anchor1(), joint.local_anchor2()],
            compliance: JointComplianceInfo {
                point: Some(joint.point_compliance),
                angle: Some(joint.angle_compliance),
                ..default()
            },
        });
    }
    if let Some(joint) = distance {
        return Some(JointDetails {
            joint_type: JointType::Distance,
            bodies: [joint.body1, joint.body2],
            anchors: [joint.local_anchor1(), joint.local_anchor2()],
            compliance: JointComplianceInfo {
                point: Some(joint.compliance),
                ..default()
            },
        });
    }
    if let Some(joint) = revolute {
        return Some(JointDetails {
            joint_type: JointType::Revolute,
            bodies: [joint.body1, joint.body2],
            anchors: [joint.local_anchor1(), joint.local_anchor2()],
            compliance: JointComplianceInfo {
                point: Some(joint.point_compliance),
                alignment: Some(joint.align_compliance),
                limit: Some(joint.limit_compliance),
                ..default()
            },
        });
    }
    if let Some(joint) = prismatic {
        return Some(JointDetails {
            joint_type: JointType::Prismatic,
            bodies: [joint.body1, joint.body2],
            anchors: [joint.local_anchor1(), joint.local_anchor2()],
            compliance: JointComplianceInfo {
                alignment: Some(joint.align_compliance),
                angle: Some(joint.angle_compliance),
                limit: Some(joint.limit_compliance),
                ..default()
            },
        });
    }
    spherical.map(|joint| JointDetails {
        joint_type: JointType::Spherical,
        bodies: [joint.body1, joint.body2],
        anchors: [joint.local_anchor1(), joint.local_anchor2()],
        compliance: JointComplianceInfo {
            point: Some(joint.point_compliance),
            swing: Some(joint.swing_compliance),
            twist: Some(joint.twist_compliance),
            ..default()
        },
    })
}

fn collider_shape_name(collider: &Collider) -> String {
    match collider.shape().as_typed_shape() {
        TypedShape::Ball(_) => "Ball",
        TypedShape::Cuboid(_) => "Cuboid",
        TypedShape::Capsule(_) => "Capsule",
        TypedShape::Cylinder(_) => "Cylinder",
        TypedShape::Cone(_) => "Cone",
        TypedShape::RoundCuboid(_) => "Round cuboid",
        TypedShape::RoundCylinder(_) => "Round cylinder",
        TypedShape::RoundCone(_) => "Round cone",
        _ => "Compound/custom",
    }
    .to_owned()
}

#[derive(Component)]
struct InspectorPanel;

#[derive(Component)]
struct InspectorBodySection;

#[derive(Component)]
struct InspectorJointSection;

#[derive(Component)]
struct InspectorText;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorControl {
    Scalar {
        property: ScalarProperty,
        step: StepDirection,
    },
    ToggleAxisLock {
        kind: AxisLockKind,
        axis: Axis,
    },
    Velocity {
        kind: VelocityKind,
        axis: Axis,
        step: StepDirection,
    },
    CycleBody,
    ToggleJointEnabled,
    DeleteJoint,
    JointAnchor {
        body: JointBody,
        axis: Axis,
        step: StepDirection,
    },
    JointCompliance {
        property: JointComplianceProperty,
        step: StepDirection,
    },
    JointDamping {
        kind: JointDampingKind,
        step: StepDirection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarProperty {
    Mass,
    GravityScale,
    LinearDamping,
    AngularDamping,
    Dominance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JointBody {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JointComplianceProperty {
    Point,
    Alignment,
    Angle,
    Limit,
    Swing,
    Twist,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JointDampingKind {
    Linear,
    Angular,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AxisLockKind {
    Translation,
    Rotation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VelocityKind {
    Linear,
    Angular,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepDirection {
    Down,
    Up,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct InspectorValueText {
    property: InspectorValueProperty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorValueProperty {
    Scalar(ScalarProperty),
    AxisLock(AxisLockKind, Axis),
    Velocity(VelocityKind, Axis),
    Body,
    Joint(JointValueProperty),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JointValueProperty {
    Enabled,
    Anchor(JointBody, Axis),
    Compliance(JointComplianceProperty),
    Damping(JointDampingKind),
    Stress,
}

fn spawn_inspector_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16.0),
                right: px(16.0),
                width: px(420.0),
                max_height: percent(92.0),
                padding: UiRect::all(px(16.0)),
                display: Display::None,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            InspectorPanel,
            Name::new("Selected Object Inspector"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont::from_font_size(16.0),
                TextColor(PANEL_TEXT),
                InspectorText,
            ));
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    InspectorBodySection,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("EDITABLE PROPERTIES (click +/-)"),
                        TextFont::from_font_size(14.0),
                        TextColor(PANEL_TEXT),
                        Node {
                            margin: UiRect::top(px(12.0)),
                            ..default()
                        },
                    ));

                    spawn_scalar_row(parent, "Mass", ScalarProperty::Mass);
                    spawn_scalar_row(parent, "Gravity scale", ScalarProperty::GravityScale);
                    spawn_scalar_row(parent, "Linear damping", ScalarProperty::LinearDamping);
                    spawn_scalar_row(parent, "Angular damping", ScalarProperty::AngularDamping);
                    spawn_scalar_row(parent, "Dominance", ScalarProperty::Dominance);
                    spawn_body_row(parent);
                    for kind in [AxisLockKind::Translation, AxisLockKind::Rotation] {
                        for axis in [Axis::X, Axis::Y, Axis::Z] {
                            spawn_axis_lock_row(parent, kind, axis);
                        }
                    }
                    for axis in [Axis::X, Axis::Y, Axis::Z] {
                        spawn_velocity_row(parent, VelocityKind::Linear, axis);
                    }
                    for axis in [Axis::X, Axis::Y, Axis::Z] {
                        spawn_velocity_row(parent, VelocityKind::Angular, axis);
                    }
                });

            parent
                .spawn((
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    InspectorJointSection,
                ))
                .with_children(spawn_joint_inspector_ui);
        });
}

fn spawn_joint_inspector_ui(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Text::new("JOINT CONFIGURATION (click +/-)"),
        TextFont::from_font_size(14.0),
        TextColor(PANEL_TEXT),
        Node {
            margin: UiRect::top(px(12.0)),
            ..default()
        },
    ));
    spawn_joint_toggle_row(parent);
    for body in [JointBody::First, JointBody::Second] {
        for axis in [Axis::X, Axis::Y, Axis::Z] {
            spawn_joint_anchor_row(parent, body, axis);
        }
    }
    for property in [
        JointComplianceProperty::Point,
        JointComplianceProperty::Alignment,
        JointComplianceProperty::Angle,
        JointComplianceProperty::Limit,
        JointComplianceProperty::Swing,
        JointComplianceProperty::Twist,
    ] {
        spawn_joint_compliance_row(parent, property);
    }
    spawn_joint_damping_row(parent, JointDampingKind::Linear);
    spawn_joint_damping_row(parent, JointDampingKind::Angular);
    parent.spawn((
        Text::new("STRESS / SOLVER OUTPUT"),
        TextFont::from_font_size(13.0),
        TextColor(PANEL_TEXT),
        Node {
            margin: UiRect::top(px(8.0)),
            ..default()
        },
    ));
    parent.spawn((
        Text::new("-"),
        TextFont::from_font_size(12.0),
        TextColor(PANEL_TEXT),
        InspectorValueText {
            property: InspectorValueProperty::Joint(JointValueProperty::Stress),
        },
    ));
}

fn spawn_joint_toggle_row(parent: &mut ChildSpawnerCommands) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label("Enabled"));
        spawn_control_button(row, "toggle", InspectorControl::ToggleJointEnabled);
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Joint(JointValueProperty::Enabled),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        spawn_control_button(row, "delete", InspectorControl::DeleteJoint);
    });
}

fn spawn_joint_anchor_row(parent: &mut ChildSpawnerCommands, body: JointBody, axis: Axis) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label(match (body, axis) {
            (JointBody::First, Axis::X) => "Anchor 1 X",
            (JointBody::First, Axis::Y) => "Anchor 1 Y",
            (JointBody::First, Axis::Z) => "Anchor 1 Z",
            (JointBody::Second, Axis::X) => "Anchor 2 X",
            (JointBody::Second, Axis::Y) => "Anchor 2 Y",
            (JointBody::Second, Axis::Z) => "Anchor 2 Z",
        }));
        spawn_control_button(
            row,
            "-",
            InspectorControl::JointAnchor {
                body,
                axis,
                step: StepDirection::Down,
            },
        );
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Joint(JointValueProperty::Anchor(body, axis)),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        spawn_control_button(
            row,
            "+",
            InspectorControl::JointAnchor {
                body,
                axis,
                step: StepDirection::Up,
            },
        );
    });
}

fn spawn_joint_compliance_row(
    parent: &mut ChildSpawnerCommands,
    property: JointComplianceProperty,
) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label(match property {
            JointComplianceProperty::Point => "Point compliance",
            JointComplianceProperty::Alignment => "Alignment compliance",
            JointComplianceProperty::Angle => "Angle compliance",
            JointComplianceProperty::Limit => "Limit compliance",
            JointComplianceProperty::Swing => "Swing compliance",
            JointComplianceProperty::Twist => "Twist compliance",
        }));
        spawn_control_button(
            row,
            "-",
            InspectorControl::JointCompliance {
                property,
                step: StepDirection::Down,
            },
        );
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Joint(JointValueProperty::Compliance(property)),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        spawn_control_button(
            row,
            "+",
            InspectorControl::JointCompliance {
                property,
                step: StepDirection::Up,
            },
        );
    });
}

fn spawn_joint_damping_row(parent: &mut ChildSpawnerCommands, kind: JointDampingKind) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label(match kind {
            JointDampingKind::Linear => "Linear joint damping",
            JointDampingKind::Angular => "Angular joint damping",
        }));
        spawn_control_button(
            row,
            "-",
            InspectorControl::JointDamping {
                kind,
                step: StepDirection::Down,
            },
        );
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Joint(JointValueProperty::Damping(kind)),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        spawn_control_button(
            row,
            "+",
            InspectorControl::JointDamping {
                kind,
                step: StepDirection::Up,
            },
        );
    });
}

fn spawn_scalar_row(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    property: ScalarProperty,
) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label(label));
        spawn_control_button(
            row,
            "-",
            InspectorControl::Scalar {
                property,
                step: StepDirection::Down,
            },
        );
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Scalar(property),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        spawn_control_button(
            row,
            "+",
            InspectorControl::Scalar {
                property,
                step: StepDirection::Up,
            },
        );
    });
}

fn spawn_axis_lock_row(parent: &mut ChildSpawnerCommands, kind: AxisLockKind, axis: Axis) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label(match (kind, axis) {
            (AxisLockKind::Translation, Axis::X) => "Lock translation X",
            (AxisLockKind::Translation, Axis::Y) => "Lock translation Y",
            (AxisLockKind::Translation, Axis::Z) => "Lock translation Z",
            (AxisLockKind::Rotation, Axis::X) => "Lock rotation X",
            (AxisLockKind::Rotation, Axis::Y) => "Lock rotation Y",
            (AxisLockKind::Rotation, Axis::Z) => "Lock rotation Z",
        }));
        spawn_control_button(
            row,
            "toggle",
            InspectorControl::ToggleAxisLock { kind, axis },
        );
        row.spawn((
            Text::new("open"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::AxisLock(kind, axis),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
    });
}

fn spawn_body_row(parent: &mut ChildSpawnerCommands) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label("Body type"));
        spawn_control_button(row, "cycle", InspectorControl::CycleBody);
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Body,
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
    });
}

fn spawn_velocity_row(parent: &mut ChildSpawnerCommands, kind: VelocityKind, axis: Axis) {
    parent.spawn(control_row_node()).with_children(|row| {
        row.spawn(control_label(match axis {
            Axis::X => match kind {
                VelocityKind::Linear => "Linear velocity X",
                VelocityKind::Angular => "Angular velocity X",
            },
            Axis::Y => match kind {
                VelocityKind::Linear => "Linear velocity Y",
                VelocityKind::Angular => "Angular velocity Y",
            },
            Axis::Z => match kind {
                VelocityKind::Linear => "Linear velocity Z",
                VelocityKind::Angular => "Angular velocity Z",
            },
        }));
        spawn_control_button(
            row,
            "-",
            InspectorControl::Velocity {
                kind,
                axis,
                step: StepDirection::Down,
            },
        );
        row.spawn((
            Text::new("-"),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
            InspectorValueText {
                property: InspectorValueProperty::Velocity(kind, axis),
            },
            Node {
                width: px(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ));
        spawn_control_button(
            row,
            "+",
            InspectorControl::Velocity {
                kind,
                axis,
                step: StepDirection::Up,
            },
        );
    });
}

fn control_row_node() -> Node {
    Node {
        width: percent(100.0),
        height: px(26.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn control_label(label: &'static str) -> impl Bundle {
    (
        Text::new(label),
        TextFont::from_font_size(13.0),
        TextColor(PANEL_TEXT),
        Node {
            width: px(170.0),
            ..default()
        },
    )
}

fn spawn_control_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    control: InspectorControl,
) {
    parent
        .spawn((
            Button,
            control,
            Node {
                width: px(38.0),
                height: px(22.0),
                margin: UiRect::horizontal(px(2.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CONTROL_BACKGROUND),
            Name::new(format!("Inspector control {label}")),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(12.0),
            TextColor(PANEL_TEXT),
        ));
}

#[allow(clippy::type_complexity)]
fn update_inspector_ui(
    inspector: Res<InspectorState>,
    mut nodes: ParamSet<(
        Query<&mut Node, With<InspectorPanel>>,
        Query<&mut Node, With<InspectorBodySection>>,
        Query<&mut Node, With<InspectorJointSection>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<InspectorText>>,
        Query<(&InspectorValueText, &mut Text)>,
    )>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<InspectorControl>>,
) {
    let has_selection = inspector.object.is_some() || inspector.joint.is_some();
    {
        let mut panels = nodes.p0();
        let Ok(mut panel) = panels.single_mut() else {
            return;
        };
        panel.display = if has_selection {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !has_selection {
        if let Ok(mut summary_text) = texts.p0().single_mut() {
            summary_text.0.clear();
        }
        return;
    }
    for mut section in &mut nodes.p1() {
        section.display = inspector
            .object
            .as_ref()
            .map_or(Display::None, |_| Display::Flex);
    }
    for mut section in &mut nodes.p2() {
        section.display = inspector
            .joint
            .as_ref()
            .map_or(Display::None, |_| Display::Flex);
    }
    if let Ok(mut summary_text) = texts.p0().single_mut() {
        summary_text.0 = inspector
            .object
            .as_ref()
            .map(format_inspector_text)
            .or_else(|| inspector.joint.as_ref().map(format_joint_inspector_text))
            .unwrap_or_default();
    }
    for (value, mut text) in &mut texts.p1() {
        text.0 = format_inspector_value(&inspector, value.property);
    }
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed => CONTROL_PRESSED.into(),
            Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn format_inspector_value(inspector: &InspectorState, property: InspectorValueProperty) -> String {
    match property {
        InspectorValueProperty::Scalar(property) => {
            let Some(object) = inspector.object.as_ref() else {
                return "-".to_owned();
            };
            format_float(match property {
                ScalarProperty::Mass => object.mass.unwrap_or(0.0),
                ScalarProperty::GravityScale => object.gravity_scale,
                ScalarProperty::LinearDamping => object.linear_damping,
                ScalarProperty::AngularDamping => object.angular_damping,
                ScalarProperty::Dominance => object.dominance as f32,
            })
        }
        InspectorValueProperty::AxisLock(kind, axis) => {
            let Some(object) = inspector.object.as_ref() else {
                return "-".to_owned();
            };
            if axis_is_locked(LockedAxes::from_bits(object.locked_axes), kind, axis) {
                "locked".to_owned()
            } else {
                "open".to_owned()
            }
        }
        InspectorValueProperty::Velocity(kind, axis) => {
            let Some(object) = inspector.object.as_ref() else {
                return "-".to_owned();
            };
            let velocity = match kind {
                VelocityKind::Linear => object.linear_velocity,
                VelocityKind::Angular => object.angular_velocity,
            };
            format_float(axis_value(velocity, axis))
        }
        InspectorValueProperty::Body => inspector
            .object
            .as_ref()
            .map_or_else(|| "-".to_owned(), |object| format!("{:?}", object.body)),
        InspectorValueProperty::Joint(property) => {
            let Some(joint) = inspector.joint.as_ref() else {
                return "-".to_owned();
            };
            format_joint_value(joint, property)
        }
    }
}

fn format_joint_value(joint: &JointInspectorSnapshot, property: JointValueProperty) -> String {
    match property {
        JointValueProperty::Enabled => {
            if joint.enabled {
                "enabled".to_owned()
            } else {
                "disabled".to_owned()
            }
        }
        JointValueProperty::Anchor(body, axis) => match body {
            JointBody::First => joint.anchor1,
            JointBody::Second => joint.anchor2,
        }
        .map_or_else(
            || "global".to_owned(),
            |anchor| format_float(axis_value(anchor, axis)),
        ),
        JointValueProperty::Compliance(property) => compliance_value(joint.compliance, property)
            .map_or_else(|| "n/a".to_owned(), format_float),
        JointValueProperty::Damping(kind) => format_float(match kind {
            JointDampingKind::Linear => joint.damping.linear,
            JointDampingKind::Angular => joint.damping.angular,
        }),
        JointValueProperty::Stress => joint.stress.map_or_else(
            || "unavailable".to_owned(),
            |stress| {
                format!(
                    "force {} | torque {} | motor {}",
                    format_vec3(stress.force),
                    format_vec3(stress.torque),
                    format_float(stress.motor_force),
                )
            },
        ),
    }
}

fn compliance_value(
    compliance: JointComplianceInfo,
    property: JointComplianceProperty,
) -> Option<f32> {
    match property {
        JointComplianceProperty::Point => compliance.point,
        JointComplianceProperty::Alignment => compliance.alignment,
        JointComplianceProperty::Angle => compliance.angle,
        JointComplianceProperty::Limit => compliance.limit,
        JointComplianceProperty::Swing => compliance.swing,
        JointComplianceProperty::Twist => compliance.twist,
    }
}

fn format_joint_inspector_text(joint: &JointInspectorSnapshot) -> String {
    let stress = joint.stress.map_or_else(
        || "unavailable".to_owned(),
        |stress| {
            format!(
                "force {} | torque {} | motor {}",
                format_vec3(stress.force),
                format_vec3(stress.torque),
                format_float(stress.motor_force),
            )
        },
    );
    format!(
        "{}\nEntity: {:?}\n\nJOINT\nType: {:?}\nEnabled: {}\nBody 1: {} ({:?})\nBody 2: {} ({:?})\nAnchor 1: {}\nAnchor 2: {}\nCompliance: {:?}\nDamping: linear {}, angular {}\nStress: {}",
        joint.name,
        joint.entity,
        joint.joint_type,
        joint.enabled,
        joint.body1_name,
        joint.body1,
        joint.body2_name,
        joint.body2,
        joint
            .anchor1
            .map_or_else(|| "global/unknown".to_owned(), format_vec3),
        joint
            .anchor2
            .map_or_else(|| "global/unknown".to_owned(), format_vec3),
        joint.compliance,
        format_float(joint.damping.linear),
        format_float(joint.damping.angular),
        stress,
    )
}

fn format_inspector_text(object: &InspectorSnapshot) -> String {
    let mass = object
        .mass
        .map_or_else(|| "infinite/default".to_owned(), format_float);
    let friction = object.friction.as_ref().map_or_else(
        || "default".to_owned(),
        |friction| {
            format!(
                "dynamic {}, static {}",
                format_float(friction.dynamic),
                format_float(friction.static_coefficient)
            )
        },
    );
    let restitution = object
        .restitution
        .map_or_else(|| "default".to_owned(), format_float);
    let contacts = format_contact_inspector_text(&object.contacts);

    format!(
        "{}\nEntity: {:?}\n\nBODY\nType: {:?}\nMass: {}\nGravity scale: {}\nLinear damping: {}\nAngular damping: {}\nDominance: {}\nAxis locks: {}\n\nCOLLIDER\nShape: {}\nScale: {}\nFriction: {}\nRestitution: {}\n\nCURRENT STATE\nPosition: {}\nRotation: {}\nLinear velocity: {}\nAngular velocity: {}\nSleeping: {}\nSleeping disabled: {}\n\nCONTACTS (LIVE)\n{}",
        object.name,
        object.entity,
        object.body,
        mass,
        format_float(object.gravity_scale),
        format_float(object.linear_damping),
        format_float(object.angular_damping),
        object.dominance,
        format_axis_locks(LockedAxes::from_bits(object.locked_axes)),
        object.collider.shape,
        format_vec3(object.collider.scale),
        friction,
        restitution,
        format_vec3(object.position),
        format_quat(object.rotation),
        format_vec3(object.linear_velocity),
        format_vec3(object.angular_velocity),
        object.sleeping,
        object.sleeping_disabled,
        contacts,
    )
}

fn format_contact_inspector_text(contacts: &[ContactInspectorSnapshot]) -> String {
    if contacts.is_empty() {
        return "No active contacts.".to_owned();
    }

    contacts
        .iter()
        .enumerate()
        .map(|(index, contact)| {
            format!(
                "Contact {}\nPartner: {} ({:?})\nPosition: {}\nNormal: {}\nRelative speed: {} m/s\nImpulse: {} N·s",
                index + 1,
                contact.other_name,
                contact.other_entity,
                format_vec3(contact.position),
                format_vec3(contact.normal),
                format_float(contact.relative_speed),
                format_float(contact.impulse),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_float(value: f32) -> String {
    format!("{value:.2}")
}

fn toggle_axis_lock(locked_axes: LockedAxes, kind: AxisLockKind, axis: Axis) -> LockedAxes {
    let is_locked = axis_is_locked(locked_axes, kind, axis);
    match (kind, axis, is_locked) {
        (AxisLockKind::Translation, Axis::X, false) => locked_axes.lock_translation_x(),
        (AxisLockKind::Translation, Axis::X, true) => locked_axes.unlock_translation_x(),
        (AxisLockKind::Translation, Axis::Y, false) => locked_axes.lock_translation_y(),
        (AxisLockKind::Translation, Axis::Y, true) => locked_axes.unlock_translation_y(),
        (AxisLockKind::Translation, Axis::Z, false) => locked_axes.lock_translation_z(),
        (AxisLockKind::Translation, Axis::Z, true) => locked_axes.unlock_translation_z(),
        (AxisLockKind::Rotation, Axis::X, false) => locked_axes.lock_rotation_x(),
        (AxisLockKind::Rotation, Axis::X, true) => locked_axes.unlock_rotation_x(),
        (AxisLockKind::Rotation, Axis::Y, false) => locked_axes.lock_rotation_y(),
        (AxisLockKind::Rotation, Axis::Y, true) => locked_axes.unlock_rotation_y(),
        (AxisLockKind::Rotation, Axis::Z, false) => locked_axes.lock_rotation_z(),
        (AxisLockKind::Rotation, Axis::Z, true) => locked_axes.unlock_rotation_z(),
    }
}

fn axis_is_locked(locked_axes: LockedAxes, kind: AxisLockKind, axis: Axis) -> bool {
    match (kind, axis) {
        (AxisLockKind::Translation, Axis::X) => locked_axes.is_translation_x_locked(),
        (AxisLockKind::Translation, Axis::Y) => locked_axes.is_translation_y_locked(),
        (AxisLockKind::Translation, Axis::Z) => locked_axes.is_translation_z_locked(),
        (AxisLockKind::Rotation, Axis::X) => locked_axes.is_rotation_x_locked(),
        (AxisLockKind::Rotation, Axis::Y) => locked_axes.is_rotation_y_locked(),
        (AxisLockKind::Rotation, Axis::Z) => locked_axes.is_rotation_z_locked(),
    }
}

fn format_axis_locks(locked_axes: LockedAxes) -> String {
    let axis = |locked: bool| if locked { "locked" } else { "open" };
    format!(
        "T({},{},{}) R({},{},{})",
        axis(locked_axes.is_translation_x_locked()),
        axis(locked_axes.is_translation_y_locked()),
        axis(locked_axes.is_translation_z_locked()),
        axis(locked_axes.is_rotation_x_locked()),
        axis(locked_axes.is_rotation_y_locked()),
        axis(locked_axes.is_rotation_z_locked()),
    )
}

fn axis_value(value: Vec3, axis: Axis) -> f32 {
    match axis {
        Axis::X => value.x,
        Axis::Y => value.y,
        Axis::Z => value.z,
    }
}

fn format_vec3(value: Vec3) -> String {
    format!("({:.2}, {:.2}, {:.2})", value.x, value.y, value.z)
}

fn format_quat(value: Quat) -> String {
    format!(
        "({:.2}, {:.2}, {:.2}, {:.2})",
        value.x, value.y, value.z, value.w
    )
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
    use crate::cursor_hover::{CursorHoverPlugin, CursorRay};

    fn inspector_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            InputPlugin,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CursorHoverPlugin,
            ObjectInspectorPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )))
        .insert_resource(Gravity(Vec3::ZERO));
        app.finish();
        app
    }

    fn spawn_body(app: &mut App, name: &str, body: RigidBody, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                body,
                Collider::cuboid(0.5, 0.5, 0.5),
                Position(position),
                Transform::from_translation(position),
                Name::new(name.to_owned()),
            ))
            .id()
    }

    fn spawn_sphere(app: &mut App, name: &str, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                Position(position),
                Transform::from_translation(position),
                Name::new(name.to_owned()),
            ))
            .id()
    }

    fn point_cursor_at(app: &mut App, target: Vec3) {
        let direction = Dir3::new(target.normalize()).expect("test ray direction");
        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::ZERO, direction);
        app.update();
    }

    fn click_left(app: &mut App) {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut()
                .resource_mut::<Messages<MouseButtonInput>>()
                .write(MouseButtonInput {
                    button: MouseButton::Left,
                    state,
                    window: Entity::PLACEHOLDER,
                });
            app.update();
        }
    }

    fn inspector_text(app: &mut App) -> String {
        let world = app.world_mut();
        let mut texts = world.query_filtered::<&Text, With<InspectorText>>();
        texts.iter(world).next().expect("inspector text").0.clone()
    }

    fn select_entity(app: &mut App, entity: Entity) {
        app.world_mut().resource_mut::<SelectionState>().entity = Some(entity);
        app.update();
    }

    fn press_control(app: &mut App, control: InspectorControl) {
        let button = {
            let world = app.world_mut();
            let mut controls = world.query::<(Entity, &InspectorControl)>();
            controls
                .iter(world)
                .find_map(|(entity, candidate)| (*candidate == control).then_some(entity))
                .expect("inspector control")
        };
        app.world_mut()
            .entity_mut(button)
            .get_mut::<Interaction>()
            .expect("button interaction")
            .clone_from(&Interaction::Pressed);
        app.update();
        app.world_mut()
            .entity_mut(button)
            .get_mut::<Interaction>()
            .expect("button interaction")
            .clone_from(&Interaction::None);
        app.update();
    }

    fn editable_body(app: &mut App, position: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                Mass(2.0),
                GravityScale(1.0),
                LinearDamping(0.0),
                AngularDamping(0.0),
                LinearVelocity::default(),
                AngularVelocity::default(),
                Position(position),
                Transform::from_translation(position),
                Name::new("Editable body"),
            ))
            .id()
    }

    #[test]
    fn selecting_a_body_populates_body_collider_and_current_state() {
        let mut app = inspector_app();
        let body = spawn_body(
            &mut app,
            "Inspector Cube",
            RigidBody::Dynamic,
            Vec3::new(0.0, 0.0, -5.0),
        );
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        click_left(&mut app);

        let inspector = app.world().resource::<InspectorState>();
        let object = inspector.object.as_ref().expect("selected object");
        assert_eq!(object.entity, body);
        assert_eq!(object.name, "Inspector Cube");
        assert_eq!(object.body, RigidBody::Dynamic);
        assert_eq!(object.collider.shape, "Cuboid");
        assert_eq!(object.position, Vec3::new(0.0, 0.0, -5.0));
        assert_eq!(object.linear_velocity, Vec3::ZERO);

        let text = inspector_text(&mut app);
        assert!(text.contains("Inspector Cube"));
        assert!(text.contains("Type: Dynamic"));
        assert!(text.contains("Shape: Cuboid"));
        assert!(text.contains("CURRENT STATE"));
        assert!(text.contains("Linear velocity: (0.00, 0.00, 0.00)"));
    }

    #[test]
    fn changing_selection_refreshes_the_inspector_for_different_body_types() {
        let mut app = inspector_app();
        let static_body = spawn_body(
            &mut app,
            "Static Wall",
            RigidBody::Static,
            Vec3::new(-2.0, 0.0, -5.0),
        );
        let dynamic_body = spawn_sphere(&mut app, "Dynamic Sphere", Vec3::new(2.0, 0.0, -5.0));
        let kinematic_body = spawn_body(
            &mut app,
            "Kinematic Platform",
            RigidBody::Kinematic,
            Vec3::new(0.0, 0.0, -5.0),
        );
        app.update();

        for (position, entity, body, shape, name) in [
            (
                Vec3::new(-2.0, 0.0, -5.0),
                static_body,
                RigidBody::Static,
                "Cuboid",
                "Static Wall",
            ),
            (
                Vec3::new(2.0, 0.0, -5.0),
                dynamic_body,
                RigidBody::Dynamic,
                "Ball",
                "Dynamic Sphere",
            ),
            (
                Vec3::new(0.0, 0.0, -5.0),
                kinematic_body,
                RigidBody::Kinematic,
                "Cuboid",
                "Kinematic Platform",
            ),
        ] {
            point_cursor_at(&mut app, position);
            click_left(&mut app);
            let object = app
                .world()
                .resource::<InspectorState>()
                .object
                .as_ref()
                .expect("selected object");
            assert_eq!(object.entity, entity);
            assert_eq!(object.body, body);
            assert_eq!(object.collider.shape, shape);
            assert_eq!(object.name, name);
        }
    }

    #[test]
    fn editable_controls_change_all_core_rigid_body_properties() {
        let mut app = inspector_app();
        let body = editable_body(&mut app, Vec3::ZERO);
        app.update();
        select_entity(&mut app, body);

        press_control(
            &mut app,
            InspectorControl::Scalar {
                property: ScalarProperty::Mass,
                step: StepDirection::Up,
            },
        );
        press_control(
            &mut app,
            InspectorControl::Scalar {
                property: ScalarProperty::GravityScale,
                step: StepDirection::Down,
            },
        );
        press_control(
            &mut app,
            InspectorControl::Scalar {
                property: ScalarProperty::LinearDamping,
                step: StepDirection::Up,
            },
        );
        press_control(
            &mut app,
            InspectorControl::Scalar {
                property: ScalarProperty::AngularDamping,
                step: StepDirection::Up,
            },
        );
        press_control(
            &mut app,
            InspectorControl::Velocity {
                kind: VelocityKind::Linear,
                axis: Axis::X,
                step: StepDirection::Up,
            },
        );
        press_control(
            &mut app,
            InspectorControl::Velocity {
                kind: VelocityKind::Angular,
                axis: Axis::Z,
                step: StepDirection::Down,
            },
        );

        let object = app.world().entity(body);
        assert_eq!(object.get::<Mass>(), Some(&Mass(2.5)));
        assert_eq!(object.get::<GravityScale>(), Some(&GravityScale(0.9)));
        assert_eq!(object.get::<LinearDamping>(), Some(&LinearDamping(0.1)));
        assert_eq!(object.get::<AngularDamping>(), Some(&AngularDamping(0.1)));
        let linear_velocity = object.get::<LinearVelocity>().unwrap().0;
        assert!((linear_velocity - Vec3::X).length() < 0.01);
        let angular_velocity = object.get::<AngularVelocity>().unwrap().0;
        assert!((angular_velocity + Vec3::Z).length() < 0.01);

        press_control(&mut app, InspectorControl::CycleBody);
        assert_eq!(
            app.world().entity(body).get::<RigidBody>(),
            Some(&RigidBody::Kinematic)
        );
        press_control(&mut app, InspectorControl::CycleBody);
        assert_eq!(
            app.world().entity(body).get::<RigidBody>(),
            Some(&RigidBody::Static)
        );
        press_control(&mut app, InspectorControl::CycleBody);
        assert_eq!(
            app.world().entity(body).get::<RigidBody>(),
            Some(&RigidBody::Dynamic)
        );
    }

    #[test]
    fn axis_lock_controls_toggle_all_six_axes_and_dominance() {
        let mut app = inspector_app();
        let body = editable_body(&mut app, Vec3::ZERO);
        app.update();
        select_entity(&mut app, body);

        for (kind, axis) in [
            (AxisLockKind::Translation, Axis::X),
            (AxisLockKind::Translation, Axis::Y),
            (AxisLockKind::Translation, Axis::Z),
            (AxisLockKind::Rotation, Axis::X),
            (AxisLockKind::Rotation, Axis::Y),
            (AxisLockKind::Rotation, Axis::Z),
        ] {
            press_control(&mut app, InspectorControl::ToggleAxisLock { kind, axis });
        }

        let locked_axes = app
            .world()
            .entity(body)
            .get::<LockedAxes>()
            .copied()
            .unwrap();
        assert_eq!(locked_axes.to_bits(), LockedAxes::ALL_LOCKED.to_bits());

        press_control(
            &mut app,
            InspectorControl::ToggleAxisLock {
                kind: AxisLockKind::Rotation,
                axis: Axis::Y,
            },
        );
        assert!(
            !app.world()
                .entity(body)
                .get::<LockedAxes>()
                .unwrap()
                .is_rotation_y_locked()
        );

        for _ in 0..5 {
            press_control(
                &mut app,
                InspectorControl::Scalar {
                    property: ScalarProperty::Dominance,
                    step: StepDirection::Up,
                },
            );
        }
        assert_eq!(
            app.world().entity(body).get::<Dominance>(),
            Some(&Dominance(5))
        );

        let snapshot = app
            .world()
            .resource::<InspectorState>()
            .object
            .as_ref()
            .expect("selected body snapshot");
        assert_eq!(snapshot.dominance, 5);
        assert_eq!(
            snapshot.locked_axes,
            LockedAxes::ALL_LOCKED.unlock_rotation_y().to_bits()
        );
    }

    #[test]
    fn force_and_torque_respect_each_locked_translation_and_rotation_axis() {
        let mut app = inspector_app();
        let bodies = [
            (LockedAxes::new().lock_translation_x(), Axis::X, true),
            (LockedAxes::new().lock_translation_y(), Axis::Y, true),
            (LockedAxes::new().lock_translation_z(), Axis::Z, true),
            (LockedAxes::new().lock_rotation_x(), Axis::X, false),
            (LockedAxes::new().lock_rotation_y(), Axis::Y, false),
            (LockedAxes::new().lock_rotation_z(), Axis::Z, false),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (locked_axes, axis, translation))| {
            let position = Vec3::new(index as f32 * 4.0, 0.0, 0.0);
            let entity = app
                .world_mut()
                .spawn((
                    RigidBody::Dynamic,
                    Collider::cuboid(0.5, 0.5, 0.5),
                    Mass(1.0),
                    GravityScale(0.0),
                    LinearDamping(0.0),
                    AngularDamping(0.0),
                    locked_axes,
                    ConstantForce(Vec3::splat(10.0)),
                    ConstantTorque(Vec3::splat(10.0)),
                    Position(position),
                    Transform::from_translation(position),
                ))
                .id();
            (entity, axis, translation)
        })
        .collect::<Vec<_>>();

        for _ in 0..60 {
            app.update();
        }

        for (entity, axis, translation_locked) in bodies {
            let body = app.world().entity(entity);
            let velocity = body.get::<LinearVelocity>().unwrap().0;
            let angular_velocity = body.get::<AngularVelocity>().unwrap().0;
            let locked_linear = axis_value(velocity, axis);
            let locked_angular = axis_value(angular_velocity, axis);
            if translation_locked {
                assert!(
                    locked_linear.abs() < 0.01,
                    "translation lock {axis:?} leaked velocity: {velocity:?}"
                );
                assert!(
                    angular_velocity.length() > 0.1,
                    "translation lock {axis:?} unexpectedly blocked torque: {angular_velocity:?}"
                );
            } else {
                assert!(
                    locked_angular.abs() < 0.01,
                    "rotation lock {axis:?} leaked angular velocity: {angular_velocity:?}"
                );
                assert!(
                    velocity.length() > 0.1,
                    "rotation lock {axis:?} unexpectedly blocked force: {velocity:?}"
                );
            }
        }
    }

    #[test]
    fn edited_velocity_and_gravity_scale_affect_the_next_physics_frames() {
        let mut app = inspector_app();
        app.world_mut()
            .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)));
        let body = editable_body(&mut app, Vec3::new(0.0, 10.0, 0.0));
        app.world_mut().entity_mut(body).insert(GravityScale(0.0));
        app.update();
        select_entity(&mut app, body);

        press_control(
            &mut app,
            InspectorControl::Velocity {
                kind: VelocityKind::Linear,
                axis: Axis::X,
                step: StepDirection::Up,
            },
        );
        let position_after_velocity_edit = app.world().entity(body).get::<Position>().unwrap().0;
        assert!(position_after_velocity_edit.x > 0.0);

        for _ in 0..10 {
            press_control(
                &mut app,
                InspectorControl::Scalar {
                    property: ScalarProperty::GravityScale,
                    step: StepDirection::Up,
                },
            );
        }
        let y_before_gravity = app.world().entity(body).get::<Position>().unwrap().0.y;
        app.update();
        let y_after_gravity = app.world().entity(body).get::<Position>().unwrap().0.y;
        assert!(y_after_gravity < y_before_gravity);
    }

    #[test]
    fn increasing_mass_reduces_the_response_to_the_same_force() {
        let mut app = inspector_app();
        let light = editable_body(&mut app, Vec3::new(-5.0, 0.0, 0.0));
        let heavy = editable_body(&mut app, Vec3::new(5.0, 0.0, 0.0));
        app.world_mut()
            .entity_mut(light)
            .insert(ConstantForce(Vec3::X * 10.0));
        app.world_mut()
            .entity_mut(heavy)
            .insert(ConstantForce(Vec3::X * 10.0));
        app.update();
        select_entity(&mut app, heavy);

        for _ in 0..6 {
            press_control(
                &mut app,
                InspectorControl::Scalar {
                    property: ScalarProperty::Mass,
                    step: StepDirection::Up,
                },
            );
        }
        for _ in 0..30 {
            app.update();
        }

        let light_velocity = app
            .world()
            .entity(light)
            .get::<LinearVelocity>()
            .unwrap()
            .0
            .x;
        let heavy_velocity = app
            .world()
            .entity(heavy)
            .get::<LinearVelocity>()
            .unwrap()
            .0
            .x;
        assert!(
            light_velocity > heavy_velocity * 2.0,
            "mass edit did not reduce force response: light={light_velocity}, heavy={heavy_velocity}"
        );
    }

    fn spawn_joint_bodies(app: &mut App, z: f32) -> [Entity; 2] {
        [
            spawn_body(
                app,
                "Joint Body 1",
                RigidBody::Dynamic,
                Vec3::new(-1.0, 0.0, z),
            ),
            spawn_body(
                app,
                "Joint Body 2",
                RigidBody::Dynamic,
                Vec3::new(1.0, 0.0, z),
            ),
        ]
    }

    fn spawn_inspectable_joint(app: &mut App, joint_type: JointType, z: f32) -> Entity {
        let [body1, body2] = spawn_joint_bodies(app, z);
        let anchor1 = Vec3::X * 0.5;
        let anchor2 = Vec3::NEG_X * 0.5;
        let mut entity = app.world_mut().spawn_empty();
        match joint_type {
            JointType::Fixed => {
                entity.insert(
                    FixedJoint::new(body1, body2)
                        .with_local_anchor1(anchor1)
                        .with_local_anchor2(anchor2),
                );
            }
            JointType::Distance => {
                entity.insert(
                    DistanceJoint::new(body1, body2)
                        .with_local_anchor1(anchor1)
                        .with_local_anchor2(anchor2),
                );
            }
            JointType::Revolute => {
                entity.insert(
                    RevoluteJoint::new(body1, body2)
                        .with_local_anchor1(anchor1)
                        .with_local_anchor2(anchor2),
                );
            }
            JointType::Prismatic => {
                entity.insert(
                    PrismaticJoint::new(body1, body2)
                        .with_local_anchor1(anchor1)
                        .with_local_anchor2(anchor2),
                );
            }
            JointType::Spherical => {
                entity.insert(
                    SphericalJoint::new(body1, body2)
                        .with_local_anchor1(anchor1)
                        .with_local_anchor2(anchor2),
                );
            }
        }
        entity
            .insert((
                JointForces::new(),
                JointDamping {
                    linear: 0.2,
                    angular: 0.3,
                },
                Name::new(format!("{joint_type:?} inspection joint")),
            ))
            .id()
    }

    #[test]
    fn clicking_a_joint_anchor_selects_and_identifies_every_joint_type() {
        let mut app = inspector_app();
        let joint = spawn_inspectable_joint(&mut app, JointType::Fixed, -5.0);
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        click_left(&mut app);
        assert_eq!(app.world().resource::<SelectionState>().entity, Some(joint));
        let selected = app
            .world()
            .resource::<InspectorState>()
            .joint
            .as_ref()
            .expect("selected joint");
        assert_eq!(selected.joint_type, JointType::Fixed);
        assert_eq!(selected.body1_name, "Joint Body 1");
        assert_eq!(selected.body2_name, "Joint Body 2");
        assert_eq!(selected.anchor1, Some(Vec3::X * 0.5));
        assert_eq!(selected.anchor2, Some(Vec3::NEG_X * 0.5));
        assert!(inspector_text(&mut app).contains("Type: Fixed"));

        for (index, joint_type) in [
            JointType::Fixed,
            JointType::Distance,
            JointType::Revolute,
            JointType::Prismatic,
            JointType::Spherical,
        ]
        .into_iter()
        .enumerate()
        {
            let joint = if index == 0 {
                joint
            } else {
                spawn_inspectable_joint(&mut app, joint_type, -10.0 - index as f32)
            };
            select_entity(&mut app, joint);
            assert_eq!(
                app.world()
                    .resource::<InspectorState>()
                    .joint
                    .as_ref()
                    .expect("joint snapshot")
                    .joint_type,
                joint_type
            );
        }
    }

    #[test]
    fn joint_controls_edit_configuration_stress_and_lifecycle_while_bodies_are_active() {
        let mut app = inspector_app();
        let joint = spawn_inspectable_joint(&mut app, JointType::Fixed, -5.0);
        app.update();
        select_entity(&mut app, joint);

        let snapshot = app
            .world()
            .resource::<InspectorState>()
            .joint
            .as_ref()
            .expect("joint snapshot");
        let stress = snapshot.stress.expect("joint stress output");
        assert!(stress.force.is_finite());
        assert!(stress.torque.is_finite());
        assert!(stress.motor_force.is_finite());
        assert_eq!(snapshot.damping.linear, 0.2);

        press_control(
            &mut app,
            InspectorControl::JointAnchor {
                body: JointBody::First,
                axis: Axis::Y,
                step: StepDirection::Up,
            },
        );
        press_control(
            &mut app,
            InspectorControl::JointCompliance {
                property: JointComplianceProperty::Point,
                step: StepDirection::Up,
            },
        );
        press_control(
            &mut app,
            InspectorControl::JointDamping {
                kind: JointDampingKind::Linear,
                step: StepDirection::Up,
            },
        );

        let fixed = app.world().entity(joint).get::<FixedJoint>().unwrap();
        assert_eq!(fixed.local_anchor1(), Some(Vec3::new(0.5, 0.1, 0.0)));
        assert_eq!(fixed.point_compliance, 0.001);
        assert_eq!(
            app.world()
                .entity(joint)
                .get::<JointDamping>()
                .unwrap()
                .linear,
            0.3
        );

        press_control(&mut app, InspectorControl::ToggleJointEnabled);
        assert!(app.world().entity(joint).contains::<JointDisabled>());
        assert!(
            !app.world()
                .resource::<InspectorState>()
                .joint
                .as_ref()
                .unwrap()
                .enabled
        );
        press_control(&mut app, InspectorControl::ToggleJointEnabled);
        assert!(!app.world().entity(joint).contains::<JointDisabled>());
        assert!(
            app.world()
                .resource::<InspectorState>()
                .joint
                .as_ref()
                .unwrap()
                .enabled
        );

        press_control(&mut app, InspectorControl::DeleteJoint);
        assert!(app.world().get_entity(joint).is_err());
        assert_eq!(app.world().resource::<SelectionState>().entity, None);
        assert_eq!(app.world().resource::<InspectorState>().joint, None);
    }

    #[test]
    fn selected_body_inspector_reports_live_contact_partner_and_solver_data() {
        let mut app = inspector_app();
        let floor = spawn_body(&mut app, "Contact Floor", RigidBody::Static, Vec3::ZERO);
        let body = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(0.5, 0.5, 0.5),
                LinearVelocity::default(),
                Position(Vec3::new(0.0, 0.4, 0.0)),
                Transform::from_xyz(0.0, 0.4, 0.0),
                Name::new("Contact Body"),
            ))
            .id();
        app.update();
        for _ in 0..10 {
            app.update();
        }
        select_entity(&mut app, body);

        let initial_contact = {
            let object = app
                .world()
                .resource::<InspectorState>()
                .object
                .as_ref()
                .expect("selected body snapshot");
            let contact = object
                .contacts
                .iter()
                .find(|contact| contact.other_entity == floor)
                .expect("selected body contact");
            assert_eq!(contact.other_name, "Contact Floor");
            assert!(contact.position.is_finite());
            assert!((contact.normal.length() - 1.0).abs() < 0.01);
            assert!(contact.relative_speed.is_finite());
            assert!(contact.impulse.is_finite());
            contact.clone()
        };
        assert!(inspector_text(&mut app).contains("CONTACTS (LIVE)"));
        assert!(inspector_text(&mut app).contains("Partner: Contact Floor"));
        app.world_mut()
            .entity_mut(body)
            .insert(LinearVelocity(Vec3::new(0.0, -1.0, 0.0)));
        app.update();
        let updated_contact = app
            .world()
            .resource::<InspectorState>()
            .object
            .as_ref()
            .expect("updated body snapshot")
            .contacts
            .iter()
            .find(|contact| contact.other_entity == floor)
            .expect("updated selected body contact");
        assert!(
            updated_contact != &initial_contact
                || updated_contact.relative_speed != initial_contact.relative_speed,
            "contact data should be refreshed while the pair remains active"
        );
    }

    #[test]
    fn clicking_empty_space_clears_selection_and_hides_the_inspector() {
        let mut app = inspector_app();
        spawn_body(
            &mut app,
            "Inspectable Cube",
            RigidBody::Dynamic,
            Vec3::new(0.0, 0.0, -5.0),
        );
        app.update();

        point_cursor_at(&mut app, Vec3::new(0.0, 0.0, -5.0));
        click_left(&mut app);
        assert!(app.world().resource::<InspectorState>().object.is_some());

        app.world_mut()
            .resource_mut::<CursorRay>()
            .set(Vec3::ZERO, Dir3::X);
        app.update();
        click_left(&mut app);

        assert_eq!(
            app.world().resource::<SelectionState>().entity,
            None,
            "empty click should clear the selected entity"
        );
        assert_eq!(app.world().resource::<InspectorState>().object, None);
        assert!(inspector_text(&mut app).is_empty());
    }
}
