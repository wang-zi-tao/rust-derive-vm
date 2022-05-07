{ pkgs ? import <nixpkgs> { } }:
let
  fenix = import
    (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz")
    { };
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    llvmPackages_10.llvm
    libffi
    libffi.dev
    libxml2
    glibc
    pkg-config
    (fenix.fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = "sha256-M3IikryuwcddYfhuifWq02PpjhVF4epZuhP1uAEgE6Q=";
    })
  ];
  LLVM_SYS_100_PREFIX = "${pkgs.llvmPackages_10.llvm}";
  RUST_LOG = "trace";
  # LD_PRELOAD="${pkgs.glibc}/lib";
} 

