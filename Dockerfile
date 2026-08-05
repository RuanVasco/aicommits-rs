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
COPY .cargo ./.cargo
COPY src ./src

RUN cargo build --release --target x86_64-pc-windows-gnu

# tokenizers' C++ code (esaxx-rs) links libstdc++ dynamically and crt-static doesn't
# cover it, so aic.exe needs these DLLs next to it on Windows: libstdc++-6.dll itself,
# plus libgcc_s_seh-1.dll which libstdc++-6.dll in turn depends on (checked with
# `objdump -p`; nothing beyond it needs bundling). `-print-file-name` asks gcc for
# whichever thread-model variant (posix/win32) is actually active, instead of
# hardcoding a path that would break silently on a gcc version bump.
RUN cp "$(x86_64-w64-mingw32-g++ -print-file-name=libstdc++-6.dll)" /app/libstdc++-6.dll && \
    cp "$(x86_64-w64-mingw32-g++ -print-file-name=libgcc_s_seh-1.dll)" /app/libgcc_s_seh-1.dll

# Empty final stage that only holds the built binary, so `docker create`
# doesn't need to pull the whole ~1GB builder image just to copy it out.
# CMD is required even though the container is never started (only created,
# to `docker cp` out of) - scratch has no default command otherwise, and
# `docker create` refuses with "no command specified" without one.
FROM scratch AS export
COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/aic.exe /aic.exe
COPY --from=builder /app/libstdc++-6.dll /libstdc++-6.dll
COPY --from=builder /app/libgcc_s_seh-1.dll /libgcc_s_seh-1.dll
CMD ["/aic.exe"]
