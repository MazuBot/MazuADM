FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY . .
RUN cargo update home@0.5.12 --precise 0.5.9 && cargo build --release

FROM alpine:3.20
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /app/target/release/mazuadm-api /usr/local/bin/
COPY --from=builder /app/target/release/mazuadm-cli /usr/local/bin/
EXPOSE 3000
CMD ["mazuadm-api"]
