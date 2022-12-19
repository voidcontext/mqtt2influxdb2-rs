{

  inputs.nru.url = "github:voidcontext/nix-rust-utils";
  outputs = { nru, ...}: 
    nru.lib.mkOutputs {
      src = ./.;
    };
}
