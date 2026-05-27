{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.crane.url = "github:ipetkov/crane";
  inputs.src.url = "github:cronos-labs/cronos-zkevm/cronos-v29.17.0";
  inputs.src.flake = false;

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, src }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        pkgsWithRust = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgsWithRust.rust-bin.nightly."2025-03-19".default;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        cargoSrc = lib.cleanSourceWith {
          src = "${src}/core";
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (lib.hasSuffix ".proto" path)
            || (lib.hasSuffix ".js" path)
            || (lib.hasSuffix ".ts" path)
            || (lib.hasSuffix ".map" path)
            || (lib.hasSuffix ".json" path)
            || (lib.hasInfix "/lib/dal/" path);
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustPlatform.bindgenHook
        ];

        buildInputs = with pkgs; [
          libclang
          openssl
          snappy
          lz4
          bzip2
          rocksdb_8_3
          postgresql
        ];

        commonArgs = {
          src = cargoSrc;
          inherit nativeBuildInputs buildInputs;
          pname = "zksync";
          version = "29.17.0";
          strictDeps = true;
          __noChroot = true;

          env = {
            OPENSSL_NO_VENDOR = "1";
            ROCKSDB_LIB_DIR = "${pkgs.rocksdb_8_3}/lib";
            ROCKSDB_INCLUDE_DIR = "${pkgs.rocksdb_8_3}/include";
            SNAPPY_LIB_DIR = "${pkgs.snappy}/lib";
            NIX_OUTPATH_USED_AS_RANDOM_SEED = "aaaaaaaaaa";
          };
        };

        zksyncBinaries = craneLib.buildPackage (commonArgs // {
          cargoExtraArgs = "--bin zksync_server --bin zksync_contract_verifier --bin snapshots_creator";
          doCheck = false;

          postPatch = ''
            mkdir -p "$TMPDIR/nix-vendor"
            cp -Lr "$cargoVendorDir" -T "$TMPDIR/nix-vendor"
            sed -i "s|$cargoVendorDir|$TMPDIR/nix-vendor/|g" "$TMPDIR/nix-vendor/config.toml"
            chmod -R +w "$TMPDIR/nix-vendor"
            cargoVendorDir="$TMPDIR/nix-vendor"
          '';
        });
      in {
        packages = {
          server = zksyncBinaries;
          contract-verifier = zksyncBinaries;
          snapshots-creator = zksyncBinaries;
          block_reverter = zksyncBinaries;
          default = zksyncBinaries;
        };
      }
    );
}
