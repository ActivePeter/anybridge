# 使用官方的 Ubuntu 18.04 镜像作为基础镜像
FROM ubuntu:18.04

# 更新包列表并安装必要的依赖项
RUN apt-get update && apt-get install -y \
    curl build-essential\
    && rm -rf /var/lib/apt/lists/*

# 下载并安装 Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 

# 设置工作目录
WORKDIR /app

# # 拷贝当前目录下的所有文件到容器的/app目录下
# COPY . /app

# 设置环境变量以指定默认的 Rust 版本（可选）
ENV PATH="/root/.cargo/bin:$PATH"

# 设置容器启动后执行的命令
# RUN cargo install bottom --locked