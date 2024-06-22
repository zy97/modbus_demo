# # 使用官方 Rust 镜像作为构建阶段的基础镜像
# FROM rust:latest as builder

# RUN apt-get update && \
#     apt-get install -y \
#     build-essential \
#     gcc \
#     libssl-dev \
#     pkg-config \
#     && rm -rf /var/lib/apt/lists/*

# # 创建一个新的空目录用于存放构建输出
# WORKDIR /app

# # 将 Cargo.toml 和 Cargo.lock 拷贝到工作目录中
# COPY Cargo.toml Cargo.lock ./
# # 创建一个空的main.rs文件用于缓存依赖
# RUN mkdir src && echo "fn main() {}" > src/main.rs

# # 构建依赖项
# RUN cargo build --release || true

# # 复制项目的源文件
# COPY . .

# # 运行最终的构建命令
# RUN cargo build --release

# # 使用更小的基础镜像运行应用程序
# FROM alpine:3.20.0 as release

# # 将应用程序拷贝到最终镜像中
# COPY --from=builder /app/target/release/modbus /app/modbus

# # 设置默认命令
# ENTRYPOINT ["/app/modbus"]

# Build Stage
FROM rust:alpine as builder

# Encourage some layer caching here rather then copying entire directory that includes docs to builder container ~CMN
WORKDIR /app/modbus
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

# Release Stage
FROM alpine:3.20.0 as release

COPY --from=builder /app/modbus/target/release/modbus ./modbus

ENTRYPOINT [ "./modbus" ]
