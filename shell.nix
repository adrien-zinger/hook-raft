let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in
with nixpkgs;
stdenv.mkDerivation {
  name = "rust-env";
  buildInputs = [
    # Note: to use nightly, just replace `stable` with `nightly`
    latest.rustChannels.stable.rust
  ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
