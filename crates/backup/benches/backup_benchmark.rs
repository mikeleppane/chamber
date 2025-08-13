use anyhow::Result;
use chamber_backup::{BackupManager, VaultOperations};
use chamber_vault::{BackupConfig, Item, ItemKind};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::time::Duration;
use tempfile::TempDir;
use time::OffsetDateTime;

use std::hint::black_box;

// Mock vault implementation for benchmarking
struct MockVault {
    items: Vec<Item>,
}

#[allow(clippy::cast_possible_wrap)]
impl MockVault {
    fn new(item_count: usize) -> Self {
        let items = (0..item_count)
            .map(|i| Item {
                id: i as i64,
                name: format!("benchmark_item_{i}"),
                kind: ItemKind::Password,
                value: format!("benchmark_password_{i}_with_additional_content_to_simulate_real_world_data"),
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            })
            .collect();

        Self { items }
    }
}

impl VaultOperations for MockVault {
    fn list_items(&self) -> Result<Vec<Item>> {
        Ok(self.items.clone())
    }
}

fn create_test_config(temp_dir: &TempDir, format: &str, compress: bool) -> BackupConfig {
    BackupConfig {
        enabled: true,
        backup_dir: temp_dir.path().to_path_buf(),
        format: format.to_string(),
        compress,
        verify_after_backup: false,
        max_backups: 10,
        interval_hours: 0,
    }
}

fn bench_backup_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("backup_creation");
    group.measurement_time(Duration::from_secs(15));

    let item_counts = [100, 500, 1000, 2000];
    let formats = ["json", "csv", "backup"];
    let compress_options = [false, true];

    for &count in &item_counts {
        let _ = MockVault::new(count);

        for format in &formats {
            for &compress in &compress_options {
                let config_name = if compress {
                    format!("{format}_compressed")
                } else {
                    (*format).to_string()
                };

                group.bench_with_input(
                    BenchmarkId::new(config_name, count),
                    &(count, format, compress),
                    |b, &(count, format, compress)| {
                        b.iter_batched(
                            || {
                                let temp_dir = TempDir::new().unwrap();
                                let config = create_test_config(&temp_dir, format, compress);
                                let vault = MockVault::new(count);
                                let manager = BackupManager::new(vault, config);
                                (temp_dir, manager)
                            },
                            |(_temp_dir, mut manager)| black_box(manager.force_backup().unwrap()),
                            criterion::BatchSize::SmallInput,
                        );
                    },
                );
            }
        }
    }

    group.finish();
}

criterion_group!(benches, bench_backup_creation,);
criterion_main!(benches);
