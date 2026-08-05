# syntax=docker/dockerfile:1

# Builds aic.exe for Windows by cross-compiling from a Linux container,
# so building the project doesn't require installing Rust (or mingw) on the host.
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc-mingw-w64-x86-64 \
    g++-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-pc-windows-gnu

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --target x86_64-pc-windows-gnu

# Empty final stage that only holds the built binary, so `docker create`
# doesn't need to pull the whole ~1GB builder image just to copy it out.
# CMD is required even though the container is never started (only created,
# to `docker cp` out of) - scratch has no default command otherwise, and
# `docker create` refuses with "no command specified" without one.
FROM scratch AS export
COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/aic.exe /aic.exe
CMD ["/aic.exe"]
