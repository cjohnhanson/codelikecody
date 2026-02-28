{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flox.url = "github:flox/flox/latest";
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      flox,
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
              nativeCheckInputs = [
                flox.packages.${system}.default
              ];
              # The darwin nix sandbox forces HOME=/var/empty for child processes.
              # Flox needs a writable config dir. Set XDG dirs and FLOX_CONFIG_DIR
              # so flox init can write its config in the build tmpdir.
              checkPhase = ''
                tmpHome="$(mktemp -d)"
                export HOME="$tmpHome"
                export TMPDIR="''${TMPDIR:-/tmp}"
                export XDG_CONFIG_HOME="$tmpHome/.config"
                export XDG_CACHE_HOME="$tmpHome/.cache"
                export XDG_DATA_HOME="$tmpHome/.local/share"
                export XDG_STATE_HOME="$tmpHome/.local/state"
                export FLOX_CONFIG_DIR="$tmpHome/.config/flox"
                export FLOX_DISABLE_METRICS=true
                mkdir -p "$FLOX_CONFIG_DIR"
                echo "DEBUG: HOME=$HOME TMPDIR=$TMPDIR"
                cargo test --profile release --locked
              '';
            }
          );
        in
        {
          default = workspace;
          clc = workspace;
        }
      );

      devShells = forEachSystem (
        { pkgs, ... }:
        {
          default = pkgs.mkShell {
            buildInputs = [
              pkgs.rust-bin.stable.latest.default
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];
          };
        }
      );
    };
}
