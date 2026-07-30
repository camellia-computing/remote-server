# syntax=docker/dockerfile:1.7

FROM rust:1.97.1-bookworm@sha256:77fac8b98f9f46062bb680b6d25d5bcaabfc400143952ebc572e924bcbedc3fa AS builder

WORKDIR /build
COPY . .
RUN cargo build --locked --release --bins \
    && install -Dm0755 target/release/camellia-remote-identity /out/camellia-remote-identity \
    && install -Dm0755 target/release/camellia-remote-relay /out/camellia-remote-relay \
    && install -Dm0755 target/release/camellia-remote-utils /out/camellia-remote-utils

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818

ARG VERSION=0.1.0
ARG REVISION=unknown
ARG SOURCE_URL
RUN test -n "$SOURCE_URL"
LABEL org.opencontainers.image.title="Camellia Remote Server" \
      org.opencontainers.image.description="Identity, rendezvous and relay services for Camellia Remote" \
      org.opencontainers.image.version="$VERSION" \
      org.opencontainers.image.revision="$REVISION" \
      org.opencontainers.image.source="$SOURCE_URL" \
      org.opencontainers.image.licenses="AGPL-3.0-only" \
      org.opencontainers.image.vendor="Camellia Computing"

RUN groupadd --gid 10001 camellia \
    && useradd --uid 10001 --gid camellia --home-dir /var/lib/camellia-remote \
      --create-home --shell /usr/sbin/nologin camellia \
    && chmod 0700 /var/lib/camellia-remote

COPY --from=builder /out/ /usr/local/bin/

USER 10001:10001
WORKDIR /var/lib/camellia-remote
ENV HOME=/var/lib/camellia-remote

EXPOSE 21115/tcp 21116/tcp 21116/udp 21117/tcp 21118/tcp 21119/tcp
VOLUME ["/var/lib/camellia-remote"]

CMD ["camellia-remote-identity"]
