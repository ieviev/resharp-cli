{
  lib,
  installShellFiles,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "resharp";
  version = "0.1.0";
  src = lib.cleanSource ./.;
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    ln -s $out/bin/resharp "$out/bin/re#"
    installShellCompletion --cmd resharp \
      --bash <($out/bin/resharp --completions bash) \
      --zsh <($out/bin/resharp --completions zsh) \
      --fish <($out/bin/resharp --completions fish)
  '';

  meta = {
    description = "grep tool powered by the resharp regex engine with intersection, complement, and lookarounds";
    license = lib.licenses.mit;
    mainProgram = "resharp";
  };
}
