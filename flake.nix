{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flakebox = {
      url = "github:rustshop/flakebox";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, flakebox }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        flakeboxLib = flakebox.lib.${system} { };
        lib = pkgs.lib;
        stdenv = pkgs.stdenv;

        stdToolchains = flakeboxLib.mkStdToolchains { };

        rustSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = "fmo";
            path = ./.;
          };
          paths = [
            "Cargo.toml"
            "Cargo.lock"
            ".cargo"
            "fmo_api_types"
            "fmo_server"
          ];
        };

        packages =
          (flakeboxLib.craneMultiBuild { toolchains = stdToolchains; }) (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = "fmo_server";
                version = "0.1.0";
                src = rustSrc;
              });
            in
            rec {
              workspaceDeps = craneLib.buildWorkspaceDepsOnly { };
              workspaceBuild = craneLib.buildWorkspace {
                cargoArtifacts = workspaceDeps;
              };
              fmo_server = craneLib.buildPackage { };
              fmo_server_image = pkgs.dockerTools.buildLayeredImage {
                name = "fmo_server";
                contents = [ fmo_server pkgs.bash pkgs.coreutils ];
                config = {
                  Cmd = [
                    "${fmo_server}/bin/fmo_server"
                  ];
                };
              };

            });
      in
      {
        devShells = flakeboxLib.mkShells {
          nativeBuildInputs = [
            pkgs.postgresql
            # sqlite is used only for creating the dump file for migrating existing instances
            pkgs.sqlite
          ] ++ lib.optionals stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          shellHook = ''
            source scripts/pg_dev/env.sh
            echo "Type 'just pg_start' to start the $PGDATABASE database, use 'pg' to connect to it"
          '';
        };

        legacyPackages = packages;
        packages.default = packages.fedimint-observer;
      }
    );
}
