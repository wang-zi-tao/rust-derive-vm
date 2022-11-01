{
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.fenix.url = "github:nix-community/fenix";
  outputs = inputs@{ self, nixpkgs, fenix, flake-utils }:

    flake-utils.lib.eachDefaultSystem
      (system:
        let
          # fenix = inputs.fenix.packages.${system};
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; with fenix.packages.${system};[
              pkg-config
              llvmPackages_10.llvm
              libffi
              libffi.dev
              libxml2
              glibc
              pkg-config
              lua
              libcxx
              cargo-expand
              (fenix.packages.${system}.fromToolchainFile {
                file = ./rust-toolchain.toml;
                sha256 = "sha256-0pVbf/D0mM9M4qrVGxGMOfvur+Z/YmLqOvlbl7Ws3pU=";
              })
            ];
            LLVM_SYS_100_PREFIX = "${pkgs.llvmPackages_10.llvm}";
            RUST_LOG = "trace";
            RUSTFLAGS = "-g";
            # ENABLE_MACRO_CACHE = "1";
            # LD_PRELOAD="${pkgs.glibc}/lib";
          };
        }
      );
}
