ARG RUST_VERSION=1.71.0

FROM rust:${RUST_VERSION}-slim-bullseye AS builder
WORKDIR /app
COPY . .
RUN \
  --mount=type=cache,target=/app/target/ \
  --mount=type=cache,target=/usr/local/cargo/registry/ \
  /bin/bash -c \
  'cargo build --locked --release --package namethat && \
  cp ./target/release/namethat /app'

FROM debian:bullseye-slim AS final
RUN adduser \
  --disabled-password \
  --gecos "" \
  --home "/nonexistent" \
  --shell "/sbin/nologin" \
  --no-create-home \
  --uid "10001" \
  appuser
COPY --from=builder /app/namethat /usr/local/bin
RUN chown appuser /usr/local/bin/namethat
# COPY --from=builder /app/config /opt/namethat/config
# RUN chown -R appuser /opt/namethat/config
USER appuser
WORKDIR /opt/namethat
ENTRYPOINT ["namethat"]
EXPOSE 3000/tcp