FROM rust:1.88-bookworm AS builder

# Copy dependency manifests first (layer cached unless deps change).
# Build at /project so cargo fingerprints match the worker's checkout path.
COPY Cargo.toml Cargo.lock /project/
COPY clc/Cargo.toml /project/clc/
COPY clc-sdk/Cargo.toml /project/clc-sdk/
COPY claude-code/Cargo.toml /project/claude-code/
COPY missouri/Cargo.toml /project/missouri/
COPY tisket/Cargo.toml /project/tisket/
COPY mdstore/Cargo.toml /project/mdstore/
COPY almanac/Cargo.toml /project/almanac/
COPY zettel/Cargo.toml /project/zettel/

# Create stub sources so cargo can build deps
RUN for d in clc clc-sdk claude-code missouri tisket mdstore almanac zettel; do \
      mkdir -p /project/$d/src && echo "" > /project/$d/src/lib.rs; \
    done && echo 'fn main() {}' > /project/clc/src/main.rs

# Build deps only (cached by Depot unless Cargo.toml/Cargo.lock change)
RUN cd /project && cargo build --workspace 2>/dev/null || true

# Copy real source and build (only project crates recompile)
COPY . /project/
RUN cd /project && cargo build --workspace && \
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
    sed -i 's/#PermitUserEnvironment no/PermitUserEnvironment yes/' /etc/ssh/sshd_config && \
    mkdir -p /root/.ssh && chmod 700 /root/.ssh

COPY --from=builder /tmp/clc /tmp/tisket /tmp/missouri /usr/local/bin/

# Rust toolchain — workers need cargo to build and test.
# Copy from builder rather than installing from scratch.
COPY --from=builder /usr/local/cargo /usr/local/cargo
COPY --from=builder /usr/local/rustup /usr/local/rustup

# Pre-built dependency artifacts — workers get the project source via git
# sync at dispatch. Without these, every worker does a cold cargo build
# (~10 min). With them, only workspace crates recompile (~30s).
# Builder uses /project so fingerprints match the worker's checkout path.
COPY --from=builder /project/target /project/target
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Make ENV vars available to SSH sessions via PermitUserEnvironment.
# sshd reads ~/.ssh/environment and injects vars into every session.
RUN echo 'CARGO_HOME=/usr/local/cargo' > /root/.ssh/environment && \
    echo 'RUSTUP_HOME=/usr/local/rustup' >> /root/.ssh/environment && \
    echo 'PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin' >> /root/.ssh/environment

ENV HOME=/root
CMD ["/usr/sbin/sshd", "-D", "-e"]
