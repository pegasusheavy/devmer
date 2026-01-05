//! Benchmarks for devmer-core crate.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use devmer_core::{
    Resource, ResourceGraph, ResourceType, Urn,
    types::PropertyValues,
};

/// Create a test resource with a given name.
fn create_test_resource(stack: &str, name: &str) -> Resource {
    Resource::new(
        stack,
        ResourceType::new("aws", "s3", "Bucket"),
        name,
        PropertyValues::new(),
    )
}

/// Benchmark URN creation and parsing.
fn bench_urn_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("urn");

    group.bench_function("create", |b| {
        b.iter(|| {
            black_box(Urn::new(
                black_box("production"),
                black_box("aws:s3:Bucket"),
                black_box("my-bucket"),
            ))
        });
    });

    let urn_str = "urn:devmer:production::aws:s3:Bucket::my-bucket";
    group.bench_function("parse", |b| {
        b.iter(|| {
            black_box(Urn::parse(black_box(urn_str)).unwrap())
        });
    });

    let urn = Urn::new("production", "aws:s3:Bucket", "my-bucket");
    group.bench_function("stack_extraction", |b| {
        b.iter(|| {
            black_box(urn.stack())
        });
    });

    group.bench_function("resource_type_extraction", |b| {
        b.iter(|| {
            black_box(urn.resource_type())
        });
    });

    group.bench_function("name_extraction", |b| {
        b.iter(|| {
            black_box(urn.name())
        });
    });

    group.finish();
}

/// Benchmark ResourceType operations.
fn bench_resource_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_type");

    group.bench_function("create", |b| {
        b.iter(|| {
            black_box(ResourceType::new(
                black_box("aws"),
                black_box("s3"),
                black_box("Bucket"),
            ))
        });
    });

    group.bench_function("parse", |b| {
        b.iter(|| {
            black_box(ResourceType::parse(black_box("aws:s3:Bucket")).unwrap())
        });
    });

    let rt = ResourceType::new("aws", "s3", "Bucket");
    group.bench_function("provider_extraction", |b| {
        b.iter(|| {
            black_box(rt.provider())
        });
    });

    group.finish();
}

/// Benchmark Resource creation.
fn bench_resource_creation(c: &mut Criterion) {
    c.bench_function("resource_new", |b| {
        b.iter(|| {
            black_box(Resource::new(
                black_box("production"),
                black_box(ResourceType::new("aws", "s3", "Bucket")),
                black_box("my-bucket"),
                black_box(PropertyValues::new()),
            ))
        });
    });
}

