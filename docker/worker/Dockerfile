FROM rust:1.88-bookworm

# Install claude-code via npm, openssh-server for SSH workspace
RUN apt-get update && apt-get install -y nodejs npm git openssh-server && \
    npm install -g @anthropic-ai/claude-code && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Configure sshd
RUN mkdir -p /run/sshd && \
    ssh-keygen -A && \
    sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config && \
    sed -i 's/#PubkeyAuthentication yes/PubkeyAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/#GatewayPorts no/GatewayPorts yes/' /etc/ssh/sshd_config && \
    mkdir -p /root/.ssh && chmod 700 /root/.ssh

# Build project binaries from source
COPY . /src/
RUN cd /src && cargo build --release --workspace && \
    cp target/release/clc target/release/tisket target/release/missouri /usr/local/bin/ && \
    rm -rf /src

ENV HOME=/root
CMD ["/usr/sbin/sshd", "-D", "-e"]
