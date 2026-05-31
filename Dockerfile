# syntax=docker/dockerfile:1
#
# Multi-stage build for the headless FH6 Telemetry server (fh6-tel-serve).
# Stage 1 builds the SvelteKit frontend, stage 2 compiles the Rust server with
# the frontend embedded (rust-embed), stage 3 is a minimal Debian runtime.
#
#   docker build -t fh6-tel-serve .
#   docker run -d --name fh6-tel \
#     -p 8080:8080 -p 20440:20440/udp \
#     -v fh6-tel-data:/data \
#     fh6-tel-serve --ip 0.0.0.0 --port 8080 --auth-token CHANGE_ME
#
# Built on Debian bullseye (glibc 2.31) for broad compatibility.

# ---------- Stage 1: build the SvelteKit frontend ----------
FROM node:20-bullseye AS frontend
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

# ---------- Stage 2: build the Rust server binary ----------
FROM rust:bullseye AS build
WORKDIR /app
COPY src-tauri ./src-tauri
# rust-embed reads ../build relative to the crate (src-tauri), so place the
# frontend output at /app/build.
COPY --from=frontend /app/build ./build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release \
      --manifest-path src-tauri/Cargo.toml \
      --no-default-features --features server --bin fh6-tel-serve \
      --target-dir /app/target \
 && cp /app/target/release/fh6-tel-serve /usr/local/bin/fh6-tel-serve \
 && strip /usr/local/bin/fh6-tel-serve

# ---------- Stage 3: minimal runtime ----------
FROM debian:bullseye-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# SQLite database + settings.json live under XDG_DATA_HOME/fh6-tel.
# Mount a volume at /data to persist sessions across container restarts.
ENV XDG_DATA_HOME=/data
RUN mkdir -p /data
VOLUME ["/data"]

COPY --from=build /usr/local/bin/fh6-tel-serve /usr/local/bin/fh6-tel-serve

# HTTP dashboard (TCP) and Forza telemetry ingest (UDP).
EXPOSE 8080/tcp
EXPOSE 20440/udp

ENTRYPOINT ["/usr/local/bin/fh6-tel-serve"]
# Default args; override at `docker run` to add --auth-token etc.
CMD ["--ip", "0.0.0.0", "--port", "8080"]
