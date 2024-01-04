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
      pkgs = nixpkgs.legacyPackages.${system};

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
        ];
        nativeBuildInputs = [pkg-config];

        BINDGEN_EXTRA_CLANG_ARGS = ''-I"${libclang.lib}/lib/clang/16/include"'';
        LIBCLANG_PATH = lib.makeLibraryPath [libclang.lib];
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs
        // {
        });

      compressor = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          cargoExtraArgs = "--bin zksync_proof_fri_compressor";
        });

      prover = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          cargoExtraArgs = "--bin zksync_prover_fri";
        });
    in {
      packages.compressor = compressor;
      packages.prover = prover;
    });
}
