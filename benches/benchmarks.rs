use criterion::*;
use nalgebra_glm::{Mat4, Vec3};
use turbo_ecs::create_archetype;
use turbo_ecs::prelude::*;

const COUNT: usize = 10000;

#[derive(Default, Component)]
struct Transform(Mat4);

#[derive(Default, Component)]
struct Translation(Vec3);

#[derive(Default, Component)]
struct Rotation(Vec3);

#[derive(Default, Component)]
struct Velocity(Vec3);

fn create_entities(c: &mut Criterion) {
    c.bench_function("Create entities", |b| {
        let mut entities = vec![Entity::default(); COUNT];
        b.iter_batched(
            || {
                let mut ecs = EcsContext::new();
                let archetype = create_archetype!(ecs, [Transform, Translation, Rotation, Velocity]);
                (ecs, archetype)
            },
            |(mut ecs, archetype)| ecs.create_entities_from_archetype(archetype, &mut entities),
            BatchSize::PerIteration,
        );
    });
}

fn destroy_entities(c: &mut Criterion) {
    c.bench_function("Destroy entities", |b| {
        b.iter_batched(
            || {
                let mut ecs = EcsContext::new();
                let mut entities = vec![Entity::default(); COUNT];
                let archetype =
                    create_archetype!(ecs, [Transform, Translation, Rotation, Velocity]);
                ecs.create_entities_from_archetype(archetype, &mut entities);
                (ecs, entities)
            },
            |(mut ecs, entities)| ecs.destroy_entities(&entities),
            BatchSize::PerIteration,
        );
    });
}

fn iterate_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("Iterate entities");
    group.bench_function("Single-threaded", |b| {
        let mut ecs = EcsContext::new();
        let mut entities = vec![Entity::default(); COUNT];
        let archetype = create_archetype!(ecs, [Transform, Translation, Rotation, Velocity]);
        ecs.create_entities_from_archetype(archetype, &mut entities);

        b.iter(|| {
            ecs.filter()
                .include::<(&mut Transform, &mut Translation, &Velocity, &Rotation)>()
                .for_each(|(m, t, v, r)| {
                    t.0 += v.0;
                    m.0 = Mat4::new_translation(&t.0) * Mat4::new_rotation(r.0);
                })
        });
    });

    group.bench_function("Multi-threaded", |b| {
        let mut ecs = EcsContext::new();
        let mut entities = vec![Entity::default(); COUNT];
        let archetype = create_archetype!(ecs, [Transform, Translation, Rotation, Velocity]);
        ecs.create_entities_from_archetype(archetype, &mut entities);

        b.iter(|| {
            ecs.filter()
                .include::<(&mut Transform, &mut Translation, &Velocity, &Rotation)>()
                .par_for_each(|(m, t, v, r)| {
                    t.0 += v.0;
                    m.0 = Mat4::new_translation(&t.0) * Mat4::new_rotation(r.0);
                })
        });
    });
}

criterion_group!(
    benchmarks,
    create_entities,
    destroy_entities,
    iterate_entities,
);
criterion_main!(benchmarks);
