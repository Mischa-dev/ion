# Evaluates the home-manager module against stand-ins for the two
# home-manager options it sets, and checks the generated config.toml against
# the fixture that ion-config's tests parse, so the Nix and Rust sides agree.
{
  lib,
  pkgs,
  runCommand,
  module,
}:
let
  eval = lib.evalModules {
    modules = [
      module
      {
        options.home.packages = lib.mkOption {
          type = lib.types.listOf lib.types.package;
          default = [ ];
        };
        options.xdg.configFile = lib.mkOption {
          type = lib.types.attrsOf (lib.types.submodule { options.source = lib.mkOption { }; });
          default = { };
        };
        config._module.args.pkgs = pkgs;
        # The example from the product spec, plus a pass-through key.
        config.programs.ion = {
          enable = true;
          package = null;
          theme.source = "dms";
          ui = {
            density = "compact";
            cornerRadius = 8;
            tabs = "vertical";
            animations = {
              enable = true;
              speed = 1.5;
            };
          };
          bangs = {
            gh = "https://github.com/search?q={}";
            nix = "https://search.nixos.org/packages?query={}";
          };
          adblock.lists = [
            "easylist"
            "easyprivacy"
            "ublock-filters"
          ];
          general.restoreSession = false;
        };
      }
    ];
  };
  generated = eval.config.xdg.configFile."ion/config.toml".source;
in
runCommand "ion-hm-module-test" { } ''
  diff -u ${../crates/ion-config/tests/fixtures/hm-module.toml} ${generated}
  touch $out
''
