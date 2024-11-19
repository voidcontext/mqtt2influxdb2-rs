{
  lib,
  stdenv,
  craneLib,
  cmake,
  darwin,
  libiconv,
  ...
}: let
  src = ../.;

  commonArgs = {
    inherit src;
    buildInputs =
      [cmake]
      ++ (lib.optionals stdenv.isDarwin [
        darwin.apple_sdk_11_0.frameworks.Security
        libiconv
      ]);
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  mqtt2influxdb2 = craneLib.buildPackage (commonArgs
    // {
      doCheck = false;

      # # Shell completions
      # COMPLETIONS_TARGET = "target/";
      # nativeBuildInputs = [installShellFiles];
      # postInstall = ''
      #   installShellCompletion --bash target/mqtt2influxdb2.bash
      #   installShellCompletion --fish target/mqtt2influxdb2.fish
      #   installShellCompletion --zsh  target/_mqtt2influxdb2
      # '';

      passthru.checks = {
        inherit mqtt2influxdb2;

        mqtt2influxdb2-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -Dwarnings -W clippy::pedantic -A clippy::missing-errors-doc -A clippy::missing-panics-doc";
          });

        mqtt2influxdb2-doc = craneLib.cargoDoc (commonArgs
          // {
            inherit cargoArtifacts;
          });

        # Check formatting
        mqtt2influxdb2-fmt = craneLib.cargoFmt {
          inherit src;
        };

        # # Audit dependencies
        # mqtt2influxdb2-audit = craneLib.cargoAudit {
        #   inherit src advisory-db;
        # };

        # # Audit licenses
        # mqtt2influxdb2-deny = craneLib.cargoDeny {
        #   inherit src;
        # };

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on `mqtt2influxdb2` if you do not want
        # the tests to run twice
        mqtt2influxdb2-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            # skip integration tests
            cargoNextestExtraArgs = "-E 'not kind(test)'";
          });
      };
    });
in
  mqtt2influxdb2
