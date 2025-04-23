FROM rust:1.74 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM registry.redhat.io/ubi9/ubi-minimal
COPY --from=builder /app/target/release/fs-core /usr/bin/fs-core
ENTRYPOINT ["fs-core"]
