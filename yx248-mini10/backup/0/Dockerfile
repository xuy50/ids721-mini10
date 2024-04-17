# Stage 1: Rust Build Stage
FROM rust:latest as builder

WORKDIR /app
COPY . .

# Install necessary build dependencies
RUN apt-get update && apt-get install -y curl build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install stable Rust and set as default
RUN rustup install stable
RUN rustup default stable

# Install formatting and linting tools
RUN rustup component add rustfmt clippy

# 构建你的应用
RUN cargo clean && cargo build --release

# 使用 Amazon Linux 2 image 作为 base image
# FROM amazonlinux:2
# FROM debian:buster-slim
FROM ubuntu:20.04

# 安装运行时依赖，包括 OpenSSL、wget、unzip 和 libgomp
# RUN yum install -y openssl-devel wget unzip libgomp libstdc++6 gcc-c++
# RUN apt-get update && apt-get install -y openssl wget unzip libgomp1 libstdc++6 gcc-c++
RUN apt-get update && apt-get install -y openssl wget unzip libgomp1 libstdc++6 gcc g++ libc6-dev

# 下载并解压 PyTorch 库
RUN wget https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-1.7.1%2Bcpu.zip
RUN unzip libtorch-cxx11-abi-shared-with-deps-1.7.1+cpu.zip -d /usr/local
RUN rm libtorch-cxx11-abi-shared-with-deps-1.7.1+cpu.zip

# 设置环境变量以包括库路径
ENV LD_LIBRARY_PATH=/usr/local/libtorch/lib:$LD_LIBRARY_PATH

# 复制编译好的可执行文件到新容器
COPY --from=builder /app/target/release/rust_lambda_hf /usr/local/bin/rust_lambda_hf

CMD ["rust_lambda_hf"]
