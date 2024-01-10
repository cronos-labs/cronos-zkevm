{
  inputs.crane.url = "github:ipetkov/crane";
  inputs.fenix.url = "github:nix-community/fenix";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs = {
    crane,
    fenix,
    flake-utils,
    nixpkgs,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };

      inherit (pkgs.lib) cleanSourceWith;

      toolchain = with fenix.packages.${system};
        combine [
          default.rustc
          default.cargo
        ];
      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

      src = cleanSourceWith {
        src = ./.;
        filter = _: _: true;
      };

      commonArgs = with pkgs; {
        inherit src;

        strictDeps = true;

        cargoLock = ./prover/Cargo.lock;
        cargoToml = ./prover/Cargo.toml;
        postUnpack = ''
          cd $sourceRoot/prover
          sourceRoot="."
        '';

        buildInputs = [
          openssl
          rocksdb
          cudaPackages_12.cudatoolkit
        ];
        nativeBuildInputs = [pkg-config cmake];

        BINDGEN_EXTRA_CLANG_ARGS = ''-I"${libclang.lib}/lib/clang/16/include"'';
        LIBCLANG_PATH = lib.makeLibraryPath [libclang.lib];

        CUDAARCHS = "all";
        CUDA_PATH = cudaPackages_12.cudatoolkit;
        CUDA_VERSION = "12.0.1";
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs
        // {
        });

      prover = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          cargoExtraArgs = "--features gpu --bin zksync_prover_fri";
        });
    in {
      packages.prover = prover;
      devShell = with pkgs;
        mkShell {
          nativeBuildInputs = [fenix.packages.${system}.default.toolchain cmake pkg-config openssl rocksdb cudaPackages_12.cudatoolkit];

          BINDGEN_EXTRA_CLANG_ARGS = ''-I"${libclang.lib}/lib/clang/16/include"'';
          LIBCLANG_PATH = lib.makeLibraryPath [libclang.lib];

          CUDAARCHS = "all";
          CUDA_PATH = cudaPackages_12.cudatoolkit;
          CUDA_VERSION = "12.0.1";
        };
    });
}