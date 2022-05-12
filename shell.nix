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
    lua
    (fenix.fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = "sha256-CNMj0ouNwwJ4zwgc/gAeTYyDYe0botMoaj/BkeDTy4M=";
    })
  ];
  LLVM_SYS_100_PREFIX = "${pkgs.llvmPackages_10.llvm}";
  RUST_LOG = "trace";
  ENABLE_MACRO_CACHE = "1";
  # LD_PRELOAD="${pkgs.glibc}/lib";
} 

