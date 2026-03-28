{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      fenix,
      crane,
      treefmt-nix,
      ...
    }:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);

      perSystem = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          toolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-zC8E38iDVJ1oPIzCqTk/Ujo9+9kx9dXq7wAwPMpkpg0=";
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

          rustFlags = "-C link-arg=-Wl,-rpath,${pkgs.libmysqlclient}/lib/mariadb";

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              (craneLib.filterCargoSources path type)
              || (builtins.baseNameOf path == "migrations" || builtins.match ".*/migrations/.*" path != null);
          };

          commonArgs = {
            inherit src;
            buildInputs = [ pkgs.libmysqlclient ];
            nativeBuildInputs = [ pkgs.pkg-config ];
            RUSTFLAGS = rustFlags;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          treefmtEval = treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.rustfmt = {
              enable = true;
              package = toolchain;
            };
          };
        in
        {
          inherit
            pkgs
            toolchain
            craneLib
            rustFlags
            treefmtEval
            commonArgs
            cargoArtifacts
            ;
        }
      );
    in
    {
      packages = forEachSystem (
        system:
        let
          inherit (perSystem.${system})
            pkgs
            toolchain
            craneLib
            commonArgs
            cargoArtifacts
            ;
        in
        rec {
          default = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            GIT_HASH = self.shortRev or self.dirtyShortRev or "NOCOMMITHASH";
          });

          rustToolchain = toolchain;

          docker = pkgs.dockerTools.buildLayeredImage {
            name = "ghcr.io/testausserveri/testauskoira-rs";
            config = {
              Cmd = [ "${default}/bin/testauskoira-rs" ];
              WorkingDir = "/app";
            };
          };
        }
      );

      devShells = forEachSystem (
        system:
        let
          inherit (perSystem.${system}) pkgs craneLib rustFlags;
        in
        {
          default = craneLib.devShell {
            packages = [
              pkgs.diesel-cli
              pkgs.pkg-config
              pkgs.libmysqlclient
            ];

            RUSTFLAGS = rustFlags;
          };
        }
      );

      formatter = forEachSystem (system: perSystem.${system}.treefmtEval.config.build.wrapper);

      checks = forEachSystem (
        system:
        let
          inherit (perSystem.${system})
            craneLib
            commonArgs
            cargoArtifacts
            treefmtEval
            ;
        in
        {
          build = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            GIT_HASH = self.shortRev or self.dirtyShortRev or "NOCOMMITHASH";
          });
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings --allow non_local_definitions";
            }
          );
          formatting = treefmtEval.config.build.check self;
        }
      );
    };
}
