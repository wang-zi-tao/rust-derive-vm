{ pkgs ? import <nixpkgs> { } }:
let
  fenix = import
    (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz")
    { };
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    llvmPackages_10.llvm
    libffi
    libxml2
    glibc
    pkg-config
    (fenix.fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = "sha256-x4VxrM7k/pHiIDCS0M6lBqsdezuDC605U6Lwz31qcGU=";
    })
  ];
  LLVM_SYS_100_PREFIX = "${pkgs.llvmPackages_10.llvm}";
  # LD_PRELOAD="${pkgs.glibc}/lib";
}
