use avian3d::{parry::shape::TypedShape, prelude::*};
use bevy::prelude::*;

use crate::cursor_hover::{HoverState, update_hover_reachability};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);

/// The physics body currently selected by the development inspector.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionState {
    /// The selected entity, or `None` when the last click hit empty space.
    pub entity: Option<Entity>,
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
    pub friction: Option<FrictionInfo>,
    pub restitution: Option<f32>,
    pub sleeping: bool,
    pub sleeping_disabled: bool,
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
                    apply_inspector_edits,
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
        }
    }
}

fn scalar_step(property: ScalarProperty) -> f32 {
    match property {
        ScalarProperty::Mass => 0.5,
        ScalarProperty::GravityScale => 0.1,
        ScalarProperty::LinearDamping | ScalarProperty::AngularDamping => 0.1,
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
        Option<&Friction>,
        Option<&Restitution>,
        Has<Sleeping>,
        Has<SleepingDisabled>,
    )>,
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
            friction: friction.map(|friction| FrictionInfo {
                dynamic: friction.dynamic_coefficient,
                static_coefficient: friction.static_coefficient,
            }),
            restitution: restitution.map(|restitution| restitution.coefficient),
            sleeping,
            sleeping_disabled,
        })
    });

    if next_object.is_none() && selected.is_some() {
        selection.entity = None;
    }
    inspector.set_if_neq(InspectorState {
        object: next_object,
    });
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
struct InspectorText;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorControl {
    Scalar {
        property: ScalarProperty,
        step: StepDirection,
    },
    Velocity {
        kind: VelocityKind,
        axis: Axis,
        step: StepDirection,
    },
    CycleBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarProperty {
    Mass,
    GravityScale,
    LinearDamping,
    AngularDamping,
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
    Velocity(VelocityKind, Axis),
    Body,
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
            spawn_body_row(parent);
            for axis in [Axis::X, Axis::Y, Axis::Z] {
                spawn_velocity_row(parent, VelocityKind::Linear, axis);
            }
            for axis in [Axis::X, Axis::Y, Axis::Z] {
                spawn_velocity_row(parent, VelocityKind::Angular, axis);
            }
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
    mut panel: Query<&mut Node, With<InspectorPanel>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<InspectorText>>,
        Query<(&InspectorValueText, &mut Text)>,
    )>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<InspectorControl>>,
) {
    let Ok(mut panel) = panel.single_mut() else {
        return;
    };
    let Some(object) = inspector.object.as_ref() else {
        panel.display = Display::None;
        if let Ok(mut summary_text) = texts.p0().single_mut() {
            summary_text.0.clear();
        }
        return;
    };

    panel.display = Display::Flex;
    if let Ok(mut summary_text) = texts.p0().single_mut() {
        summary_text.0 = format_inspector_text(object);
    }
    for (value, mut text) in &mut texts.p1() {
        text.0 = format_inspector_value(object, value.property);
    }
    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed => CONTROL_PRESSED.into(),
            Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn format_inspector_value(object: &InspectorSnapshot, property: InspectorValueProperty) -> String {
    match property {
        InspectorValueProperty::Scalar(property) => format_float(match property {
            ScalarProperty::Mass => object.mass.unwrap_or(0.0),
            ScalarProperty::GravityScale => object.gravity_scale,
            ScalarProperty::LinearDamping => object.linear_damping,
            ScalarProperty::AngularDamping => object.angular_damping,
        }),
        InspectorValueProperty::Velocity(kind, axis) => {
            let velocity = match kind {
                VelocityKind::Linear => object.linear_velocity,
                VelocityKind::Angular => object.angular_velocity,
            };
            format_float(axis_value(velocity, axis))
        }
        InspectorValueProperty::Body => format!("{:?}", object.body),
    }
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

    format!(
        "{}\nEntity: {:?}\n\nBODY\nType: {:?}\nMass: {}\nGravity scale: {}\nLinear damping: {}\nAngular damping: {}\n\nCOLLIDER\nShape: {}\nScale: {}\nFriction: {}\nRestitution: {}\n\nCURRENT STATE\nPosition: {}\nRotation: {}\nLinear velocity: {}\nAngular velocity: {}\nSleeping: {}\nSleeping disabled: {}",
        object.name,
        object.entity,
        object.body,
        mass,
        format_float(object.gravity_scale),
        format_float(object.linear_damping),
        format_float(object.angular_damping),
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
    )
}

fn format_float(value: f32) -> String {
    format!("{value:.2}")
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
