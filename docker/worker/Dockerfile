FROM rust:1.88-bookworm

# Install claude-code via npm
RUN apt-get update && apt-get install -y nodejs npm git && \
    npm install -g @anthropic-ai/claude-code && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Build project binaries from source
COPY . /src/
RUN cd /src && cargo build --release --workspace && \
    cp target/release/clc target/release/tisket target/release/missouri /usr/local/bin/ && \
    rm -rf /src

ENV HOME=/root
CMD ["bash"]
