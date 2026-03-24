FROM rust:1.88-bookworm AS builder

# Copy dependency manifests first (layer cached unless deps change)
COPY Cargo.toml Cargo.lock /src/
COPY clc/Cargo.toml /src/clc/
COPY clc-sdk/Cargo.toml /src/clc-sdk/
COPY claude-code/Cargo.toml /src/claude-code/
COPY missouri/Cargo.toml /src/missouri/
COPY tisket/Cargo.toml /src/tisket/
COPY mdstore/Cargo.toml /src/mdstore/
COPY almanac/Cargo.toml /src/almanac/
COPY zettel/Cargo.toml /src/zettel/

# Create stub sources so cargo can build deps
RUN for d in clc clc-sdk claude-code missouri tisket mdstore almanac zettel; do \
      mkdir -p /src/$d/src && echo "" > /src/$d/src/lib.rs; \
    done && echo 'fn main() {}' > /src/clc/src/main.rs

# Build deps only (cached by Depot unless Cargo.toml/Cargo.lock change)
RUN cd /src && cargo build --workspace 2>/dev/null || true

# Copy real source and build (only project crates recompile)
COPY . /src/
RUN cd /src && cargo build --workspace && \
    cp target/debug/clc target/debug/tisket target/debug/missouri /tmp/

# Runtime image — small, no Rust toolchain
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    nodejs npm git openssh-server curl ca-certificates && \
    npm install -g @anthropic-ai/claude-code && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /run/sshd && \
    ssh-keygen -A && \
    sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config && \
    sed -i 's/#PubkeyAuthentication yes/PubkeyAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/#GatewayPorts no/GatewayPorts yes/' /etc/ssh/sshd_config && \
    mkdir -p /root/.ssh && chmod 700 /root/.ssh

COPY --from=builder /tmp/clc /tmp/tisket /tmp/missouri /usr/local/bin/

ENV HOME=/root
CMD ["/usr/sbin/sshd", "-D", "-e"]
