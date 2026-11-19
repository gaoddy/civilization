# room-service 运行镜像。构建上下文为仓库根：protocol crate 引用根 protocol/generated。
FROM rust:1.94.1-bookworm AS builder
WORKDIR /src
COPY server/ /src/server/
COPY protocol/ /src/protocol/
WORKDIR /src/server
RUN cargo build --locked --release -p room-service

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin room-service
COPY --from=builder /src/server/target/release/room-service /usr/local/bin/room-service
USER 10001
EXPOSE 8080
ENTRYPOINT ["room-service"]
