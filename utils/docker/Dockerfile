# syntax=docker/dockerfile:1.3

# Based on https://levelup.gitconnected.com/1940db638a6c
#
# We don't do cross compilation at the moment but build the dependencies first
# anyway to get the ability to increment.
#
# cf. https://github.com/varfish-org/viguno/issues/6#issuecomment-1586216139

# ---------------------------------------------------------------------------
# Builder
# ---------------------------------------------------------------------------

# Pinning Rust version for now because of this issue:
#
# - https://github.com/rust-lang/rust/issues/95926
FROM rust:1-buster AS builder

# Build dependencies first.
#
# Install dependencies for compilation of C code (e.g., rocksdb).
RUN apt-get update && \
    apt-get install -y clang
# Add the needed Cargo components.
RUN rustup component add rustfmt
# Install build dependency `protoc`.
COPY utils/install-protoc.sh /tmp
RUN PREFIX=/usr/local bash /tmp/install-protoc.sh

# Now for the two-step building.
#
# Set initial workdir.
WORKDIR /usr/src
# Create blank project.
RUN USER=root cargo new viguno
# We want dependencies cached, so copy those first.
COPY Cargo.toml Cargo.lock /usr/src/viguno/
# Set the working directory.
WORKDIR /usr/src/viguno
# This is a dummy build to get the dependencies cached.
RUN cargo build --release
#
# Now copy in the rest of the sources.
COPY build.rs /usr/src/viguno/
COPY src /usr/src/viguno/src/
COPY protos /usr/src/viguno/protos/
COPY utils/alpine-linker-script.sh /usr/src/viguno/utils/
RUN chmod a+rx /usr/src/viguno/utils/alpine-linker-script.sh
COPY .cargo /usr/src/viguno/.cargo/
## Touch main.rs to prevent cached release build.
RUN touch /usr/src/viguno/src/main.rs
# This is the actual application build.
RUN cargo build --release

# ---------------------------------------------------------------------------
# Runtime
# ---------------------------------------------------------------------------

FROM debian:buster-slim AS runtime

# Copy application binary from builder image
COPY --from=builder \
    /usr/src/viguno/target/release/viguno \
    /usr/local/bin

# Copy the entrypoint script and make it executable.
COPY utils/docker/entrypoint.sh /
RUN chmod a+rx /entrypoint.sh

# Set the entrypoint.
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
# Set port to expose
EXPOSE 8080
