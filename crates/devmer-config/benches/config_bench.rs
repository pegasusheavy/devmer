//! Benchmarks for devmer-config crate.

use std::hint::black_box;
use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use devmer_config::interpolation::Interpolator;

/// Benchmark interpolator creation.
fn bench_interpolator_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpolator_creation");

    group.bench_function("new", |b| {
        b.iter(|| {
            black_box(Interpolator::new())
        });
    });

    group.bench_function("with_single_var", |b| {
        b.iter(|| {
            black_box(Interpolator::new().with_var("KEY", "value"))
        });
    });

    let vars: HashMap<String, String> = (0..10)
        .map(|i| (format!("VAR_{}", i), format!("value_{}", i)))
        .collect();

    group.bench_function("with_10_vars", |b| {
        b.iter(|| {
            black_box(Interpolator::new().with_vars(vars.clone()))
        });
    });

    group.finish();
}

/// Benchmark simple interpolation.
fn bench_simple_interpolation(c: &mut Criterion) {
    let interp = Interpolator::new()
        .with_var("VAR1", "hello")
        .with_var("VAR2", "world")
        .with_var("LONG_VALUE", "this_is_a_much_longer_value_that_might_be_used");

    let mut group = c.benchmark_group("simple_interpolation");

    // No interpolation needed
    group.bench_function("no_vars", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box("plain text without variables")).unwrap())
        });
    });

    // Single variable
    group.bench_function("single_var", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box("value: ${VAR1}")).unwrap())
        });
    });

    // Multiple variables
    group.bench_function("multiple_vars", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box("${VAR1} ${VAR2}")).unwrap())
        });
    });

    // Variable with default
    group.bench_function("var_with_default_found", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box("${VAR1:-default}")).unwrap())
        });
    });

    group.bench_function("var_with_default_missing", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box("${MISSING:-default_value}")).unwrap())
        });
    });

    group.finish();
}

