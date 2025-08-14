# Use Debian for building to avoid Alpine + jemalloc issues
FROM rust:1.89-bookworm AS builder

# Install dependencies for building
RUN apt update && apt install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    musl-tools \
    && apt clean \
    && rm -rf /var/lib/apt/lists/*

# Add MUSL target
RUN rustup target add x86_64-unknown-linux-musl

# Set working directory
WORKDIR /usr/src/chamber

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the application
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - minimal Alpine image
FROM gcr.io/distroless/static-debian12:nonroot

# Copy binary from builder stage
COPY --from=builder /usr/src/chamber/target/x86_64-unknown-linux-musl/release/chamber /usr/local/bin/chamber

# Create directory for vault data (distroless already has nonroot user)
USER 65532:65532
WORKDIR /home/nonroot

ENTRYPOINT ["/usr/local/bin/chamber"]
CMD ["--help"]
