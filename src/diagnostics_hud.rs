use avian3d::{
    dynamics::solver::SolverDiagnostics,
    prelude::{Collider, Physics, PhysicsDiagnosticsPlugin, RigidBody, Sleeping, SubstepCount},
    schedule::PhysicsTime,
};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::system::SystemParam,
    prelude::*,
};

use avian3d::diagnostics::PhysicsTotalDiagnosticsPlugin;

/// Shows live application and physics workload metrics in a compact overlay.
pub(crate) struct DiagnosticsHudPlugin;

impl Plugin for DiagnosticsHudPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "dev-physics-validation")]
        info!("Avian physics validation enabled (dev-physics-validation)");

        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            PhysicsDiagnosticsPlugin,
            PhysicsTotalDiagnosticsPlugin,
        ))
        .init_resource::<DiagnosticsSnapshot>()
        .add_systems(Startup, spawn_diagnostics_hud)
        .add_systems(Update, update_diagnostics_hud);
    }
}

#[derive(Component)]
struct DiagnosticsHudText;

#[derive(Resource, Debug, Default, PartialEq, Eq, Clone, Copy)]
struct DiagnosticsSnapshot {
    entities: usize,
    dynamic_bodies: usize,
    sleeping_bodies: usize,
    colliders: usize,
}

fn spawn_diagnostics_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.85)),
        Text::new("DIAGNOSTICS\nFPS: --  |  FRAME: -- ms\nBODIES: -- dynamic (-- sleeping)\nCOLLIDERS: --  |  CONTACTS: --\nENTITIES: --\nPHYSICS: -- ms last step  |  FIXED: -- ms"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::WHITE),
        DiagnosticsHudText,
    ));
}

#[derive(SystemParam)]
struct HudFrameMetrics<'w> {
    diagnostics: Res<'w, DiagnosticsStore>,
    physics_time: Res<'w, Time<Physics>>,
    fixed_time: Res<'w, Time<Fixed>>,
    substeps: Res<'w, SubstepCount>,
    physics_step: Res<'w, avian3d::diagnostics::PhysicsTotalDiagnostics>,
    solver: Res<'w, SolverDiagnostics>,
}

