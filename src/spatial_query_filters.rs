use avian3d::prelude::*;
use bevy::prelude::*;

use crate::collision_layers::SandboxLayer;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const FILTER_ACTIVE: Color = Color::srgb(0.25, 0.85, 0.5);

/// The collision-layer mask shared by the sandbox's spatial-query stations.
#[derive(Resource, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpatialQueryFilterState {
    pub mask: LayerMask,
}

impl Default for SpatialQueryFilterState {
    fn default() -> Self {
        Self {
            mask: LayerMask::ALL,
        }
    }
}

impl SpatialQueryFilterState {
    pub fn query_filter(self) -> SpatialQueryFilter {
        SpatialQueryFilter::from_mask(self.mask)
    }

    pub fn includes(self, layer: SandboxLayer) -> bool {
        self.mask.0 & layer.to_bits() != 0
    }

    pub fn toggle(&mut self, layer: SandboxLayer) {
        self.mask.0 ^= layer.to_bits();
    }
}

#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
enum SpatialQueryFilterAction {
    Toggle(SandboxLayer),
    IncludeAll,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct SpatialQueryFilterControl(SandboxLayer);

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct SpatialQueryFilterLabel(SandboxLayer);

#[derive(Component)]
struct SpatialQueryFilterStatus;

#[derive(Component)]
struct SpatialQueryFilterReset;

#[derive(Component)]
struct SpatialQueryFilterPanel;

/// Owns the UI and state used to include or exclude collision layers from
/// raycasts and shape casts.
pub struct SpatialQueryFiltersPlugin;

#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SpatialQueryFilterSet {
    ApplyControls,
}

impl Plugin for SpatialQueryFiltersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialQueryFilterState>()
            .add_message::<SpatialQueryFilterAction>()
            .configure_sets(Update, SpatialQueryFilterSet::ApplyControls)
            .add_systems(Startup, spawn_spatial_query_filter_ui)
            .add_systems(
                Update,
                (
                    queue_spatial_query_filter_actions,
                    apply_spatial_query_filter_actions,
                    update_spatial_query_filter_ui,
                )
                    .chain()
                    .in_set(SpatialQueryFilterSet::ApplyControls),
            );
    }
}

fn spawn_spatial_query_filter_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(235.0),
                width: px(270.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(3.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            SpatialQueryFilterPanel,
            Name::new("Spatial Query Layer Filters"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SPATIAL QUERY FILTERS"),
                TextFont::from_font_size(17.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Raycasts and shape casts include checked layers."),
                TextFont::from_font_size(11.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Included: ALL LAYERS"),
                TextFont::from_font_size(12.0),
                TextColor(FILTER_ACTIVE),
                SpatialQueryFilterStatus,
            ));

            for layer in SandboxLayer::DEMONSTRATED {
                parent
                    .spawn((
                        Button,
                        SpatialQueryFilterControl(layer),
                        Node {
                            width: px(240.0),
                            height: px(21.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            padding: UiRect::horizontal(px(8.0)),
                            ..default()
                        },
                        BackgroundColor(CONTROL_BACKGROUND),
                        Name::new(format!("Toggle {} spatial query layer", layer.label())),
                    ))
                    .with_child((
                        Text::new(format!("[x] {}", layer.label())),
                        TextFont::from_font_size(11.0),
                        TextColor(PANEL_TEXT),
                        SpatialQueryFilterLabel(layer),
                    ));
            }

            parent
                .spawn((
                    Button,
                    SpatialQueryFilterReset,
                    Node {
                        width: px(240.0),
                        height: px(21.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Include all spatial query layers"),
                ))
                .with_child((
                    Text::new("include ALL layers"),
                    TextFont::from_font_size(11.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn queue_spatial_query_filter_actions(
    controls: Query<(&Interaction, &SpatialQueryFilterControl), Changed<Interaction>>,
    reset_button: Query<&Interaction, (With<SpatialQueryFilterReset>, Changed<Interaction>)>,
    mut actions: MessageWriter<SpatialQueryFilterAction>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            actions.write(SpatialQueryFilterAction::Toggle(control.0));
        }
    }
    if reset_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        actions.write(SpatialQueryFilterAction::IncludeAll);
    }
}

fn apply_spatial_query_filter_actions(
    mut actions: MessageReader<SpatialQueryFilterAction>,
    mut state: ResMut<SpatialQueryFilterState>,
) {
    for action in actions.read() {
        match action {
            SpatialQueryFilterAction::Toggle(layer) => state.toggle(*layer),
            SpatialQueryFilterAction::IncludeAll => state.mask = LayerMask::ALL,
        }
    }
}

fn update_spatial_query_filter_ui(
    state: Res<SpatialQueryFilterState>,
    mut status: Query<
        &mut Text,
        (
            With<SpatialQueryFilterStatus>,
            Without<SpatialQueryFilterLabel>,
        ),
    >,
    mut labels: Query<(&SpatialQueryFilterLabel, &mut Text), Without<SpatialQueryFilterStatus>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<SpatialQueryFilterControl>>,
) {
    if let Ok(mut status) = status.single_mut() {
        let included: Vec<_> = SandboxLayer::DEMONSTRATED
            .into_iter()
            .filter(|layer| state.includes(*layer))
            .map(SandboxLayer::label)
            .collect();
        status.0 = if included.is_empty() {
            "Included: NONE (no hits)".to_owned()
        } else {
            format!("Included: {}", included.join(" · "))
        };
    }

    for (label, mut text) in &mut labels {
        text.0 = format!(
            "[{}] {}",
            if state.includes(label.0) { 'x' } else { ' ' },
            label.0.label()
        );
    }

    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::collision_layers::layers_for;

    use super::*;

    #[test]
    fn toggling_a_layer_changes_the_query_mask() {
        let mut state = SpatialQueryFilterState::default();
        assert!(state.includes(SandboxLayer::World));

        state.toggle(SandboxLayer::World);

        assert!(!state.includes(SandboxLayer::World));
        assert!(
            !state
                .query_filter()
                .test(Entity::PLACEHOLDER, layers_for(SandboxLayer::World))
        );
    }

    #[test]
    fn filter_state_can_include_all_layers_again() {
        let mut state = SpatialQueryFilterState {
            mask: LayerMask::NONE,
        };
        state.mask = LayerMask::ALL;

        assert!(state.includes(SandboxLayer::World));
        assert!(state.includes(SandboxLayer::Objects));
    }
}
