# home-manager module: `programs.ion` writes ~/.config/ion/config.toml, the
# read-only base layer of Ion's config. Changes made in Ion's settings UI go to
# overrides.toml next to it and win over these values.
#
# Option names are the TOML keys, so `programs.ion.ui.cornerRadius = 8;`
# becomes `[ui] cornerRadius = 8`. Settings left unset (null) keep Ion's
# default. Keys without a typed option below pass through as-is.
{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib) mkOption types;
  cfg = config.programs.ion;
  toml = pkgs.formats.toml { };

  # A setting that is only written when set.
  setting =
    type: description:
    mkOption {
      type = types.nullOr type;
      default = null;
      inherit description;
    };

  # Drop unset settings and the sections they leave empty.
  clean =
    attrs:
    lib.filterAttrs (_: v: v != null && v != { }) (
      lib.mapAttrs (_: v: if builtins.isAttrs v && !lib.isDerivation v then clean v else v) attrs
    );

  settings = clean (
    removeAttrs cfg [
      "enable"
      "package"
    ]
  );
in
{
  options.programs.ion = mkOption {
    default = { };
    description = "Ion browser configuration, written to `~/.config/ion/config.toml`.";
    type = types.submodule {
      freeformType = toml.type;
      options = {
        enable = lib.mkEnableOption "the Ion browser";

        package = mkOption {
          type = types.nullOr types.package;
          default = self.packages.${pkgs.stdenv.hostPlatform.system}.default or null;
          defaultText = lib.literalExpression "ion.packages.\${pkgs.stdenv.hostPlatform.system}.default";
          description = "The Ion package to install, or null to only write the config.";
        };

        general = {
          homePage = setting types.str "Page opened at startup when there is nothing to restore.";
          restoreSession = setting types.bool "Reopen the previous session's tabs on start.";
        };

        search = {
          engine = setting types.str "Name of the default search engine.";
          template = setting types.str "Search URL with `{}` where the query goes.";
        };

        ui = {
          density = setting (types.enum [
            "comfortable"
            "compact"
          ]) "How much space the interface uses.";
          cornerRadius = setting types.ints.unsigned "Corner radius in logical pixels.";
          tabs = setting (types.enum [
            "horizontal"
            "vertical"
          ]) "Tab strip layout.";
          animations = {
            enable = setting types.bool "Whether the interface animates.";
            speed = setting (types.addCheck types.number (x: x > 0)) "Animation speed multiplier.";
          };
        };

        theme = {
          source = setting (types.enum [
            "builtin"
            "dms"
            "system"
            "manual"
          ]) "Where Ion's palette comes from.";
          name = setting types.str "Built-in theme to use with `source = \"builtin\"`.";
          palette = setting types.str "Palette file for the `manual` and `dms` sources.";
          pages =
            setting
              (types.enum [
                "match"
                "system"
                "darken"
              ])
              "What web pages see: the theme's light/dark (`match`), the system's, or `match` plus darkening pages without a dark style.";
        };

        bangs = mkOption {
          type = types.attrsOf types.str;
          default = { };
          example = {
            gh = "https://github.com/search?q={}";
          };
          description = "Bangs (`!name query`) to URL templates with `{}`, added to Ion's built-in set.";
        };

        adblock = {
          enable = setting types.bool "Whether to block ads and trackers.";
          lists = setting (types.listOf types.str) "Filter lists, by well-known name or URL.";
        };

        shortcuts = mkOption {
          type = types.attrsOf types.str;
          default = { };
          example = {
            palette = "Ctrl+K";
          };
          description = "Shortcut remaps: command id to Qt key sequence.";
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = lib.optional (cfg.package != null) cfg.package;
    xdg.configFile."ion/config.toml".source = toml.generate "ion-config.toml" settings;
  };
}
