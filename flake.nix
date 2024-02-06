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
          default.cargo
          default.rustc
          default.rustfmt
        ];
      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

      bellman-cuda = with pkgs;
        gcc11Stdenv.mkDerivation {
          pname = "bellman-cuda";
          version = "1.0.0";

          src = fetchFromGitHub {
            owner = "matter-labs";
            repo = "era-bellman-cuda";
            rev = "b52127574e67f4c7938890eab0670ae75f49c22f";
            hash = "sha256-fACNERGEWpVzt2lcderix7Ybh80kvxqFBtbfDER7Erg=";
          };

          nativeBuildInputs = [
            cmake
            cudaPackages.autoAddOpenGLRunpathHook
            pkg-config
          ];

          buildInputs = [
            cudatoolkit
          ];

          CUDAToolkit_ROOT = cudatoolkit;

          postInstall = ''
            mkdir -p $out/src
            cp $src/src/bellman-cuda.h $out/src
          '';

          CMAKE_CUDA_ARCHITECTURES = 75;
        };

      src = cleanSourceWith {
        src = ./.;
        filter = _: _: true;
      };

      commonArgs = with pkgs; {
        stdenv = gcc11Stdenv;
        inherit src;

        strictDeps = true;

        cargoLock = ./prover/Cargo.lock;
        cargoToml = ./prover/Cargo.toml;
        postUnpack = ''
          cd $sourceRoot/prover
          sourceRoot="."
        '';

        nativeBuildInputs = [
          cmake
          cudaPackages.autoAddOpenGLRunpathHook
          pkg-config
        ];
        buildInputs = [
          openssl
          rocksdb
          cudatoolkit
        ];

        BINDGEN_EXTRA_CLANG_ARGS = ''-I"${libclang.lib}/lib/clang/16/include"'';
        LIBCLANG_PATH = lib.makeLibraryPath [libclang.lib];

        CUDAARCHS = "75";
        CUDA_PATH = cudatoolkit;
        CUDA_VERSION = "12.2";

        BELLMAN_CUDA_DIR = bellman-cuda;
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs
        // {
          cargoExtraArgs = "--features gpu --bin crane-dummy-zksync_prover_fri";
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
          nativeBuildInputs = [fenix.packages.${system}.default.toolchain pkg-config openssl];

          BINDGEN_EXTRA_CLANG_ARGS = ''-I"${libclang.lib}/lib/clang/16/include"'';
          LIBCLANG_PATH = lib.makeLibraryPath [libclang.lib];
        };
    });
}
