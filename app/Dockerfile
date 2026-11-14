# syntax=docker/dockerfile:1
FROM rust:1-slim-bookworm AS build
WORKDIR /src

# Build dependencies first so they're cached unless Cargo.toml/Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs \
    && cargo build --release --locked && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# distroless has no shell or curl; the binary has its own `healthcheck` subcommand.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/shortener /app/shortener
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/app/shortener"]
