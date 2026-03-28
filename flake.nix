{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      fenix,
      crane,
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
        in
        { inherit pkgs toolchain craneLib rustFlags; }
      );
    in
    {
      packages = forEachSystem (
        system:
        let
          inherit (perSystem.${system}) pkgs toolchain craneLib rustFlags;
        in
        rec {
          default = craneLib.buildPackage {
            buildInputs = [
              pkgs.libmysqlclient
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
            ];

            RUSTFLAGS = rustFlags;

            src = pkgs.lib.cleanSourceWith {
              src = ./.;
              filter = path: type:
                (craneLib.filterCargoSources path type) || (builtins.baseNameOf path == "migrations" || builtins.match ".*/migrations/.*" path != null);
            };
          };

          rustToolchain = toolchain;

          docker = pkgs.dockerTools.buildLayeredImage {
            name = "ghcr.io/testausserveri/testauskoira-rs";
            config.Cmd = [ "${default}/bin/testauskoira-rs" ];
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
    };
}
