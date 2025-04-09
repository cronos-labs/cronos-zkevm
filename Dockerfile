# Will work locally only after prior contracts build

ARG BASE_IMAGE_TAG

FROM ghcr.io/matter-labs/zksync-build-base:latest AS builder

WORKDIR /usr/src/zksync
COPY . .
RUN cargo build --manifest-path ./core/Cargo.toml --release --bin zksync_external_node --bin block_reverter

FROM ghcr.io/cronos-labs/zkevm-base-image:${BASE_IMAGE_TAG} AS base

FROM ghcr.io/matter-labs/zksync-runtime-base:latest

COPY --from=builder /usr/src/zksync/core/target/release/zksync_external_node /usr/bin
COPY --from=builder /usr/src/zksync/core/target/release/block_reverter /usr/bin
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/bin
COPY --from=builder /usr/src/zksync/docker/external-node/entrypoint.sh /usr/bin
COPY --from=builder /usr/src/zksync/backups/generate_secrets.sh /scripts/

COPY --from=base /contracts/ /contracts/
COPY --from=base /etc/ /etc/
COPY --from=base /migrations/ /migrations/

RUN chmod +x /usr/bin/entrypoint.sh

ENTRYPOINT [ "sh", "/usr/bin/entrypoint.sh"]