/// Benchmark interpolation with varying input sizes.
fn bench_interpolation_input_sizes(c: &mut Criterion) {
    let interp = Interpolator::new()
        .with_var("VAR", "replaced");

    let mut group = c.benchmark_group("interpolation_input_size");

    // Generate inputs of varying sizes with embedded variables
    let inputs: Vec<(String, String)> = vec![
        ("small".to_string(), "prefix ${VAR} suffix".to_string()),
        ("medium".to_string(), format!(
            "{} ${{VAR}} {}",
            "x".repeat(100),
            "y".repeat(100)
        )),
        ("large".to_string(), format!(
            "{} ${{VAR}} {} ${{VAR}} {}",
            "x".repeat(500),
            "y".repeat(500),
            "z".repeat(500)
        )),
    ];

    for (name, input) in &inputs {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("size", name),
            input,
            |b, inp| {
                b.iter(|| {
                    black_box(interp.interpolate(black_box(inp)).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark interpolation with varying number of variables.
fn bench_interpolation_var_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpolation_var_count");

    for var_count in [1, 5, 10, 20] {
        let mut interp = Interpolator::new();
        let mut input = String::new();

        for i in 0..var_count {
            interp = interp.with_var(format!("VAR_{}", i), format!("value_{}", i));
            input.push_str(&format!("${{VAR_{}}} ", i));
        }

        group.throughput(Throughput::Elements(var_count as u64));
        group.bench_with_input(
            BenchmarkId::new("vars", var_count),
            &(interp, input),
            |b, (int, inp)| {
                b.iter(|| {
                    black_box(int.interpolate(black_box(inp)).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark needs_interpolation check.
fn bench_needs_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("needs_interpolation");

    let test_cases = [
        ("no_vars", "plain text without any variables"),
        ("env_var", "text with ${ENV_VAR} in it"),
        ("file_ref", "text with ${file:/path/to/file} ref"),
        ("secret_ref", "text with ${secret:my_secret} ref"),
        ("multiple", "${VAR1} and ${file:/path} and ${secret:key}"),
    ];

    for (name, input) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("case", name),
            &input,
            |b, inp| {
                b.iter(|| {
                    black_box(Interpolator::needs_interpolation(black_box(inp)))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark secret reference detection.
fn bench_secret_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("secret_detection");

    group.bench_function("has_secret_refs_true", |b| {
        b.iter(|| {
            black_box(Interpolator::has_secret_refs(
                black_box("password: ${secret:db_password}")
            ))
        });
    });

    group.bench_function("has_secret_refs_false", |b| {
        b.iter(|| {
            black_box(Interpolator::has_secret_refs(
                black_box("password: ${DB_PASSWORD}")
            ))
        });
    });

    group.finish();
}

/// Benchmark secret reference extraction.
fn bench_extract_secret_refs(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_secret_refs");

    let test_cases = [
        ("none", "no secrets here"),
        ("one", "db: ${secret:db_password}"),
        ("three", "db: ${secret:db_password}, api: ${secret:api_key}, token: ${secret:auth_token}"),
        ("mixed", "env: ${ENV_VAR}, secret: ${secret:key}, file: ${file:/path}"),
    ];

    for (name, input) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("case", name),
            &input,
            |b, inp| {
                b.iter(|| {
                    black_box(Interpolator::extract_secret_refs(black_box(inp)))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark multiline config interpolation.
fn bench_multiline_interpolation(c: &mut Criterion) {
    let interp = Interpolator::new()
        .with_var("DB_HOST", "localhost")
        .with_var("DB_PORT", "5432")
        .with_var("DB_NAME", "myapp")
        .with_var("API_KEY", "secret123");

    let mut group = c.benchmark_group("multiline_interpolation");

    let small_config = r#"
host = "${DB_HOST}"
port = ${DB_PORT}
"#;

    let medium_config = r#"
[database]
host = "${DB_HOST}"
port = ${DB_PORT}
name = "${DB_NAME}"

[api]
key = "${API_KEY}"
url = "https://${DB_HOST}:8080"
"#;

    let large_config = format!(
        r#"
[database]
host = "${{DB_HOST}}"
port = ${{DB_PORT}}
name = "${{DB_NAME}}"

[api]
key = "${{API_KEY}}"
url = "https://${{DB_HOST}}:8080"

# Repeated sections for size
{}
"#,
        (0..10)
            .map(|i| format!(
                r#"
[service_{}]
host = "${{DB_HOST}}"
port = {}
"#,
                i,
                5000 + i
            ))
            .collect::<Vec<_>>()
            .join("")
    );

    group.bench_function("small", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box(small_config)).unwrap())
        });
    });

    group.bench_function("medium", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box(medium_config)).unwrap())
        });
    });

    group.bench_function("large", |b| {
        b.iter(|| {
            black_box(interp.interpolate(black_box(&large_config)).unwrap())
        });
    });

    group.finish();
}

/// Benchmark strict vs non-strict mode.
fn bench_strict_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("strict_mode");

    let input = "${EXISTING} and ${MISSING}";

    let strict_interp = Interpolator::new()
        .with_var("EXISTING", "value")
        .strict(true);

    let non_strict_interp = Interpolator::new()
        .with_var("EXISTING", "value")
        .strict(false);

    // Strict mode with all vars present
    let all_present_input = "${EXISTING}";
    group.bench_function("strict_all_present", |b| {
        b.iter(|| {
            black_box(strict_interp.interpolate(black_box(all_present_input)).unwrap())
        });
    });

    // Non-strict mode with missing var
    group.bench_function("non_strict_missing", |b| {
        b.iter(|| {
            black_box(non_strict_interp.interpolate(black_box(input)).unwrap())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_interpolator_creation,
    bench_simple_interpolation,
    bench_interpolation_input_sizes,
    bench_interpolation_var_count,
    bench_needs_interpolation,
    bench_secret_detection,
    bench_extract_secret_refs,
    bench_multiline_interpolation,
    bench_strict_mode,
);

criterion_main!(benches);
