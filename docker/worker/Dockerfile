FROM nixos/nix:latest
RUN echo 'experimental-features = nix-command flakes' >> /etc/nix/nix.conf

# Install claude-code
RUN NIXPKGS_ALLOW_UNFREE=1 nix profile install --impure nixpkgs#claude-code

# Install git and standard tools
RUN nix profile install nixpkgs#git nixpkgs#coreutils nixpkgs#bash nixpkgs#gnugrep nixpkgs#findutils

# Build project binaries from the flake
COPY . /src/
RUN cd /src && nix build .#packages.$(nix eval --raw --expr 'builtins.currentSystem').default --out-link /tmp/clc-build && \
    cp /tmp/clc-build/bin/* /usr/local/bin/ && \
    rm -rf /src

ENV HOME=/root
CMD ["bash"]
