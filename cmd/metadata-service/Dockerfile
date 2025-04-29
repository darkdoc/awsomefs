ARG METADATA_SERVICE_DIR=cmd/metadata-service

FROM rust:1.74 as builder
ARG METADATA_SERVICE_DIR

WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler
RUN cargo build --manifest-path ${METADATA_SERVICE_DIR}/Cargo.toml --release

FROM registry.redhat.io/ubi9/ubi-minimal
ARG METADATA_SERVICE_DIR
COPY --from=builder /app/${METADATA_SERVICE_DIR}/target/release/metadata-service /usr/bin/metadata-service
ENTRYPOINT ["metadata-service"]
