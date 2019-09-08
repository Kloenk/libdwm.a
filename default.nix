{
  system ? builtins.currentSystem,
  pkgs ? import <nixpkgs> { },
  ...
}:


with pkgs;
let
  #rust = rustChannelOfTargets "stable" null [ "x86_64-unknown-linux-gnu" ];
  rustPlatform = makeRustPlatform {
    rustc = cargo;
    cargo = cargo;
  };
in 
  rustPlatform.buildRustPackage rec {
    name = "rwm-${version}";
    version = "0.0.0";
    src = ./.;
    cargoSha256 = "13ryq99p7kl2finbxh9rhaijlz36s6vvqjvryyzlw7pfhsjxldc2";
    buildInputs = [ ];
    CARGO_HOME="$(mktemp -d cargo-home.XXX)";
    doCheck = false;
    installPhase = ''
      mkdir -p $out/usr/include;
      mkdir -p $out/lib/;
      cp target/release/librwm.a $out/lib/librwm.a
      cp target/release/librwm.so $out/lib/librwm.so
      cp target/rwm.h $out/usr/include/rwm.h
    '';

    meta = with lib; {
      homepage = https://github.com/kloenk/rwm;
      description = "rwm module for dwm";
      # TODO add license
      platforms = with stdenv.lib.platforms; all;
    };
  }

