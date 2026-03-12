{
  description = "resharp - grep powered by the RE# regex engine";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f {
        pkgs = nixpkgs.legacyPackages.${system};
      });
    in
    {
      packages = forAllSystems ({ pkgs }: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "resharp";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [ pkgs.installShellFiles ];

          postInstall = ''
            ln -s $out/bin/resharp "$out/bin/re#"
            installShellCompletion --cmd resharp \
              --bash <($out/bin/resharp --completions bash) \
              --zsh <($out/bin/resharp --completions zsh) \
              --fish <($out/bin/resharp --completions fish)
          '';

          meta = {
            description = "grep tool powered by the resharp regex engine with intersection, complement, and lookarounds";
            license = pkgs.lib.licenses.mit;
            mainProgram = "resharp";
          };
        };

        aarch64-linux = pkgs.pkgsCross.aarch64-multiplatform.rustPlatform.buildRustPackage {
          pname = "resharp";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            description = "grep tool powered by the resharp regex engine with intersection, complement, and lookarounds";
            license = pkgs.lib.licenses.mit;
            mainProgram = "resharp";
          };
        };

        windows = pkgs.pkgsCross.mingwW64.rustPlatform.buildRustPackage {
          pname = "resharp";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            description = "grep tool powered by the resharp regex engine with intersection, complement, and lookarounds";
            license = pkgs.lib.licenses.mit;
            mainProgram = "resharp";
          };
        };
      });

      devShells = forAllSystems ({ pkgs }: {
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
      });
    };
}
