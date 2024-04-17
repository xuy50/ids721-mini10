# Stage 1: Rust Build Stage
FROM rust:1.77.2 as builder

WORKDIR /app
COPY . .

# Install necessary build dependencies & stable Rust and set as default
# RUN apt-get update && apt-get install -y curl build-essential && \
#     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
#     rustup install stable && \
#     rustup default stable && \
#     apt-get clean && \
#     rm -rf /var/lib/apt/lists/*

# ENV PATH="/root/.cargo/bin:${PATH}"

# Install formatting and linting tools
# RUN rustup component add rustfmt clippy

# Build application
RUN cargo clean && cargo build --release

# Stage 2: Amazon Linux Runtime Stage
FROM public.ecr.aws/lambda/provided:al2-x86_64

# Install dependencies
RUN yum install -y openssl openssl-devel wget unzip libgomp libstdc++ gcc-c++ && \
    yum clean all && \
    rm -rf /var/cache/yum
    # yum update -y && \

# Download and extract the latest PyTorch library suitable for CPU usage
RUN wget https://download.pytorch.org/libtorch/cu121/libtorch-cxx11-abi-shared-with-deps-2.2.0%2Bcu121.zip && \
    unzip libtorch-cxx11-abi-shared-with-deps-2.2.0+cu121.zip -d /usr/local && \
    rm libtorch-cxx11-abi-shared-with-deps-2.2.0+cu121.zip

# Set environment variables to include library paths
ENV LIBTORCH=/usr/local/libtorch
ENV LD_LIBRARY_PATH=/usr/local/libtorch/lib:$LD_LIBRARY_PATH

# Copy the compiled executable file to the new container
COPY --from=builder /app/target/release/rust_lambda_hf /usr/local/bin/rust_lambda

# Check if the executable file exists
RUN if [ -f /usr/local/bin/rust_lambda_hf_mini10 ]; then echo "Executable exists"; else echo "Executable does not exist"; exit 1; fi

CMD ["rust_lambda_hf"]
