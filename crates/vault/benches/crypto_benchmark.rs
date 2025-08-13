use chamber_vault::crypto::{
    aead_decrypt, aead_encrypt, derive_key, unwrap_vault_key, wrap_vault_key, KdfParams, KeyMaterial,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::time::Duration;

fn bench_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_generation");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("random_key", |b| {
        b.iter(|| black_box(KeyMaterial::random()));
    });

    group.finish();
}

fn bench_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_derivation");
    group.measurement_time(Duration::from_secs(30)); // KDF is slow

    let passwords = [
        "short",
        "medium_length_password",
        "very_long_password_that_might_be_used_by_someone",
    ];

    // Test with different KDF parameters
    let kdf_configs = vec![
        (
            "fast",
            KdfParams {
                salt: vec![0u8; 32],
                m_cost_kib: 4096, // 4MB
                t_cost: 1,
                p_cost: 1,
            },
        ),
        ("secure", KdfParams::default_secure()),
        (
            "high_security",
            KdfParams {
                salt: vec![0u8; 32],
                m_cost_kib: 65536, // 64MB
                t_cost: 3,
                p_cost: 4,
            },
        ),
    ];

    for (config_name, kdf_params) in kdf_configs {
        for password in &passwords {
            group.bench_with_input(
                BenchmarkId::new(config_name, password.len()),
                &(password, &kdf_params),
                |b, &(password, kdf_params)| {
                    b.iter(|| black_box(derive_key(password, kdf_params).unwrap()));
                },
            );
        }
    }

    group.finish();
}

fn bench_key_wrapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_wrapping");
    group.measurement_time(Duration::from_secs(10));

    let master_key = KeyMaterial::random();
    let vault_key = KeyMaterial::random();

    group.bench_function("wrap", |b| {
        b.iter(|| black_box(wrap_vault_key(&master_key, &vault_key).unwrap()));
    });

    let (wrapped, verifier) = wrap_vault_key(&master_key, &vault_key).unwrap();

    group.bench_function("unwrap_with_verifier", |b| {
        b.iter(|| black_box(unwrap_vault_key(&master_key, &wrapped, Some(&verifier)).unwrap()));
    });

    group.bench_function("unwrap_without_verifier", |b| {
        b.iter(|| black_box(unwrap_vault_key(&master_key, &wrapped, None).unwrap()));
    });

    group.finish();
}

fn bench_aead_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("aead_operations");
    group.measurement_time(Duration::from_secs(10));

    let key = KeyMaterial::random();
    let associated_data = b"test associated data";

    // Test with different data sizes
    let data_sizes = [16, 256, 1024, 8192, 65536]; // 16B to 64KB

    for &size in &data_sizes {
        let plaintext = vec![0u8; size];

        group.bench_with_input(BenchmarkId::new("encrypt", size), &plaintext, |b, plaintext| {
            b.iter(|| black_box(aead_encrypt(&key, plaintext, associated_data).unwrap()));
        });

        let (nonce, ciphertext) = aead_encrypt(&key, &plaintext, associated_data).unwrap();

        group.bench_with_input(
            BenchmarkId::new("decrypt", size),
            &(&nonce, &ciphertext),
            |b, (nonce, ciphertext)| {
                b.iter(|| black_box(aead_decrypt(&key, nonce, ciphertext, associated_data).unwrap()));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_key_generation,
    bench_key_derivation,
    bench_key_wrapping,
    bench_aead_operations
);
criterion_main!(benches);