/// Benchmark ResourceGraph operations with varying graph sizes.
fn bench_graph_add_resource(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_add_resource");

    for size in [10, 100, 500, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("graph_size", size),
            &size,
            |b, &graph_size| {
                b.iter_batched(
                    || {
                        let mut graph = ResourceGraph::new();
                        for i in 0..graph_size {
                            let resource = create_test_resource("test", &format!("resource-{}", i));
                            graph.add_resource(resource);
                        }
                        graph
                    },
                    |mut graph| {
                        let new_resource = create_test_resource("test", "new-resource");
                        black_box(graph.add_resource(new_resource))
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark graph lookup operations.
fn bench_graph_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_lookup");

    for size in [10, 100, 500, 1000] {
        // Create graph with resources
        let mut graph = ResourceGraph::new();
        let mut urns = Vec::with_capacity(size);

        for i in 0..size {
            let resource = create_test_resource("test", &format!("resource-{}", i));
            urns.push(resource.urn.clone());
            graph.add_resource(resource);
        }

        // Lookup in middle of graph
        let target_urn = &urns[size / 2];

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("graph_size", size),
            &(&graph, target_urn),
            |b, (g, urn)| {
                b.iter(|| {
                    black_box(g.get_resource(black_box(urn)))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark topological sort with varying graph sizes.
fn bench_graph_topological_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_topological_sort");

    for size in [10, 50, 100, 200] {
        // Create graph with chain dependencies
        let mut graph = ResourceGraph::new();
        let mut prev_urn: Option<Urn> = None;

        for i in 0..size {
            let resource = create_test_resource("test", &format!("resource-{}", i));
            let urn = resource.urn.clone();
            graph.add_resource(resource);

            // Chain dependency: each resource depends on the previous one
            if let Some(ref prev) = prev_urn {
                graph.add_dependency(
                    &urn,
                    prev,
                    devmer_core::graph::DependencyKind::Explicit,
                ).unwrap();
            }
            prev_urn = Some(urn);
        }

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("chain_size", size),
            &graph,
            |b, g| {
                b.iter(|| {
                    black_box(g.topological_sort().unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark creation order calculation.
fn bench_graph_creation_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_creation_order");

    for size in [10, 50, 100] {
        // Create graph with tree-like dependencies
        let mut graph = ResourceGraph::new();
        let mut urns = Vec::with_capacity(size);

        for i in 0..size {
            let resource = create_test_resource("test", &format!("resource-{}", i));
            urns.push(resource.urn.clone());
            graph.add_resource(resource);

            // Tree structure: resource i depends on resource i/2
            if i > 0 {
                graph.add_dependency(
                    &urns[i],
                    &urns[i / 2],
                    devmer_core::graph::DependencyKind::Explicit,
                ).unwrap();
            }
        }

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("tree_size", size),
            &graph,
            |b, g| {
                b.iter(|| {
                    black_box(g.creation_order().unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cycle detection.
fn bench_graph_cycle_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_cycle_detection");

    for size in [10, 50, 100] {
        // Create graph with chain dependencies
        let mut graph = ResourceGraph::new();
        let mut urns = Vec::with_capacity(size);

        for i in 0..size {
            let resource = create_test_resource("test", &format!("resource-{}", i));
            urns.push(resource.urn.clone());
            graph.add_resource(resource);

            if i > 0 {
                graph.add_dependency(
                    &urns[i],
                    &urns[i - 1],
                    devmer_core::graph::DependencyKind::Explicit,
                ).unwrap();
            }
        }

        // Check if adding edge from first to last would create cycle
        let first = &urns[0];
        let last = &urns[size - 1];

        group.bench_with_input(
            BenchmarkId::new("chain_size", size),
            &(&graph, first, last),
            |b, (g, f, l)| {
                b.iter(|| {
                    black_box(g.would_create_cycle(black_box(f), black_box(l)))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark dependency/dependent lookup.
fn bench_graph_dependency_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_dependency_lookup");

    // Create graph with star topology (one center, many leaves)
    let mut graph = ResourceGraph::new();
    let center = create_test_resource("test", "center");
    let center_urn = center.urn.clone();
    graph.add_resource(center);

    let num_leaves = 50;
    for i in 0..num_leaves {
        let leaf = create_test_resource("test", &format!("leaf-{}", i));
        let leaf_urn = leaf.urn.clone();
        graph.add_resource(leaf);
        graph.add_dependency(
            &leaf_urn,
            &center_urn,
            devmer_core::graph::DependencyKind::Explicit,
        ).unwrap();
    }

    group.bench_function("dependencies", |b| {
        let leaf_urn = Urn::new("test", "aws:s3:Bucket", "leaf-0");
        b.iter(|| {
            black_box(graph.dependencies(black_box(&leaf_urn)))
        });
    });

    group.bench_function("dependents", |b| {
        b.iter(|| {
            black_box(graph.dependents(black_box(&center_urn)))
        });
    });

    group.finish();
}

/// Benchmark PropertyValues operations.
fn bench_property_values(c: &mut Criterion) {
    use devmer_core::types::PropertyValue;

    let mut group = c.benchmark_group("property_values");

    group.bench_function("new", |b| {
        b.iter(|| {
            black_box(PropertyValues::new())
        });
    });

    group.bench_function("insert_string", |b| {
        b.iter_batched(
            PropertyValues::new,
            |mut props| {
                props.insert("key".to_string(), PropertyValue::String("value".to_string()));
                black_box(props)
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Create a populated PropertyValues for lookup benchmarks
    let mut props = PropertyValues::new();
    for i in 0..20 {
        props.insert(
            format!("key-{}", i),
            PropertyValue::String(format!("value-{}", i)),
        );
    }

    group.bench_function("get_existing", |b| {
        b.iter(|| {
            black_box(props.get(black_box("key-10")))
        });
    });

    group.bench_function("get_missing", |b| {
        b.iter(|| {
            black_box(props.get(black_box("nonexistent")))
        });
    });

    group.finish();
}

/// Benchmark resource serialization.
fn bench_resource_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_serialization");

    let resource = create_test_resource("production", "my-bucket");

    group.bench_function("serialize_json", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(black_box(&resource)).unwrap())
        });
    });

    let json_str = serde_json::to_string(&resource).unwrap();
    group.bench_function("deserialize_json", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<Resource>(black_box(&json_str)).unwrap())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_urn_operations,
    bench_resource_type,
    bench_resource_creation,
    bench_graph_add_resource,
    bench_graph_lookup,
    bench_graph_topological_sort,
    bench_graph_creation_order,
    bench_graph_cycle_detection,
    bench_graph_dependency_lookup,
    bench_property_values,
    bench_resource_serialization,
);

criterion_main!(benches);
