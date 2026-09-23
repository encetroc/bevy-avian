use bevy::prelude::*;

use crate::object_inspector::SelectionState;

/// Capacity and current fill amount for a sandbox liquid container.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct LiquidContainer {
    capacity: f32,
    amount: f32,
}

impl LiquidContainer {
    pub fn new(capacity: f32, amount: f32) -> Self {
        assert!(capacity.is_finite() && capacity > 0.0);
        assert!(amount.is_finite());
        Self {
            capacity,
            amount: amount.clamp(0.0, capacity),
        }
    }

    pub const fn capacity(self) -> f32 {
        self.capacity
    }

    pub const fn amount(self) -> f32 {
        self.amount
    }

    pub fn set_amount(&mut self, amount: f32) {
        if amount.is_finite() {
            self.amount = amount.clamp(0.0, self.capacity);
        }
    }
}

#[derive(Component)]
struct LiquidContainerReadout;

/// Adds amount controls and a visible readout for the selected liquid container.
pub struct LiquidContainersPlugin;

impl Plugin for LiquidContainersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_readout)
            .add_systems(Update, (adjust_selected_amount, update_readout).chain());
    }
}

fn spawn_readout(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: px(16.0),
                padding: UiRect::all(px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.05, 0.08, 0.94)),
            LiquidContainerReadout,
            Name::new("Liquid container readout"),
        ))
        .with_child((
            Text::new("Select a liquid container; use = / - to change its amount."),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.75, 0.9, 1.0)),
        ));
}

fn adjust_selected_amount(
    keyboard: Res<ButtonInput<KeyCode>>,
    selection: Res<SelectionState>,
    mut containers: Query<&mut LiquidContainer>,
) {
    let Some(entity) = selection.entity else {
        return;
    };
    let Ok(mut container) = containers.get_mut(entity) else {
        return;
    };
    let step = container.capacity() * 0.1;
    let amount = container.amount();
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        container.set_amount(amount + step);
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        container.set_amount(amount - step);
    }
}

fn update_readout(
    selection: Res<SelectionState>,
    containers: Query<(&LiquidContainer, Option<&Name>)>,
    mut readouts: Query<&mut Text, With<LiquidContainerReadout>>,
) {
    let text = selection
        .entity
        .and_then(|entity| containers.get(entity).ok())
        .map_or_else(
            || "Select a liquid container; use = / - to change its amount.".to_owned(),
            |(container, name)| {
                format!(
                    "{} — {:.0} / {:.0} L",
                    name.map_or("Liquid container", Name::as_str),
                    container.amount(),
                    container.capacity()
                )
            },
        );
    for mut readout in &mut readouts {
        *readout = Text::new(text.clone());
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use avian3d::prelude::{Collider, PhysicsPlugins, Position, RigidBody};
    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    #[test]
    fn full_partial_and_empty_container_amounts_are_valid_and_clamped() {
        let full = LiquidContainer::new(100.0, 100.0);
        let partial = LiquidContainer::new(100.0, 37.5);
        let empty = LiquidContainer::new(100.0, 0.0);
        assert_eq!(full.amount(), full.capacity());
        assert_eq!(partial.amount(), 37.5);
        assert_eq!(empty.amount(), 0.0);

        let mut partial = partial;
        partial.set_amount(150.0);
        assert_eq!(partial.amount(), 100.0);
        partial.set_amount(-4.0);
        assert_eq!(partial.amount(), 0.0);
    }

    #[test]
    fn amount_changes_while_a_dynamic_container_continues_simulating() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        let container = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cylinder(0.5, 1.0),
                LiquidContainer::new(100.0, 50.0),
                Transform::from_xyz(0.0, 5.0, 0.0),
            ))
            .id();
        app.finish();
        for _ in 0..12 {
            app.update();
        }

        let before = app.world().entity(container).get::<Position>().unwrap().y;
        app.world_mut()
            .entity_mut(container)
            .get_mut::<LiquidContainer>()
            .unwrap()
            .set_amount(25.0);
        for _ in 0..12 {
            app.update();
        }
        let body = app.world().entity(container);
        assert_eq!(body.get::<LiquidContainer>().unwrap().amount(), 25.0);
        assert_eq!(body.get::<RigidBody>(), Some(&RigidBody::Dynamic));
        assert!(body.get::<Position>().unwrap().y < before);
    }
}
