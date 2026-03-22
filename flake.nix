{
  description = "why-not-tape Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        src = craneLib.cleanCargoSource ./.;

        # Build dependencies-only derivation (cached separately)
        cargoArtifacts = craneLib.buildDepsOnly { inherit src; };

        package = craneLib.buildPackage {
          inherit src cargoArtifacts;
        };
      in
      {
        packages.default = package;

        apps.default = {
          type = "app";
          program = "${package}/bin/why-not-tape";
        };

        checks = {
          inherit package;
          clippy = craneLib.cargoClippy {
            inherit src cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          };
          fmt = craneLib.cargoFmt { inherit src; };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ package ];
          buildInputs = [ rustToolchain pkgs.pkg-config ];
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      });
}