# syntax=docker/dockerfile:1.3

# Based on https://levelup.gitconnected.com/1940db638a6c

# ---------------------------------------------------------------------------
# Builder
# ---------------------------------------------------------------------------

FROM rust:1-buster AS builder

ARG git_url=https://github.com/bihealth/viguno.git
ARG git_treeish=v0.1.0

# Set initial workdir.
WORKDIR /usr/src
# Create blank project
RUN USER=root cargo new viguno
# We want dependencies cached, so copy those first.
COPY Cargo.toml Cargo.lock /usr/src/medium-rust-dockerize/
# Set the working directory
WORKDIR /usr/src/viguno
# Install build dependency protoc
RUN bash utils/install-protoc
# Install target platform (Cross-Compilation) --> Needed for Alpine
RUN rustup target add x86_64-unknown-linux-musl
# This is a dummy build to get the dependencies cached.
RUN cargo build --target x86_64-unknown-linux-musl --release
# Now copy in the rest of the sources
COPY src /usr/src/viguno/src/
## Touch main.rs to prevent cached release build
RUN touch /usr/src/viguno/src/main.rs
# This is the actual application build.
RUN cargo build --target x86_64-unknown-linux-musl --release

# ---------------------------------------------------------------------------
# Runtime
# ---------------------------------------------------------------------------

FROM alpine:3.16.0 AS runtime

# Copy application binary from builder image
COPY --from=builder \
    /usr/src/viguno/target/x86_64-unknown-linux-musl/release/viguno \
    /usr/local/bin

# Copy the entrypoint script and make it executable.
COPY utils/docker/entrypoint.sh /
RUN chmod a+rx /entrypoint.sh

# Set the entrypoint.
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
# Set port to expose
EXPOSE 8080
