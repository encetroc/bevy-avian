//! Headless stress measurements for Avian builds with and without its `parallel` feature.
//!
//! Run the same test once per Cargo feature configuration and compare its emitted rows:
//! `cargo test parallel_physics_comparison -- --nocapture` and
//! `cargo test --no-default-features parallel_physics_comparison -- --nocapture`.

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use avian3d::{
        diagnostics::{PhysicsTotalDiagnostics, PhysicsTotalDiagnosticsPlugin},
        dynamics::solver::SolverDiagnostics,
        prelude::{Collider, PhysicsPlugins, Position, RigidBody},
    };
    use bevy::{
        diagnostic::FrameTimeDiagnosticsPlugin, mesh::MeshPlugin, prelude::*,
        time::TimeUpdateStrategy, transform::TransformPlugin,
    };

    const STEP: Duration = Duration::from_nanos(16_666_667);
    const MEASURED_STEPS: usize = 120;
    const BODY_COUNTS: [usize; 4] = [10, 100, 500, 1_000];

    #[derive(Debug)]
    struct Observation {
        bodies: usize,
        average_frame_ms: f64,
        average_fps: f64,
        last_physics_ms: f64,
        contacts: u32,
        finite_positions: usize,
        failed: bool,
    }

    fn measure(bodies: usize) -> Observation {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            TransformPlugin,
            PhysicsPlugins::default(),
            FrameTimeDiagnosticsPlugin::default(),
            PhysicsTotalDiagnosticsPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP));
        app.finish();

        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(100.0, 1.0, 100.0),
            Position(Vec3::new(0.0, -0.5, 0.0)),
            Transform::from_xyz(0.0, -0.5, 0.0),
        ));
        for index in 0..bodies {
            let x = (index % 25) as f32 * 0.35 - 4.2;
            let z = (index / 25) as f32 * 0.35;
            let y = 0.5 + (index / 625) as f32 * 0.45;
            app.world_mut().spawn((
                RigidBody::Dynamic,
                Collider::cuboid(0.4, 0.4, 0.4),
                Position(Vec3::new(x, y, z)),
                Transform::from_xyz(x, y, z),
                Name::new(format!("parallel comparison body {index}")),
            ));
        }

        let started = Instant::now();
        for _ in 0..MEASURED_STEPS {
            app.update();
        }
        let elapsed = started.elapsed();

        let world = app.world_mut();
        let mut query = world.query_filtered::<&Position, With<RigidBody>>();
        let positions: Vec<_> = query.iter(world).copied().collect();
        let dynamic_count = {
            let world = app.world_mut();
            let mut query = world.query::<&RigidBody>();
            query
                .iter(world)
                .filter(|body| **body == RigidBody::Dynamic)
                .count()
        };
        let contacts = app
            .world()
            .resource::<SolverDiagnostics>()
            .contact_constraint_count;
        let physics_ms = app
            .world()
            .resource::<PhysicsTotalDiagnostics>()
            .step_time
            .as_secs_f64()
            * 1_000.0;
        let avg_frame_ms = elapsed.as_secs_f64() * 1_000.0 / MEASURED_STEPS as f64;
        let avg_fps = MEASURED_STEPS as f64 / elapsed.as_secs_f64();
        let failed = dynamic_count != bodies
            || positions.len() != bodies + 1
            || positions.iter().any(|position| !position.0.is_finite());

        Observation {
            bodies,
            average_frame_ms: avg_frame_ms,
            average_fps: avg_fps,
            last_physics_ms: physics_ms,
            contacts,
            finite_positions: positions
                .iter()
                .filter(|position| position.0.is_finite())
                .count(),
            failed,
        }
    }

    #[test]
    fn identical_stress_counts_report_performance_contacts_and_stability() {
        let mode = if cfg!(feature = "parallel-physics") {
            "parallel"
        } else {
            "serial"
        };
        println!(
            "physics mode={mode}; steps={MEASURED_STEPS}; fixed_step_ms={:.3}",
            STEP.as_secs_f64() * 1_000.0
        );
        for count in BODY_COUNTS {
            let result = measure(count);
            println!(
                "bodies={} avg_fps={:.1} avg_frame_ms={:.3} last_physics_ms={:.3} contacts={} finite_positions={} failed={}",
                result.bodies,
                result.average_fps,
                result.average_frame_ms,
                result.last_physics_ms,
                result.contacts,
                result.finite_positions,
                result.failed,
            );
            assert!(
                !result.failed,
                "unstable workload in {mode} mode: {result:?}"
            );
        }
    }
}
