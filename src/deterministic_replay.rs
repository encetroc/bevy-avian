//! Headless deterministic-replay experiment. Each run starts from the same
//! initial world and consumes the same tick-indexed input trace.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use avian3d::prelude::*;
    use bevy::{
        mesh::MeshPlugin, prelude::*, time::TimeUpdateStrategy, transform::TransformPlugin,
    };

    const STEP: f32 = 1.0 / 60.0;
    const RUN_STEPS: usize = 180;

    #[derive(Component)]
    struct ReplayBody;

    #[derive(Clone, Copy, Debug)]
    struct ReplayInput {
        tick: usize,
        velocity_delta: Vec3,
    }

    const INPUT_TRACE: [ReplayInput; 3] = [
        ReplayInput {
            tick: 0,
            velocity_delta: Vec3::new(2.0, 0.0, 0.0),
        },
        ReplayInput {
            tick: 30,
            velocity_delta: Vec3::new(0.0, 0.0, -1.0),
        },
        ReplayInput {
            tick: 90,
            velocity_delta: Vec3::new(-0.5, 0.0, 0.0),
        },
    ];

    #[derive(Clone, Debug)]
    struct ReplayOutcome {
        position: Vec3,
        rotation: Quat,
        linear_velocity: Vec3,
        dynamic_bodies: usize,
        colliders: usize,
        active_contacts: usize,
    }

    #[derive(Debug)]
    struct ReplayDifference {
        measurement: &'static str,
        first: String,
        replay: String,
    }

    fn replay() -> ReplayOutcome {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            MeshPlugin,
            TransformPlugin,
            PhysicsPlugins::default(),
        ))
        .insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            STEP,
        )));
        app.finish();

        let body = app
            .world_mut()
            .spawn((
                ReplayBody,
                RigidBody::Dynamic,
                Collider::cuboid(0.8, 0.8, 0.8),
                Position(Vec3::new(0.0, 4.0, 0.0)),
                Transform::from_xyz(0.0, 4.0, 0.0),
                Rotation::default(),
                LinearVelocity::default(),
                Name::new("deterministic replay body"),
            ))
            .id();
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(20.0, 0.5, 20.0),
            Position(Vec3::new(0.0, -0.25, 0.0)),
            Transform::from_xyz(0.0, -0.25, 0.0),
            Name::new("replay floor"),
        ));

        let mut next_input = 0;
        for tick in 0..RUN_STEPS {
            while INPUT_TRACE
                .get(next_input)
                .is_some_and(|input| input.tick == tick)
            {
                let input = INPUT_TRACE[next_input];
                let mut entity = app.world_mut().entity_mut(body);
                let mut velocity = entity
                    .get_mut::<LinearVelocity>()
                    .expect("replay body velocity");
                velocity.0 += input.velocity_delta;
                next_input += 1;
            }
            app.update();
        }
        assert_eq!(
            next_input,
            INPUT_TRACE.len(),
            "every input must be replayed"
        );

        let entity = app.world().entity(body);
        ReplayOutcome {
            position: entity.get::<Position>().expect("body position").0,
            rotation: entity.get::<Rotation>().expect("body rotation").0,
            linear_velocity: entity.get::<LinearVelocity>().expect("body velocity").0,
            dynamic_bodies: {
                let world = app.world_mut();
                let mut query = world.query::<&RigidBody>();
                query
                    .iter(world)
                    .filter(|body| **body == RigidBody::Dynamic)
                    .count()
            },
            colliders: {
                let world = app.world_mut();
                let mut query = world.query_filtered::<Entity, With<Collider>>();
                query.iter(world).count()
            },
            active_contacts: app
                .world()
                .resource::<ContactGraph>()
                .iter_active_touching()
                .count(),
        }
    }

    fn compare(first: &ReplayOutcome, replay: &ReplayOutcome) -> Vec<ReplayDifference> {
        let mut differences = Vec::new();
        macro_rules! compare_measurement {
            ($field:ident) => {
                if first.$field != replay.$field {
                    differences.push(ReplayDifference {
                        measurement: stringify!($field),
                        first: format!("{:?}", first.$field),
                        replay: format!("{:?}", replay.$field),
                    });
                }
            };
        }
        compare_measurement!(position);
        compare_measurement!(rotation);
        compare_measurement!(linear_velocity);
        compare_measurement!(dynamic_bodies);
        compare_measurement!(colliders);
        compare_measurement!(active_contacts);
        differences
    }

    #[test]
    fn replaying_the_same_inputs_from_reset_compares_final_state_and_diagnostics() {
        let first = replay();
        let replayed = replay();
        let differences = compare(&first, &replayed);

        assert!(
            differences.is_empty(),
            "replay diverged; all differences are retained here: {}\nfirst: {first:#?}\nreplay: {replayed:#?}",
            differences
                .iter()
                .map(|difference| format!(
                    "{}: {} != {}",
                    difference.measurement, difference.first, difference.replay
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(first.position.is_finite());
        assert!(first.linear_velocity.is_finite());
        assert_eq!(first.dynamic_bodies, 1);
        assert_eq!(first.colliders, 2);
        assert!(
            first.position.y < 4.0,
            "scenario did not advance under gravity: {first:#?}"
        );
    }
}
