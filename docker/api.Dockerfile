# ---- Build de l'API Rust ----
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo test --release --locked \
 && cargo build --release --locked -p api \
 && cp target/release/allocation-api /allocation-api

# ---- Image finale minimale (sans shell), utilisateur non root ----
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /allocation-api /usr/local/bin/allocation-api
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
USER nonroot
HEALTHCHECK --interval=30s --timeout=3s --start-period=3s CMD ["/usr/local/bin/allocation-api", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/allocation-api"]
