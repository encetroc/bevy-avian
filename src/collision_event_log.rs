use std::collections::{HashMap, HashSet};

use avian3d::prelude::*;
use bevy::prelude::*;

const PANEL_BACKGROUND: Color = Color::srgba(0.035, 0.05, 0.08, 0.94);
const PANEL_TEXT: Color = Color::srgb(0.87, 0.92, 0.98);
const CONTROL_BACKGROUND: Color = Color::srgb(0.12, 0.18, 0.28);
const CONTROL_PRESSED: Color = Color::srgb(0.2, 0.38, 0.55);
const MAX_LOG_ENTRIES: usize = 16;

/// The three collision states shown by the event log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionEventType {
    Enter,
    Contact,
    Exit,
}

impl CollisionEventType {
    const fn label(self) -> &'static str {
        match self {
            Self::Enter => "ENTER",
            Self::Contact => "CONTACT",
            Self::Exit => "EXIT",
        }
    }
}

/// One collision event and the two entities that participated in it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollisionLogEntry {
    pub event_type: CollisionEventType,
    pub entities: [Entity; 2],
}

/// The rolling collision history displayed by the sandbox UI.
#[derive(Resource, Debug, Default)]
pub struct CollisionEventLog {
    pub entries: Vec<CollisionLogEntry>,
    entered_pairs: HashSet<CollisionPairKey>,
    contacted_pairs: HashMap<CollisionPairKey, [Entity; 2]>,
}

impl CollisionEventLog {
    fn record(&mut self, event_type: CollisionEventType, first: Entity, second: Entity) {
        if self.entries.len() == MAX_LOG_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(CollisionLogEntry {
            event_type,
            entities: [first, second],
        });
    }

    /// Removes displayed history while preserving collision tracking state.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CollisionPairKey(u64, u64);

fn pair_key(first: Entity, second: Entity) -> CollisionPairKey {
    let [first, second] = [first, second];
    if first.to_bits() <= second.to_bits() {
        CollisionPairKey(first.to_bits(), second.to_bits())
    } else {
        CollisionPairKey(second.to_bits(), first.to_bits())
    }
}

/// Headless-friendly actions exposed by the event log controls.
#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionEventLogAction {
    Clear,
}

/// Owns collision event collection and its on-screen history panel.
pub struct CollisionEventLogPlugin;

impl Plugin for CollisionEventLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionEventLog>()
            .add_message::<CollisionEventLogAction>()
            .add_systems(PostStartup, enable_collision_events)
            .add_systems(
                Update,
                (
                    enable_collision_events,
                    record_collision_events,
                    queue_collision_log_actions,
                    apply_collision_log_actions,
                    update_collision_event_log_ui,
                )
                    .chain(),
            )
            .add_systems(Startup, spawn_collision_event_log_ui);
    }
}

/// Enables Avian's collision messages for all current and newly spawned colliders.
fn enable_collision_events(
    mut commands: Commands,
    colliders: Query<Entity, (With<Collider>, Without<CollisionEventsEnabled>)>,
) {
    for entity in &colliders {
        commands.entity(entity).insert(CollisionEventsEnabled);
    }
}

fn event_participants(
    collider1: Entity,
    collider2: Entity,
    body1: Option<Entity>,
    body2: Option<Entity>,
) -> [Entity; 2] {
    [body1.unwrap_or(collider1), body2.unwrap_or(collider2)]
}

