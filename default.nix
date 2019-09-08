let 
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
in
with pkgs;
let
  rust = rustChannelOfTargets "stable" null [ "x86_64-unknown-linux-gnu" ];
  rustPlatform = makeRustPlatform {
    rustc = rust;
    cargo = rust;
  };
  rwm = rustPlatform.buildRustPackage rec {
    name = "rwm";
    version = "0.0.0";
    src = ./.;
    cargoSha256 = "13ryq99p7kl2finbxh9rhaijlz36s6vvqjvryyzlw7pfhsjxldc2";
    buildInputs = [ ];
    CARGO_HOME="$(mktemp -d cargo-home.XXX)";
    doCheck = false;
    installPhase = ''
      mkdir -p $out/lib;
      mkdir -p $out/inc;
      cp target/release/librwm.a $out/lib/librwm.a
      cp target/release/librwm.so $out/lib/librwm.so
      cp target/rwm.h $out/inc/rwm.h
    '';
  };
in {
  inherit rustPlatform rwm;
}
