{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, rust-overlay, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        pallas-scripts = with pkgs;
          [
            (writeShellScriptBin "pallas-test" ''
              ${cargo-watch}/bin/cargo-watch watch \
                -s "${cargo-nextest}/bin/cargo-nextest nextest run"
            '')
          ];
      in {
        devShell = pkgs.mkShell {
          packages = with pkgs; [
            cargo-insta
            cargo-nextest
            cargo-watch

            pallas-scripts

            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "clippy" "rustfmt" ];
            })
          ];
        };
      });
}
