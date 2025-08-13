# List available commands
default:
    @just --list

all: format lint

# Format code using rustfmt
format:
    cargo fmt --all

# Run clippy to lint the code
lint:
    cargo fmt -- --check
    cargo clippy --all-features --all-targets -- -D warnings

# Fix linting issues where possible
lint-fix:
    cargo clippy --fix -- -D warnings

# Run tests
test:
    cargo nextest run --workspace

# Run all benchmarks
bench-all:
    cargo bench

# Run password generation benchmarks
bench-password:
    cargo bench --package chamber-password-gen

# Run crypto benchmarks
bench-crypto:
    cargo bench --package chamber-vault

# Run import/export benchmarks
bench-import-export:
    cargo bench --package chamber-import-export

# Run backup benchmarks
bench-backup:
    cargo bench --package chamber-backup

# Generate benchmark reports with flame graphs (requires additional setup)
bench-profile:
    cargo bench --all -- --profile-time=5

# Clean benchmark results
bench-clean:
    rm -rf target/criterion

# Compare benchmarks (run after making changes)
bench-compare baseline_name:
    cargo bench -- --save-baseline {{baseline_name}}
    # After making changes, run: cargo bench -- --baseline {{baseline_name}}