fn record_collision_events(
    mut log: ResMut<CollisionEventLog>,
    mut starts: MessageReader<CollisionStart>,
    mut ends: MessageReader<CollisionEnd>,
    contact_graph: Res<ContactGraph>,
    sensors: Query<Entity, With<Sensor>>,
) {
    for event in starts.read() {
        let [first, second] =
            event_participants(event.collider1, event.collider2, event.body1, event.body2);
        if log.entered_pairs.insert(pair_key(first, second)) {
            log.record(CollisionEventType::Enter, first, second);
        }
    }

    for event in ends.read() {
        let [first, second] =
            event_participants(event.collider1, event.collider2, event.body1, event.body2);
        let key = pair_key(first, second);
        log.contacted_pairs.remove(&key);
        log.entered_pairs.remove(&key);
        log.record(CollisionEventType::Exit, first, second);
    }

    let touching_pairs: Vec<([Entity; 2], CollisionPairKey)> = contact_graph
        .iter_active_touching()
        .chain(contact_graph.iter_sleeping_touching())
        // Sensor intersections produce ENTER/EXIT messages but never physical
        // contact events. Keep them out of the contact-history bookkeeping.
        .filter(|pair| !sensors.contains(pair.collider1) && !sensors.contains(pair.collider2))
        .map(|pair| {
            let participants = [
                pair.body1.unwrap_or(pair.collider1),
                pair.body2.unwrap_or(pair.collider2),
            ];
            (participants, pair_key(participants[0], participants[1]))
        })
        .collect();

    for (participants, key) in &touching_pairs {
        if log.entered_pairs.insert(*key) {
            log.record(CollisionEventType::Enter, participants[0], participants[1]);
        }
        if !log.contacted_pairs.contains_key(key) {
            log.record(
                CollisionEventType::Contact,
                participants[0],
                participants[1],
            );
            log.contacted_pairs.insert(*key, *participants);
        }
    }

    let touching_keys: HashSet<_> = touching_pairs.iter().map(|(_, key)| *key).collect();
    let ended_pairs: Vec<_> = log
        .contacted_pairs
        .iter()
        .filter_map(|(key, participants)| {
            (!touching_keys.contains(key)).then_some((*key, *participants))
        })
        .collect();
    for (key, [first, second]) in ended_pairs {
        log.contacted_pairs.remove(&key);
        log.entered_pairs.remove(&key);
        log.record(CollisionEventType::Exit, first, second);
    }
}

#[derive(Component)]
struct CollisionEventLogPanel;

#[derive(Component)]
struct CollisionEventLogText;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum CollisionEventLogControl {
    Clear,
}

fn queue_collision_log_actions(
    controls: Query<(&Interaction, &CollisionEventLogControl), Changed<Interaction>>,
    mut actions: MessageWriter<CollisionEventLogAction>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            match control {
                CollisionEventLogControl::Clear => {
                    actions.write(CollisionEventLogAction::Clear);
                }
            }
        }
    }
}

fn apply_collision_log_actions(
    mut actions: MessageReader<CollisionEventLogAction>,
    mut log: ResMut<CollisionEventLog>,
) {
    for action in actions.read() {
        match action {
            CollisionEventLogAction::Clear => log.clear(),
        }
    }
}

fn spawn_collision_event_log_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                bottom: px(16.0),
                width: px(360.0),
                height: px(250.0),
                padding: UiRect::all(px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.0),
                ..default()
            },
            BackgroundColor(PANEL_BACKGROUND),
            CollisionEventLogPanel,
            Name::new("Collision Event Log"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PHYSICS EVENTS"),
                TextFont::from_font_size(18.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("Enter, contact, and exit events"),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
            ));
            parent.spawn((
                Text::new("No collision events yet."),
                TextFont::from_font_size(12.0),
                TextColor(PANEL_TEXT),
                CollisionEventLogText,
                Node {
                    flex_grow: 1.0,
                    overflow: Overflow::clip(),
                    margin: UiRect::top(px(4.0)),
                    ..default()
                },
            ));
            parent
                .spawn((
                    Button,
                    CollisionEventLogControl::Clear,
                    Node {
                        width: px(72.0),
                        height: px(22.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(CONTROL_BACKGROUND),
                    Name::new("Clear collision event log"),
                ))
                .with_child((
                    Text::new("clear"),
                    TextFont::from_font_size(10.0),
                    TextColor(PANEL_TEXT),
                ));
        });
}

fn update_collision_event_log_ui(
    log: Res<CollisionEventLog>,
    names: Query<&Name>,
    mut text: Query<&mut Text, With<CollisionEventLogText>>,
    mut buttons: Query<(&Interaction, &mut BackgroundColor), With<CollisionEventLogControl>>,
) {
    if let Ok(mut text) = text.single_mut() {
        text.0 = if log.entries.is_empty() {
            "No collision events yet.".to_owned()
        } else {
            log.entries
                .iter()
                .map(|entry| {
                    format!(
                        "{} → {} {}\n",
                        entity_name(&names, entry.entities[0]),
                        entity_name(&names, entry.entities[1]),
                        entry.event_type.label()
                    )
                })
                .collect()
        };
    }

    for (interaction, mut color) in &mut buttons {
        *color = match interaction {
            Interaction::Pressed | Interaction::Hovered => CONTROL_PRESSED.into(),
            Interaction::None => CONTROL_BACKGROUND.into(),
        };
    }
}

