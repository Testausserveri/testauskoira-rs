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

      perSystem =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          toolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-zC8E38iDVJ1oPIzCqTk/Ujo9+9kx9dXq7wAwPMpkpg0=";
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        in
        { inherit pkgs toolchain craneLib; };
    in
    {
      packages = forEachSystem (
        system:
        let
          inherit (perSystem system) pkgs toolchain craneLib;
        in
        rec {
          default = craneLib.buildPackage {
            buildInputs = [
              pkgs.libmysqlclient
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
            ];

            RUSTFLAGS = "-C link-arg=-Wl,-rpath,${pkgs.libmysqlclient}/lib/mariadb";

            src = ./.;
          };

          rustToolchain = toolchain;

          docker = pkgs.dockerTools.buildLayeredImage {
            name = "ghcr.io/testausserveri/testauskoira-rs";
            config.Cmd =
              let
                entrypoint = pkgs.writeShellScriptBin "entrypoint.sh" ''
                  while [ 1 ];
                  do
                      ${pkgs.diesel-cli}/bin/diesel database setup --migration-dir ${./migrations} && break;
                  done
                  ${default}/bin/testauskoira-rs
                '';
              in
              [ "./${entrypoint}/bin/entrypoint.sh" ];
          };
        }
      );

      devShells = forEachSystem (
        system:
        let
          inherit (perSystem system) pkgs craneLib;
        in
        {
          default = craneLib.devShell {
            packages = [
              pkgs.diesel-cli
              pkgs.pkg-config
              pkgs.libmysqlclient
            ];

            RUSTFLAGS = "-C link-arg=-Wl,-rpath,${pkgs.libmysqlclient}/lib/mariadb";
          };
        }
      );
    };
}
