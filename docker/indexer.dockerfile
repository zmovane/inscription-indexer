FROM lukemathwalker/cargo-chef:latest-rust-1.75.0 as chef
WORKDIR /app

FROM chef as planner
COPY . .

RUN apt-get update -y \
    && apt-get install libclang-dev -y
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.
COPY . .
# Build our project
RUN apt-get update -y \
    && apt-get install protobuf-compiler -y
RUN cargo build --release --bin inscription

FROM debian:bullseye-slim AS runtime
ARG CHAIN_ID
WORKDIR /app/insc-indexer-$CHAIN_ID
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/inscription insc-indexer-$CHAIN_ID
