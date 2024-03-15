###################################################################################################
#
# To build the rust components with this flake, run:
# $ nix build .#cargoDeps
# set `cargoHash` below to the result of the build
# then
# $ nix build .#zksync_server
# or
# $ nix build .#zksync_server.contract_verifier
# $ nix build .#zksync_server.external_node
# $ nix build .#zksync_server.server
# $ nix build .#zksync_server.snapshots_creator
# $ nix build .#zksync_server.block_reverter
#
# To enter the development shell, run:
# $ nix develop --impure
#
# To vendor the dependencies manually, run:
# $ nix shell .#cargo-vendor -c cargo vendor --no-merge-sources
#
###################################################################################################
{
  description = "zkSync-era";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        cargoHash = "sha256-kG5CAws05cnZGhQC/Q35FaNK2BlCMBNI3MfbV1vqiso=";

        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };

        # patched version of cargo to support `cargo vendor` for vendoring dependencies
        # see https://github.com/matter-labs/zksync-era/issues/1086
        # used as `cargo vendor --no-merge-sources`
        cargo-vendor = pkgs.rustPlatform.buildRustPackage rec {
          pname = "cargo-vendor";
          version = "0.78.0";
          src = pkgs.fetchFromGitHub {
            owner = "haraldh";
            repo = "cargo";
            rev = "3ee1557d2bd95ca9d0224c5dbf1d1e2d67186455";
            hash = "sha256-A8xrOG+NmF8dQ7tA9I2vJSNHlYxsH44ZRXdptLblCXk=";
          };
          doCheck = false;
          cargoHash = "sha256-LtuNtdoX+FF/bG5LQc+L2HkFmgCtw5xM/m0/0ShlX2s=";
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.rustPlatform.bindgenHook
          ];
          buildInputs = [
            pkgs.openssl
          ];
        };

        # custom import-cargo-lock to import Cargo.lock file and vendor dependencies
        # see https://github.com/matter-labs/zksync-era/issues/1086
        import-cargo-lock = { lib, cacert, runCommand }: { src, cargoHash ? null } @ args:
          runCommand "import-cargo-lock"
            {
              inherit src;
              nativeBuildInputs = [ cargo-vendor cacert ];
              preferLocalBuild = true;
              outputHashMode = "recursive";
              outputHashAlgo = "sha256";
              outputHash = if cargoHash != null then cargoHash else lib.fakeSha256;
            }
            ''
              mkdir -p $out/.cargo
              mkdir -p $out/cargo-vendor-dir

              HOME=$(pwd)
              pushd ${src}/prover
              HOME=$HOME cargo vendor --no-merge-sources $out/cargo-vendor-dir > $out/.cargo/config
              sed -i -e "s#$out#import-cargo-lock#g" $out/.cargo/config
              cp $(pwd)/Cargo.lock $out/Cargo.lock
              popd
            ''
        ;
        cargoDeps = pkgs.buildPackages.callPackage import-cargo-lock { } { inherit src; inherit cargoHash; };

        rustVersion = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;

        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;

        rustPlatform = (pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
          inherit stdenv;
        });

        hardeningEnable = [ "fortify3" "pie" "relro" ];

        src = with pkgs.lib.fileset; toSource {
          root = ./.;
          fileset = unions [
            ./Cargo.lock
            ./Cargo.toml
            ./core
            ./sdk
            ./.github/release-please/manifest.json
            ./prover
          ];
        };

        prover = with pkgs; stdenv.mkDerivation {
          pname = "prover";
          version = "1.0.0";

          updateAutotoolsGnuConfigScriptsPhase = ":";

          nativeBuildInputs = [
            pkg-config
            rustPlatform.bindgenHook
            rustPlatform.cargoSetupHook
            rustPlatform.cargoBuildHook
            rustPlatform.cargoInstallHook
          ];

          buildInputs = [
            libclang
            openssl
          ];

          inherit src;
          sourceRoot = "${src.name}/prover";
          cargoBuildFlags = "--all";
          cargoBuildType = "release";
          inherit cargoDeps;

          inherit hardeningEnable;

          outputs = [
            "out"
            "proof_fri_compressor"
            "prover_fri"
            "prover_fri_gateway"
            "witness_generator"
            "witness_vector_generator"
          ];

          postInstall = ''
            for i in $outputs; do
              [[ $i == "out" ]] && continue
              mkdir -p "''${!i}/bin"
              if [[ -e "$out/bin/zksync_$i" ]]; then
                mv "$out/bin/zksync_$i" "''${!i}/bin"
              else
                mv "$out/bin/$i" "''${!i}/bin"
              fi
            done
          '';
        };
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages = {
          inherit prover;
          default = prover;
          inherit cargo-vendor;
          inherit cargoDeps;
        };

        devShells = with pkgs;{
          default = pkgs.mkShell.override { inherit stdenv; } {
            inputsFrom = [ zksync_server ];

            packages = [
              docker-compose
              nodejs
              yarn
              axel
              postgresql
              python3
              solc
              sqlx-cli
            ];

            inherit hardeningEnable;

            shellHook = ''
              export ZKSYNC_HOME=$PWD
              export PATH=$ZKSYNC_HOME/bin:$PATH
            '';

            # hardhat solc requires ld-linux
            # Nixos has to fake it with nix-ld
            NIX_LD_LIBRARY_PATH = lib.makeLibraryPath [ ];
            NIX_LD = builtins.readFile "${clangStdenv.cc}/nix-support/dynamic-linker";
          };
        };
      });
}

