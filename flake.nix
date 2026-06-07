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

        commonArgs = {
          inherit src;
          nativeBuildInputs = [ pkgs.pkg-config pkgs.llvmPackages.libclang ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          # v4l2-sys-mit bindgen needs videodev2.h (linux) and sys/time.h (glibc)
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linuxHeaders}/include -I${pkgs.glibc.dev}/include";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        package = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        packages.default = package;

        apps.default = {
          type = "app";
          program = "${package}/bin/why-not-tape";
        };

        checks = {
          inherit package;
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });
          fmt = craneLib.cargoFmt { inherit src; };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ package ];
          buildInputs = [ rustToolchain ];
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.linuxHeaders}/include -I${pkgs.glibc.dev}/include";
          # wayland and xkbcommon are dlopen'd at runtime by minifb
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
          ];
        };
      });
}
