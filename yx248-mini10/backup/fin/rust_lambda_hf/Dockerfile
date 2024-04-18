# Stage 1: Rust Build Stage
FROM ghcr.io/cargo-lambda/cargo-lambda:latest as builder

WORKDIR /app
COPY . .

# Build application
RUN cargo clean
RUN cargo lambda build --release --compiler cargo --target x86_64-unknown-linux-gnu

# Stage 2: Amazon Linux Runtime Stage
FROM public.ecr.aws/lambda/provided:al2023-x86_64

WORKDIR /rust_lambda_hf_mini10

# Install dependencies
RUN dnf update -y && \
    dnf install -y openssl openssl-devel wget unzip libgomp libstdc++ gcc-c++ && \
    dnf clean all && \
    rm -rf /var/cache/dnf

# Download and extract the latest PyTorch library suitable for CPU usage
RUN wget https://download.pytorch.org/libtorch/cu117/libtorch-cxx11-abi-shared-with-deps-1.13.1%2Bcu117.zip
RUN unzip libtorch-cxx11-abi-shared-with-deps-1.13.1+cu117.zip -d /usr/local
RUN rm libtorch-cxx11-abi-shared-with-deps-1.13.1+cu117.zip
# RUN wget https://download.pytorch.org/libtorch/cu121/libtorch-cxx11-abi-shared-with-deps-2.2.0%2Bcu121.zip && \
#     unzip libtorch-cxx11-abi-shared-with-deps-2.2.0+cu121.zip -d /usr/local && \
#     rm libtorch-cxx11-abi-shared-with-deps-2.2.0+cu121.zip

# Set environment variables to include library paths
ENV LIBTORCH=/usr/local/libtorch
ENV LD_LIBRARY_PATH=/usr/local/libtorch/lib:$LD_LIBRARY_PATH

# Set rust-bert cache directory to /tmp
ENV RUSTBERT_CACHE=/tmp

# Copy the compiled executable file to the new container
COPY --from=builder /app/target/ ./

# Check if the executable file exists
RUN if [ -d /rust_lambda_hf_mini10/lambda/rust_lambda_hf/ ]; then echo "Directory exists"; else echo "Directory does not exist"; fi
RUN if [ -f /rust_lambda_hf_mini10/lambda/rust_lambda_hf/bootstrap ]; then echo "File exists"; else echo "File does not exist"; fi

ENTRYPOINT ["/rust_lambda_hf_mini10/lambda/rust_lambda_hf/bootstrap"]
