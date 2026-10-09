FROM rust:1.88-bookworm AS builder
WORKDIR /build
COPY Cargo.toml ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 cybcore
COPY --from=builder /build/target/release/cybcore-server /usr/local/bin/cybcore-server
USER 10001
EXPOSE 8080
CMD ["cybcore-server"]
