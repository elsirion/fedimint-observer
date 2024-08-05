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

        toolchains = flakeboxLib.mkFenixToolchain {
          components = [
            "rustc"
            "cargo"
            "clippy"
            "rust-analyzer"
            "rust-src"
          ];

          args = {
            nativeBuildInputs = with pkgs; [
              wasm-bindgen-cli
              wasm-pack
              trunk
              nodejs
              nodePackages.tailwindcss
            ];
          };
          targets = (pkgs.lib.getAttrs
            ([
              "default"
              "wasm32-unknown"
            ])
            (flakeboxLib.mkStdTargets { })
          );
        };

        rustSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = "flakebox-tutorial";
            path = ./.;
          };
          paths = [
            "fmo_api_types"
            "Cargo.toml"
            "Cargo.lock"
            ".cargo"
            "src"
            "schema"
          ];
        };

        packages =
          (flakeboxLib.craneMultiBuild { toolchains = toolchains; }) (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = "fedimint-observer";
                src = rustSrc;
              });
            in
            rec {
              workspaceDeps = craneLib.buildWorkspaceDepsOnly { };
              workspaceBuild = craneLib.buildWorkspace {
                cargoArtifacts = workspaceDeps;
              };
              fedimint-observer = craneLib.buildPackage { };
              fedimint-observer-image = pkgs.dockerTools.buildLayeredImage {
                name = "fedimint-observer";
                contents = [ fedimint-observer pkgs.bash pkgs.coreutils ];
                config = {
                  Cmd = [
                    "${fedimint-observer}/bin/fedimint-observer"
                  ];
                };
              };

            });
      in
      {
        devShells = flakeboxLib.mkShells {
          toolchain = toolchains;

          nativeBuildInputs = [
            pkgs.postgresql
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
