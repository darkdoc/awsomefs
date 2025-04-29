ARG FS_CORE_DIR=cmd/fs-core

FROM rust:1.74 as builder
ARG FS_CORE_DIR

WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y libfuse-dev protobuf-compiler
RUN cargo build --manifest-path ${FS_CORE_DIR}/Cargo.toml --release

FROM registry.redhat.io/ubi9/ubi-minimal
ARG FS_CORE_DIR
RUN microdnf install -y fuse fuse-devel
COPY --from=builder /app/${FS_CORE_DIR}/target/release/fs-core /usr/bin/fs-core
ENTRYPOINT ["fs-core"]