fn update_diagnostics_hud(
    metrics: HudFrameMetrics,
    mut snapshot: ResMut<DiagnosticsSnapshot>,
    entities: Query<(Option<&RigidBody>, Has<Sleeping>, Has<Collider>)>,
    mut hud: Query<&mut Text, With<DiagnosticsHudText>>,
) {
    let mut counts = DiagnosticsSnapshot {
        entities: entities.iter().count(),
        ..default()
    };
    for (body, sleeping, collider) in &entities {
        if body == Some(&RigidBody::Dynamic) {
            counts.dynamic_bodies += 1;
            counts.sleeping_bodies += usize::from(sleeping);
        }
        counts.colliders += usize::from(collider);
    }
    *snapshot = counts;

    let fps = metrics
        .diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diagnostic| diagnostic.smoothed())
        .map_or_else(|| "--".to_owned(), |value| format!("{value:.0}"));
    let frame_ms = metrics
        .diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|diagnostic| diagnostic.smoothed())
        .map_or_else(|| "--".to_owned(), |value| format!("{value:.2}"));
    let state = if metrics.physics_time.is_paused() {
        "PAUSED"
    } else {
        "RUNNING"
    };
    let speed = metrics.physics_time.relative_speed();
    let physics_ms = metrics.physics_step.step_time.as_secs_f64() * 1_000.0;
    let fixed_ms = metrics.fixed_time.timestep().as_secs_f64() * 1_000.0;

    for mut text in &mut hud {
        let validation = if cfg!(feature = "dev-physics-validation") {
            "ENABLED"
        } else {
            "disabled"
        };
        **text = format!(
            "DIAGNOSTICS\nPHYSICS VALIDATION: {validation}\nFPS: {fps}  |  FRAME: {frame_ms} ms\nBODIES: {} dynamic ({} sleeping)\nCOLLIDERS: {}  |  CONTACTS: {} last step\nENTITIES: {}\nPHYSICS: {physics_ms:.2} ms last step  |  FIXED: {fixed_ms:.2} ms  |  {state} {speed}x  |  SUBSTEPS: {}",
            counts.dynamic_bodies,
            counts.sleeping_bodies,
            counts.colliders,
            metrics.solver.contact_constraint_count,
            counts.entities,
            metrics.substeps.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use avian3d::prelude::{PhysicsPlugins, Position};
    use bevy::{mesh::MeshPlugin, time::TimeUpdateStrategy};

    use super::*;

    fn diagnostics_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            PhysicsPlugins::default(),
            DiagnosticsHudPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        app.finish();
        app
    }

    fn hud_text(app: &mut App) -> String {
        let world = app.world_mut();
        let mut query = world.query_filtered::<&Text, With<DiagnosticsHudText>>();
        query.single(world).expect("one diagnostics HUD").0.clone()
    }

    #[test]
    fn metrics_update_when_dynamic_colliders_are_spawned() {
        let mut app = diagnostics_app();
        app.update();
        let before = *app.world().resource::<DiagnosticsSnapshot>();

        app.world_mut().spawn((
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            Sleeping,
            Name::new("HUD metric test body"),
        ));
        app.update();

        let after = app.world().resource::<DiagnosticsSnapshot>();
        assert_eq!(after.dynamic_bodies, before.dynamic_bodies + 1);
        assert_eq!(after.sleeping_bodies, before.sleeping_bodies + 1);
        assert_eq!(after.colliders, before.colliders + 1);
        assert_eq!(after.entities, before.entities + 1);

        let text = hud_text(&mut app);
        assert!(text.contains("FPS:"));
        assert!(text.contains("CONTACTS:"));
        assert!(text.contains("PHYSICS:"));
        assert_eq!(
            text.contains("PHYSICS VALIDATION: ENABLED"),
            cfg!(feature = "dev-physics-validation")
        );
    }

    #[test]
    fn hud_remains_responsive_with_a_stress_sized_body_count() {
        let mut app = diagnostics_app();
        app.update();
        let baseline = *app.world().resource::<DiagnosticsSnapshot>();
        for index in 0..512 {
            app.world_mut().spawn((
                RigidBody::Dynamic,
                Collider::sphere(0.25),
                Name::new(format!("HUD stress body {index}")),
            ));
        }
        app.update();

        let snapshot = app.world().resource::<DiagnosticsSnapshot>();
        assert_eq!(snapshot.dynamic_bodies, baseline.dynamic_bodies + 512);
        assert_eq!(snapshot.colliders, baseline.colliders + 512);
        assert_eq!(snapshot.entities, baseline.entities + 512);
        assert!(hud_text(&mut app).contains("BODIES: 512 dynamic"));
    }

    #[test]
    fn representative_physics_scene_keeps_validation_and_diagnostics_available() {
        let mut app = diagnostics_app();
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(10.0, 1.0, 10.0),
            Position(Vec3::new(0.0, -0.5, 0.0)),
            Transform::from_xyz(0.0, -0.5, 0.0),
            Name::new("validation smoke-test floor"),
        ));
        let body = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::sphere(0.5),
                Position(Vec3::Y * 2.0),
                Transform::from_translation(Vec3::Y * 2.0),
                Name::new("validation smoke-test body"),
            ))
            .id();

        for _ in 0..30 {
            app.update();
        }

        let position = app.world().get::<Position>(body).unwrap().0;
        assert!(position.is_finite());
        let text = hud_text(&mut app);
        assert!(text.contains("PHYSICS VALIDATION:"));
        assert!(text.contains("CONTACTS:"));
        assert!(text.contains("PHYSICS:"));
    }

    #[test]
    fn pause_keeps_live_counts_and_last_step_timing_labeled() {
        let mut app = diagnostics_app();
        app.update();
        app.world_mut().resource_mut::<Time<Physics>>().pause();
        app.update();

        let text = hud_text(&mut app);
        assert!(text.contains("PAUSED"));
        assert!(text.contains("last step"));
        assert!(text.contains("FIXED:"));
    }
}
