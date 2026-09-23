use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::object_grabbing::GrabState;

const PANEL_TEXT: Color = Color::srgb(0.9, 0.94, 1.0);
const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);

/// The latest entity selected through Bevy's rendered-mesh picking backend.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct MeshPickSelection {
    pub entity: Option<Entity>,
    pub name: Option<String>,
}

#[derive(Component)]
struct PickingComparisonPanel;

/// Compares Bevy mesh picking for selection with Avian's physical spring grab.
pub struct PickingComparisonPlugin;

impl Plugin for PickingComparisonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeshPickSelection>()
            .add_observer(record_mesh_pick)
            .add_systems(Startup, spawn_comparison_panel)
            .add_systems(Update, update_comparison_panel);
    }
}

/// Records rendered-mesh clicks on physics bodies. This does not start or
/// control a spring grab; that remains owned by `ObjectGrabbingPlugin`.
fn record_mesh_pick(
    click: On<Pointer<Click>>,
    selectable: Query<(), (With<Collider>, With<RigidBody>)>,
    names: Query<&Name>,
    mut selection: ResMut<MeshPickSelection>,
) {
    let entity = click.entity;
    if !selectable.contains(entity) {
        return;
    }

    let name = names
        .get(entity)
        .map(|name| name.as_str().to_owned())
        .unwrap_or_else(|_| format!("Physics Entity {entity:?}"));
    selection.set_if_neq(MeshPickSelection {
        entity: Some(entity),
        name: Some(name),
    });
}

fn spawn_comparison_panel(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12),
                right: px(12),
                padding: UiRect::all(px(10)),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            PickingComparisonPanel,
            Name::new("Picking and grabbing comparison panel"),
        ))
        .with_child((
            Text::new(""),
            TextFont::from_font_size(14.0),
            TextColor(PANEL_TEXT),
        ));
}

fn update_comparison_panel(
    selection: Res<MeshPickSelection>,
    grab: Res<GrabState>,
    names: Query<&Name>,
    mut panel: Query<&mut Text, With<PickingComparisonPanel>>,
) {
    let Ok(mut text) = panel.single_mut() else {
        return;
    };

    let selected = selection.name.as_deref().unwrap_or("none yet");
    let grabbed = grab
        .grab
        .map(|grab| {
            names
                .get(grab.entity)
                .map(|name| name.as_str().to_owned())
                .unwrap_or_else(|_| format!("Physics Entity {:?}", grab.entity))
        })
        .unwrap_or_else(|| "none (selection does not grab)".to_owned());

    text.0 = format!(
        "PICKING vs GRABBING\nClick a mesh to select. Hold G + left-drag to spring-grab.\nBevy mesh selection: {selected}\nAvian spring grab: {grabbed}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        app::App,
        camera::NormalizedRenderTarget,
        picking::{
            backend::HitData,
            pointer::{Location, PointerId},
        },
    };

    #[test]
    fn mesh_pick_observer_records_physics_entity_but_ignores_scenery() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(PickingComparisonPlugin);
        app.finish();

        let body = app
            .world_mut()
            .spawn((
                Collider::cuboid(1.0, 1.0, 1.0),
                RigidBody::Dynamic,
                Name::new("Comparison Body"),
            ))
            .id();
        let scenery = app.world_mut().spawn(Name::new("Unphysical scenery")).id();

        app.world_mut().trigger(Pointer::<Click>::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
            Click {
                button: PointerButton::Primary,
                hit: HitData {
                    camera: Entity::PLACEHOLDER,
                    depth: 0.0,
                    position: None,
                    normal: None,
                    extra: None,
                },
                duration: std::time::Duration::ZERO,
                count: 1,
            },
            body,
        ));
        assert_eq!(
            app.world().resource::<MeshPickSelection>(),
            &MeshPickSelection {
                entity: Some(body),
                name: Some("Comparison Body".to_owned()),
            }
        );

        app.world_mut().trigger(Pointer::<Click>::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
            Click {
                button: PointerButton::Primary,
                hit: HitData {
                    camera: Entity::PLACEHOLDER,
                    depth: 0.0,
                    position: None,
                    normal: None,
                    extra: None,
                },
                duration: std::time::Duration::ZERO,
                count: 1,
            },
            scenery,
        ));
        assert_eq!(
            app.world().resource::<MeshPickSelection>().entity,
            Some(body)
        );
    }
}