fn entity_name(names: &Query<&Name>, entity: Entity) -> String {
    names
        .get(entity)
        .map(|name| name.as_str().to_owned())
        .unwrap_or_else(|_| format!("Entity {}", entity.to_bits()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    const PHYSICS_STEP: f32 = 1.0 / 60.0;

    #[derive(Component)]
    struct TestFloor;

    #[derive(Component)]
    struct TestBody;

    fn event_log_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            CollisionEventLogPlugin,
        ))
        .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PHYSICS_STEP,
        )));
        app.add_systems(Startup, spawn_test_scene);
        app.finish();
        app
    }

    fn spawn_test_scene(mut commands: Commands) {
        commands.spawn((
            TestFloor,
            RigidBody::Static,
            Collider::cuboid(10.0, 0.5, 10.0),
            Transform::from_xyz(0.0, -0.25, 0.0),
            Name::new("Test Floor"),
        ));
        commands.spawn((
            TestBody,
            RigidBody::Dynamic,
            Collider::cuboid(0.5, 0.5, 0.5),
            Transform::from_xyz(0.0, 3.0, 0.0),
            Name::new("Test Cube"),
        ));
    }

    fn run_steps(app: &mut App, steps: usize) {
        for _ in 0..steps {
            app.update();
        }
    }

    fn body_entity(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut bodies = world.query_filtered::<Entity, With<TestBody>>();
        bodies.iter(world).next().expect("test body")
    }

    #[test]
    fn falling_body_records_named_enter_and_contact_events() {
        let mut app = event_log_app();
        app.update();
        run_steps(&mut app, 180);

        let entries = app.world().resource::<CollisionEventLog>().entries.clone();
        assert!(entries.iter().any(|entry| {
            entry.event_type == CollisionEventType::Enter
                && entry.entities.iter().any(|entity| {
                    app.world()
                        .entity(*entity)
                        .get::<Name>()
                        .is_some_and(|name| name.as_str() == "Test Cube")
                })
        }));
        assert!(
            entries
                .iter()
                .any(|entry| entry.event_type == CollisionEventType::Contact)
        );

        let body = body_entity(&mut app);
        app.world_mut().entity_mut(body).insert((
            Position(Vec3::new(0.0, 4.0, 0.0)),
            Transform::from_xyz(0.0, 4.0, 0.0),
            LinearVelocity::default(),
        ));
        run_steps(&mut app, 180);

        let entries = app.world().resource::<CollisionEventLog>().entries.clone();
        assert!(
            entries
                .iter()
                .any(|entry| entry.event_type == CollisionEventType::Exit),
            "separating the body should record an exit event: {entries:?}"
        );
    }

    #[test]
    fn clearing_history_does_not_stop_new_collision_events() {
        let mut app = event_log_app();
        app.update();
        run_steps(&mut app, 180);
        assert!(
            !app.world()
                .resource::<CollisionEventLog>()
                .entries
                .is_empty()
        );

        let clear_button = {
            let world = app.world_mut();
            let mut controls = world.query_filtered::<Entity, With<CollisionEventLogControl>>();
            controls.iter(world).next().expect("clear button")
        };
        app.world_mut()
            .entity_mut(clear_button)
            .get_mut::<Interaction>()
            .expect("button interaction")
            .clone_from(&Interaction::Pressed);
        app.update();
        assert!(
            app.world()
                .resource::<CollisionEventLog>()
                .entries
                .is_empty()
        );

        let body = body_entity(&mut app);
        app.world_mut().entity_mut(body).insert((
            Position(Vec3::new(0.0, 4.0, 0.0)),
            Transform::from_xyz(0.0, 4.0, 0.0),
            LinearVelocity::default(),
        ));
        run_steps(&mut app, 180);

        assert!(
            !app.world()
                .resource::<CollisionEventLog>()
                .entries
                .is_empty()
        );
        assert!(
            app.world()
                .resource::<CollisionEventLog>()
                .entries
                .iter()
                .any(|entry| entry.event_type == CollisionEventType::Enter)
        );
    }
}
