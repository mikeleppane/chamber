use chamber_import_export::{ExportFormat, export_items, import_items};
use chamber_vault::{Item, ItemKind};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;
use tempfile::{NamedTempFile, TempDir};
use time::OffsetDateTime;

#[allow(clippy::cast_possible_wrap)]
fn create_test_items(count: usize) -> Vec<Item> {
    (0..count)
        .map(|i| Item {
            id: i as u64,
            name: format!("test_item_{i}"),
            kind: match i % 4 {
                0 => ItemKind::SshKey,
                1 => ItemKind::Password,
                2 => ItemKind::Note,
                _ => ItemKind::ApiKey,
            },
            value: format!("test_value_{i}_with_some_longer_content_to_simulate_real_data"),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        })
        .collect()
}

fn bench_export_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_operations");
    group.measurement_time(Duration::from_secs(15));

    let item_counts = [10, 100, 1000, 5000];
    let formats = [
        ("json", ExportFormat::Json),
        ("csv", ExportFormat::Csv),
        ("backup", ExportFormat::ChamberBackup),
    ];

    for &count in &item_counts {
        let items = create_test_items(count);

        for (format_name, format) in &formats {
            group.bench_with_input(
                BenchmarkId::new(*format_name, count),
                &(&items, format),
                |b, (items, format)| {
                    b.iter_batched(
                        || {
                            let temp_dir = TempDir::new().unwrap();
                            let file_path = temp_dir.path().join("test_export");
                            (temp_dir, file_path)
                        },
                        |(_temp_dir, file_path)| {
                            export_items(items, format, &file_path).unwrap();
                            black_box(());
                        },
                        criterion::BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    group.finish();
}

fn bench_import_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("import_operations");
    group.measurement_time(Duration::from_secs(15));

    let item_counts = [10, 100, 1000, 5000];
    let formats = [
        ("json", ExportFormat::Json),
        ("csv", ExportFormat::Csv),
        ("backup", ExportFormat::ChamberBackup),
    ];

    for &count in &item_counts {
        let items = create_test_items(count);

        for (format_name, format) in &formats {
            // Pre-create the test file
            let temp_file = NamedTempFile::new().unwrap();
            export_items(&items, format, temp_file.path()).unwrap();

            group.bench_with_input(
                BenchmarkId::new(*format_name, count),
                &(temp_file.path(), format),
                |b, (path, format)| {
                    b.iter(|| black_box(import_items(path, format).unwrap()));
                },
            );
        }
    }

    group.finish();
}

fn bench_round_trip_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip");
    group.measurement_time(Duration::from_secs(20));

    let item_counts = [100, 1000];
    let formats = [
        ("json", ExportFormat::Json),
        ("csv", ExportFormat::Csv),
        ("backup", ExportFormat::ChamberBackup),
    ];

    for &count in &item_counts {
        let items = create_test_items(count);

        for (format_name, format) in &formats {
            group.bench_with_input(
                BenchmarkId::new(*format_name, count),
                &(&items, format),
                |b, (items, format)| {
                    b.iter_batched(
                        || {
                            let temp_dir = TempDir::new().unwrap();
                            let file_path = temp_dir.path().join("test_roundtrip");
                            (temp_dir, file_path)
                        },
                        |(_temp_dir, file_path)| {
                            // Export then import
                            export_items(items, format, &file_path).unwrap();
                            let imported = import_items(&file_path, format).unwrap();
                            black_box(imported)
                        },
                        criterion::BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_export_operations,
    bench_import_operations,
    bench_round_trip_operations
);
criterion_main!(benches);
