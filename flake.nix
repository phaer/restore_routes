{
  description = "restore network routes";
  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages."${system}";
      naersk-lib = naersk.lib."${system}";
    in rec {
      # `nix build`
      packages.restore-routes = naersk-lib.buildPackage {
        pname = "restore-routes";
        root = ./.;
      };
      defaultPackage = packages.restore-routes;

      # `nix develop`
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs;
          [
            rustc cargo
            clippy rust-analyzer rustfmt
          ];
      };
    });
}
