use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use nexa_core::domain::scheduler::{NodeSnapshot, PodRequest, SchedulerWeights, WeightedScheduler};
use uuid::Uuid;

fn make_nodes(count: usize) -> Vec<NodeSnapshot> {
    (0..count)
        .map(|i| {
            let fraction = (i + 1) as f64 / count as f64;
            NodeSnapshot {
                node_id: Uuid::new_v4(),
                cpu_available: 4.0 * (1.0 - fraction * 0.5),
                cpu_total: 4.0,
                memory_available: (8_000_000_000.0 * (1.0 - fraction * 0.4)) as u64,
                memory_total: 8_000_000_000,
                running_pods: (fraction * 60.0) as u32,
                max_pods: 100,
                recent_failures: vec![],
            }
        })
        .collect()
}

fn bench_scheduler_spread(c: &mut Criterion) {
    let scheduler = WeightedScheduler::new(SchedulerWeights::spread());
    let request = PodRequest {
        cpu_request: 0.5,
        memory_request: 512_000_000,
    };

    let mut group = c.benchmark_group("scheduler_spread");
    for count in [5, 20] {
        let nodes = make_nodes(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &nodes, |b, nodes| {
            b.iter(|| scheduler.select_node(&request, nodes).ok());
        });
    }
    group.finish();
}

fn bench_scheduler_binpack(c: &mut Criterion) {
    let scheduler = WeightedScheduler::new(SchedulerWeights::binpack());
    let request = PodRequest {
        cpu_request: 0.5,
        memory_request: 512_000_000,
    };

    let mut group = c.benchmark_group("scheduler_binpack");
    for count in [5, 20] {
        let nodes = make_nodes(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &nodes, |b, nodes| {
            b.iter(|| scheduler.select_node(&request, nodes).ok());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_scheduler_spread, bench_scheduler_binpack);
criterion_main!(benches);
