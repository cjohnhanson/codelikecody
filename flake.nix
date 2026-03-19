{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      ...
    }:
    let
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs
          [
            "aarch64-darwin"
            "x86_64-linux"
            "aarch64-linux"
            "x86_64-darwin"
          ]
          (
            system:
            f {
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
              };
              inherit system;
            }
          );
    in
    {
      packages = forEachSystem (
        { pkgs, system }:
        let
          toolchain = pkgs.rust-bin.stable.latest.default;
          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

          # cleanCargoSource strips non-Rust files. Test fixtures need .missouri/
          # dirs, .yml configs, .txt data files, etc. Filter from raw source.
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              (craneLib.filterCargoSources path type)
              || (builtins.match ".*tests/fixtures/.*" path != null)
              || (builtins.match ".*tests/missouri/.*" path != null)
              || (builtins.match ".*\\.yml$" path != null)
              || (builtins.match ".*\\.yaml$" path != null)
              || (builtins.match ".*\\.txt$" path != null)
              || (builtins.match ".*\\.missouri.*" path != null);
          };

          commonArgs = {
            pname = "clc";
            version = "0.1.0";
            inherit src;
            strictDeps = true;
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          workspace = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              nativeCheckInputs = with pkgs; [
                # Packages needed by missouri test fixtures
                python3
                uv
                duckdb
                cargo
                rustc
                coreutils
                git
                jq
              ];
              # MISSOURI_SANDBOX=preinstalled tells missouri to skip `nix shell`
              # wrapping — the packages above are already on PATH via
              # nativeCheckInputs.
              checkPhase = ''
                tmpHome="$(mktemp -d)"
                export HOME="$tmpHome"
                export MISSOURI_SANDBOX=preinstalled
                cargo test --profile release --locked
              '';
            }
          );
          # OCI image for missouri test sandboxes (always aarch64-linux,
          # since microsandbox runs Linux microVMs via libkrun).
          # Build with: nix build .#missouri-base-image
          linuxPkgs = import nixpkgs { system = "aarch64-linux"; };
          missouri-base-image = linuxPkgs.dockerTools.buildLayeredImage {
            name = "missouri-base";
            tag = "latest";
            contents = with linuxPkgs; [
              # Shell and core utilities
              bash
              coreutils
              gnugrep
              gnused
              findutils
              # Network tools
              curl
              cacert
              mitmproxy
              # Version control
              git
            ];
            config = {
              Cmd = [ "${linuxPkgs.bash}/bin/bash" ];
              Env = [
                "SSL_CERT_FILE=${linuxPkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                "PATH=/usr/local/bin:/usr/bin:/bin:${
                  linuxPkgs.lib.makeBinPath (
                    with linuxPkgs;
                    [
                      bash
                      coreutils
                      gnugrep
                      gnused
                      findutils
                      curl
                      git
                      mitmproxy
                    ]
                  )
                }"
              ];
            };
          };
        in
        {
          default = workspace;
          clc = workspace;
          inherit missouri-base-image;
        }
      );

      devShells = forEachSystem (
        { pkgs, ... }:
        {
          default = pkgs.mkShell {
            buildInputs = [
              pkgs.rust-bin.stable.latest.default
              pkgs.git
              pkgs.jq
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];
          };
        }
      );
    };
}
