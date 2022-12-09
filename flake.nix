{
  # Use the current stable release of nixpkgs
  inputs.nixpkgs.url = "nixpkgs/release-22.05";

  # flake-utils helps removing some boilerplate
  inputs.flake-utils.url = "github:numtide/flake-utils";

  # Oxalica's rust overlay to easyly add rust build targets using rustup from nix
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  inputs.crane.url = "github:ipetkov/crane";
  inputs.crane.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, crane, ... }@inputs: inputs.flake-utils.lib.eachDefaultSystem (system:
    let
      overlays = [ inputs.rust-overlay.overlays.default ];

      pkgs = import inputs.nixpkgs { inherit system overlays; };

      rust = pkgs.rust-bin.stable."1.61.0".default;

      nativeBuildInputs = with pkgs.lib;
        (optional pkgs.stdenv.isLinux pkgs.pkg-config) ++ [pkgs.cmake];

      buildInputs = with pkgs.lib;
        (optional pkgs.stdenv.isLinux pkgs.openssl) ++
        (optional (system == "x86_64-darwin")
          pkgs.darwin.apple_sdk.frameworks.Security);

      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust;

      commonArgs = {
        src = craneLib.cleanCargoSource ./.;

        inherit buildInputs nativeBuildInputs;
      };


      mqtt2influxdb2-deps = craneLib.buildDepsOnly (commonArgs // {
        pname = "mqtt2influxdb2-deps";
      });

      mqtt2influxdb2 = craneLib.buildPackage {
        src = craneLib.cleanCargoSource ./.;
        pname = "mqtt2influxdb";
        version = "0.1.0";
        cargoArtifacts = mqtt2influxdb2-deps;
        preCheck = ''
          cargo fmt --check
          cargo clippy  -- -W clippy::pedantic -A clippy::missing-errors-doc -A clippy::missing-panics-doc
        '';

        inherit buildInputs nativeBuildInputs;
      };
    in
    rec {

      checks = {
        inherit mqtt2influxdb2;
      };

      packages.default = mqtt2influxdb2;

      devShells.default = pkgs.mkShell {
        buildInputs = nativeBuildInputs ++ buildInputs ++ [
          rust
          pkgs.cargo-outdated
          pkgs.cargo-watch
          pkgs.cargo-bloat
          pkgs.cargo-udeps
          pkgs.rust-analyzer
          pkgs.rustfmt
          pkgs.nixpkgs-fmt
        ];
      };
    }
  );
}
