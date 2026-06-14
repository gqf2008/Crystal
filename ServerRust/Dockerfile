# Crystal Server — Phase 2.1: 生产级 Docker 镜像
# 多阶段构建:builder 编译 → slim 运行

# ===== Stage 1: Builder =====
FROM rust:1.82-bookworm AS builder

WORKDIR /build

# 安装 SQLite 开发库(sqlx 编译需要)
RUN apt-get update && apt-get install -y libsqlite3-dev pkg-config && rm -rf /var/lib/apt/lists/*

# 先拷贝 SharedRust(依赖最底层)
COPY SharedRust/ /build/SharedRust/

# 拷贝 ServerRust
COPY ServerRust/ /build/ServerRust/

# 编译 release
WORKDIR /build/ServerRust
RUN cargo build --release --bin mir2_server

# ===== Stage 2: Runtime =====
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 拷贝二进制
COPY --from=builder /build/ServerRust/target/release/mir2_server /app/mir2_server

# 拷贝配置 + 数据目录(容器启动后需要挂载 Data/ 和 Daneo1989/)
COPY ServerRust/config/ /app/config/

# 暴露游戏端口 + admin health check 端口
EXPOSE 7000 7001

# 环境变量
ENV RUST_LOG=info

# 数据卷(持久化 DB + 游戏资源)
VOLUME ["/app/data", "/app/Data", "/app/Daneo1989"]

# 启动
ENTRYPOINT ["/app/mir2_server"]
CMD ["config/server.toml"]
