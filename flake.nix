{
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.fenix.url = "github:nix-community/fenix";
  outputs = { self, nixpkgs, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in
        {
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              llvmPackages_10.llvm
              libffi
              libffi.dev
              libxml2
              glibc
              pkg-config
              lua
              (fenix.packages.${system}.fromToolchainFile {
                file = ./rust-toolchain.toml;
                sha256 = "sha256-CNMj0ouNwwJ4zwgc/gAeTYyDYe0botMoaj/BkeDTy4M=";
              })
            ];
            LLVM_SYS_100_PREFIX = "${pkgs.llvmPackages_10.llvm}";
            RUST_LOG = "trace";
            ENABLE_MACRO_CACHE = "1";
            # LD_PRELOAD="${pkgs.glibc}/lib";
          };
        }
      );
}
