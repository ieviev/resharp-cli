{
  description = "resharp - grep powered by the RE# regex engine";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      overlay = final: prev: {
        resharp = final.callPackage ./resharp.nix { };
      };
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f {
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          }
        );
    in
    {
      overlays.default = overlay;
      packages = forAllSystems (
        { pkgs }:
        {
          default = pkgs.resharp;
          aarch64-linux = pkgs.pkgsCross.aarch64-multiplatform.callPackage ./resharp.nix { };
          windows = pkgs.pkgsCross.mingwW64.callPackage ./resharp.nix { };
        }
      );

      devShells = forAllSystems (
        { pkgs }:
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              ripgrep
              hyperfine
            ];
          };
        }
      );
    };
}
