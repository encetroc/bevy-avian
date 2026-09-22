use avian3d::{parry::shape::TypedShape, prelude::*};
use bevy::prelude::*;

use crate::cursor_hover::{HoverState, update_hover_reachability};

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);

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
    mut selection: ResMut<SelectionState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    selection.set_if_neq(SelectionState {
        entity: hover.object.as_ref().map(|object| object.entity),
    });
}

/// Copies the selected entity's live Avian components into a stable, UI-ready
/// snapshot. The snapshot is refreshed every frame so velocity and sleeping
/// information remains useful while a body is moving or being grabbed.
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

fn spawn_inspector_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16.0),
                right: px(16.0),
                width: px(360.0),
                max_height: percent(92.0),
                padding: UiRect::all(px(16.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            InspectorPanel,
            Name::new("Selected Object Inspector"),
        ))
        .with_child((
            Text::new(""),
            TextFont::from_font_size(16.0),
            TextColor(PANEL_TEXT),
            InspectorText,
        ));
}

fn update_inspector_ui(
    inspector: Res<InspectorState>,
    mut panel: Query<&mut Node, With<InspectorPanel>>,
    mut text: Query<&mut Text, With<InspectorText>>,
) {
    let Ok(mut panel) = panel.single_mut() else {
        return;
    };
    let Ok(mut text) = text.single_mut() else {
        return;
    };

    let Some(object) = inspector.object.as_ref() else {
        panel.display = Display::None;
        text.0.clear();
        return;
    };

    panel.display = Display::Flex;
    text.0 = format_inspector_text(object);
}

fn format_inspector_text(object: &InspectorSnapshot) -> String {
    let mass = object
        .mass
        .map_or_else(|| "infinite/default".to_owned(), |mass| format_float(mass));
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
