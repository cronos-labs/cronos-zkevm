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

      inherit (pkgs) stdenv;
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

        buildInputs = [
          openssl
          rocksdb
        ];
        nativeBuildInputs = [pkg-config];

        BINDGEN_EXTRA_CLANG_ARGS = ''-I"${libclang.lib}/lib/clang/16/include"'';
        LIBCLANG_PATH = lib.makeLibraryPath [libclang.lib];
      };

      prover = let
        commonArgs' =
          commonArgs
          // {
            cargoLock = ./prover/Cargo.lock;
            cargoToml = ./prover/Cargo.toml;
            postUnpack = ''
              cd $sourceRoot/prover
              sourceRoot="."
            '';
          };
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs'
          // {
          });
      in {
        compressor = craneLib.buildPackage (commonArgs'
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--bin zksync_proof_fri_compressor";
          });

        prover = craneLib.buildPackage (commonArgs'
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--bin zksync_prover_fri";
          });

        witness-generator = craneLib.buildPackage (commonArgs'
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--bin zksync_witness_generator";
          });

        witness-vector-generator = craneLib.buildPackage (commonArgs'
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--bin zksync_witness_vector_generator";
          });

        gateway = craneLib.buildPackage (commonArgs'
          // {
            inherit cargoArtifacts;

            cargoExtraArgs = "--bin zksync_prover_fri_gateway";
          });
      };

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs
        // {
        });

      contract-verifier = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          cargoExtraArgs = "--bin zksync_contract_verifier";
        });

      server = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          cargoExtraArgs = "--bin zksync_server";
        });

      prover-vk-setup-data-generator-server-fri-data = stdenv.mkDerivation {
        pname = "prover-vk-setup-data-generator-server-fri-data";
        version = "1.0.0";
        src = ./prover/vk_setup_data_generator_server_fri/data/.;
        dontBuild = true;
        dontUnpack = true;
        installPhase = ''
          mkdir -p $out/prover/vk_setup_data_generator_server_fri/data
          cp $src/* $out/prover/vk_setup_data_generator_server_fri/data
        '';
      };
    in {
      packages.compressor = prover.compressor;
      packages.gateway = prover.gateway;
      packages.prover = prover.prover;
      packages.server = prover.server;
      packages.witness-generator = prover.witness-generator;
      packages.witness-vector-generator = prover.witness-vector-generator;

      packages.contract-verifier = contract-verifier;

      packages.prover-vk-setup-data-generator-server-fri-data = prover-vk-setup-data-generator-server-fri-data;
    });
}
