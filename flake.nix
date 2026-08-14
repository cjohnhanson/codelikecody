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
              || (builtins.match ".*\\.missouri.*" path != null)
              || (builtins.match ".*/docs$" path != null)
              || (builtins.match ".*/docs/.*" path != null)
              || (builtins.match ".*/skills$" path != null)
              || (builtins.match ".*/skills/.*" path != null);
          };

          commonArgs = {
            pname = "clc";
            version = "0.2.0";
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
              # Man pages and shell completions come from the built
              # missouri binary itself, so they always match the real CLI.
              postInstall = ''
                mkdir -p $out/share/man/man1
                $out/bin/missouri gen-man $out/share/man/man1
                mkdir -p $out/share/zsh/site-functions
                mkdir -p $out/share/bash-completion/completions
                mkdir -p $out/share/fish/vendor_completions.d
                $out/bin/missouri gen-completions zsh > $out/share/zsh/site-functions/_missouri
                $out/bin/missouri gen-completions bash > $out/share/bash-completion/completions/missouri
                $out/bin/missouri gen-completions fish > $out/share/fish/vendor_completions.d/missouri.fish
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
