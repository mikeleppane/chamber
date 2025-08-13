use chamber_password_gen::{PasswordConfig, generate_memorable_password};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;
fn bench_password_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("password_generation");
    group.measurement_time(Duration::from_secs(10));

    // Test different password lengths
    let lengths = [8, 16, 32, 64, 128];

    for length in lengths {
        let config = PasswordConfig {
            length,
            include_uppercase: true,
            include_lowercase: true,
            include_digits: true,
            include_symbols: true,
            exclude_ambiguous: false,
        };

        group.bench_with_input(BenchmarkId::new("standard", length), &config, |b, config| {
            b.iter(|| black_box(config.generate().unwrap()));
        });
    }

    // Test different character set combinations
    let configs = vec![
        (
            "minimal",
            PasswordConfig {
                length: 16,
                include_uppercase: false,
                include_lowercase: true,
                include_digits: true,
                include_symbols: false,
                exclude_ambiguous: false,
            },
        ),
        (
            "alphanumeric",
            PasswordConfig {
                length: 16,
                include_uppercase: true,
                include_lowercase: true,
                include_digits: true,
                include_symbols: false,
                exclude_ambiguous: false,
            },
        ),
        (
            "full_charset",
            PasswordConfig {
                length: 16,
                include_uppercase: true,
                include_lowercase: true,
                include_digits: true,
                include_symbols: true,
                exclude_ambiguous: false,
            },
        ),
        (
            "no_ambiguous",
            PasswordConfig {
                length: 16,
                include_uppercase: true,
                include_lowercase: true,
                include_digits: true,
                include_symbols: true,
                exclude_ambiguous: true,
            },
        ),
    ];

    for (name, config) in configs {
        group.bench_with_input(BenchmarkId::new("charset", name), &config, |b, config| {
            b.iter(|| black_box(config.generate().unwrap()));
        });
    }

    group.finish();
}

fn bench_memorable_password_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memorable_password");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("generate", |b| {
        b.iter(|| black_box(generate_memorable_password()));
    });

    // Benchmark generating multiple passwords at once
    let batch_sizes = [10, 100, 1000];
    for size in batch_sizes {
        group.bench_with_input(BenchmarkId::new("batch", size), &size, |b, &size| {
            b.iter(|| {
                let passwords: Vec<String> = (0..size).map(|_| generate_memorable_password()).collect();
                black_box(passwords)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_password_generation, bench_memorable_password_generation);
criterion_main!(benches);
