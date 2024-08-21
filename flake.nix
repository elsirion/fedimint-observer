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
            name = "fmo";
            path = ./.;
          };
          paths = [
            "Cargo.toml"
            "Cargo.lock"
            ".cargo"
            "fmo_api_types"
            "fmo_server"
            "fmo_frontend"
            "tailwind.config.js"
          ];
        };

        nativePackages =
          (flakeboxLib.craneMultiBuild { toolchains = { default = toolchains; }; }) (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = "fmo_server";
                version = "0.1.0";
                src = rustSrc;
                cargoExtraArgs = "--package=fmo_server";
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

        wasmPackages =
          let
            craneLib = (flakeboxLib.mkStdToolchains { }).wasm32-unknown.craneLib;

            wasmArgs = {
              src = rustSrc;

              cargoExtraArgs = "--package=fmo_frontend";
              trunkIndexPath = "fmo_frontend/index.html";
              strictDeps = true;

              pname = "fmo_frontend";
              version = "0.1.0";

              # Specify the wasm32 target
              CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
              RUSTFLAGS = "--cfg=web_sys_unstable_apis";
            };

            cargoArtifactsWasm = craneLib.buildDepsOnly (wasmArgs // {
              doCheck = false;
            });
          in
          {
            fmo_frontend = api: craneLib.buildTrunkPackage (wasmArgs // {
              nativeBuildInputs = with pkgs; [
                wasm-pack
                nodejs
                binaryen
                nodePackages.tailwindcss
              ];

              FMO_API_SERVER = api;

              wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
                version = "0.2.92";
                hash = "sha256-1VwY8vQy7soKEgbki4LD+v259751kKxSxmo/gqE6yV0=";
                cargoHash = "sha256-aACJ+lYNEU8FFBs158G1/JG8sc6Rq080PeKCMnwdpH0=";
              };
            });
          };
      in
      {
        devShells = flakeboxLib.mkShells {
          toolchain = toolchains;

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

          RUSTFLAGS = "--cfg=web_sys_unstable_apis";
        };

        legacyPackages = nativePackages // wasmPackages;
        packages.default = nativePackages.fmo_server;
      }
    );
}
