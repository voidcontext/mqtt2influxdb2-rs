{
  inputs.nixpkgs.url = "nixpkgs/release-24.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.crane = {
    url = "github:ipetkov/crane/v0.18.0";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    ...
  }: let
    mkMqtt2influxdb2 = import ./nix/mqtt2influxdb2.nix;

    outputs = flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
        craneLib = crane.mkLib pkgs;
        callPackage = pkgs.lib.callPackageWith (pkgs // {inherit craneLib;});
        mqtt2influxdb2 = callPackage mkMqtt2influxdb2 {};
      in {
        checks = mqtt2influxdb2.checks;

        packages.default = mqtt2influxdb2;

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.rust-analyzer
            pkgs.cargo-outdated
          ];
        };
      }
    );
  in
    outputs
    // {
      overlays.default = final: prev: {
        mqtt2influxdb2 = outputs.packages.${final.system}.default;
      };

      overlays.withHostPkgs = final: prev: let
        callPackage = final.lib.callPackageWith (final // {inherit crane;});
      in {
        mqtt2influxdb2 = callPackage mkMqtt2influxdb2 {};
      };
    };
}
