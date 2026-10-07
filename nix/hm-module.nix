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

  # One agent's trust settings (`agents.<id>`); see docs/SAFETY.md.
  agentType = types.submodule {
    options = {
      name = setting types.str "Name shown in prompts and the activity log.";
      trust = setting (types.enum [
        "ask"
        "trustedSites"
        "full"
        "custom"
      ]) "How much the agent may do without asking.";
      trustedSites = setting (types.listOf types.str) "Sites a `trustedSites` agent acts on freely.";
      connectors = setting (types.listOf types.str) "Connectors the agent may use without asking.";
      rules = setting (types.listOf (
        types.submodule {
          options = {
            action = mkOption {
              type = types.str;
              description = "An action id, `tier:<tier>`, or `*`.";
            };
            site = mkOption {
              type = types.str;
              default = "*";
              description = "A domain, an origin, or `*`.";
            };
            effect = mkOption {
              type = types.enum [
                "allow"
                "ask"
                "deny"
              ];
              description = "What the rule decides.";
            };
          };
        }
      )) "Rules for this agent, most specific wins.";
    };
  };

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
          suspendTabsAfter = setting types.ints.unsigned "Unload background tabs unseen for this many minutes (0 = never).";
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
          collapseSidebar = setting types.bool "With vertical tabs, show only favicons until the sidebar is hovered.";
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
          pageControls = setting types.bool "Give web page scrollbars, checkboxes and text selection the theme's colors, unless the page styles them itself.";
          sites = mkOption {
            type = types.attrsOf (
              types.submodule {
                options = {
                  darken = setting types.bool "Darken this site under a dark theme (true) or never (false); unset follows `theme.pages`.";
                  css = setting types.lines "CSS added to this site's pages.";
                };
              }
            );
            default = { };
            example = {
              "example.com".css = "body { max-width: 50em; margin: auto }";
              "github.com".darken = false;
            };
            description = "Per-site theming, keyed by site. A site also covers its subdomains; the most specific entry wins.";
          };
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

        extensions =
          setting (types.listOf (types.coercedTo types.package toString types.str))
            "Unpacked Chrome extension folders (Manifest V3), or packages that build one, to load at startup.";

        keyboard = {
          vim = setting types.bool "Vim-style keys and link hints in pages.";
        };

        privacy = {
          globalPrivacyControl = setting types.bool "Send the Global Privacy Control signal to sites.";
          blockThirdPartyCookies = setting types.bool "Refuse cookies from sites other than the one in the address bar.";
        };

        sites = mkOption {
          type = types.attrsOf (
            types.submodule {
              options = {
                javascript = setting types.bool "Run JavaScript on the site.";
              };
            }
          );
          default = { };
          example = {
            "example.com".javascript = false;
          };
          description = "Per-site settings keyed by host (site CSS is `theme.sites`); a host also covers its subdomains, `*` covers every site.";
        };

        agents = mkOption {
          default = { };
          example = {
            claude-code.trust = "full";
            mcp.enable = true;
          };
          description = "Agents' trust, by agent id, and Ion's MCP server (`agents.mcp`).";
          type = types.submodule {
            freeformType = types.attrsOf agentType;
            options.mcp.enable = setting types.bool "Serve Ion's browser tools to outside agents over localhost MCP.";
          };
        };

        safety = {
          auditRetentionDays = setting types.ints.positive "Days of agent activity log kept.";
        };

        downloads = {
          directory = setting types.str "Download folder; empty uses the system's. `~` means home.";
          folders = mkOption {
            type = types.attrsOf types.str;
            default = { };
            example = {
              image = "~/Pictures/Downloads";
              video = "~/Videos";
            };
            description = "Per-type download folders: document, image, audio, video, archive or other.";
          };
        };

        newTab = {
          shortcuts = setting (types.listOf (
            types.submodule {
              options = {
                title = mkOption {
                  type = types.str;
                  default = "";
                  description = "Tile name; empty uses the site's host.";
                };
                url = mkOption {
                  type = types.str;
                  description = "Page the tile opens.";
                };
              };
            }
          )) "Tiles always shown first on the new-tab page, in order.";
          mostVisited = setting types.bool "Fill the remaining new-tab tiles with the sites visited most (default true).";
          tiles = setting types.ints.positive "How many new-tab tiles to show at most (default 8).";
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
