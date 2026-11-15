FROM rust:alpine AS builder
RUN apk add --no-cache upx
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release
RUN upx --best --lzma target/release/ddnsflare


FROM scratch AS release
COPY --from=builder /build/target/release/ddnsflare /ddnsflare
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY api_key.env /api_key.env
ENTRYPOINT ["/ddnsflare", "/api_key.env"]
