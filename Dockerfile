# Build a fully static musl binary; the builder does not become part of the image.
FROM rust:1.88-alpine AS builder

WORKDIR /build
RUN apk add --no-cache musl-dev
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release

FROM scratch

WORKDIR /app
COPY --from=builder /build/target/release/film-record-lite /usr/local/bin/film-record-lite

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/film-record-lite", "--healthcheck", "--port", "8000"]

ENTRYPOINT ["/usr/local/bin/film-record-lite"]
CMD ["--port", "8000"]
