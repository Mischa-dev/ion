{
  description = "Ion: a fast, themeable desktop browser (QtWebEngine + QML + Rust)";

  inputs = {
    # Channel tarball (not github:) so the lock resolves to an immutable
    # releases.nixos.org snapshot that has passed Hydra.
    nixpkgs.url = "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        ion = pkgs.callPackage ./nix/package.nix { src = self; };
        default = ion;
      });

      homeManagerModules = rec {
        ion = import ./nix/hm-module.nix { inherit self; };
        default = ion;
      };

      devShells = forAllSystems (pkgs: {
        default = pkgs.callPackage ./nix/shell.nix {
          ion = self.packages.${pkgs.stdenv.hostPlatform.system}.ion;
        };
      });

      checks = forAllSystems (
        pkgs:
        let
          ion = self.packages.${pkgs.stdenv.hostPlatform.system}.ion;
        in
        {
          inherit ion;
          packaging = pkgs.callPackage ./nix/packaging-check.nix { inherit ion; };
          hm-module = pkgs.callPackage ./nix/hm-module-test.nix {
            module = self.homeManagerModules.default;
          };
          clippy = ion.overrideAttrs (old: {
            pname = "ion-clippy";
            nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
            buildPhase = ''
              runHook preBuild
              cargo clippy --workspace --all-targets --offline -- -D warnings
              runHook postBuild
            '';
            doCheck = false;
            installPhase = "touch $out";
            dontFixup = true;
            dontWrapQtApps = true;
          });
          fmt =
            pkgs.runCommand "ion-fmt"
              {
                nativeBuildInputs = [
                  pkgs.cargo
                  pkgs.rustfmt
                  pkgs.nixfmt
                ];
              }
              ''
                cd ${self}
                cargo fmt --all --check
                nixfmt --check flake.nix nix/*.nix
                touch $out
              '';
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
    };
}
