# Ion permission and safety model

Status: design for review, written 2026-10-07 while Mischa was asleep. Defaults
picked without Mischa are marked **(default)** so they are easy to find and
change. Code lives in `crates/ion-safety` (Qt-free) with a thin `Safety` QML
singleton in `crates/ion-app/src/bridge/safety.rs`.

The spec says agents may not ship until this exists ("Permission model in the AI
section is required for any agent release"). This document is that model. It
also covers site permissions (camera, location…), so there is one place that
answers "may X do Y here?" and one log of the answers.

## 1. Goals

1. One question, one answer: every permission check in Ion is
   `decide(request) -> Allow | Deny | Ask`, made in Rust, deterministic and unit
   tested. QML never decides; it only shows prompts and passes answers back.
2. Trust you control: from ask-every-time to full trust, per agent, set in the
   UI or in Nix (`programs.ion.agents.<id>.trust`).
3. Hard limits that no setting, rule or prompt answer can lift.
4. Everything an agent does is logged and reviewable, without the log itself
   becoming a privacy leak.
5. Costs nothing when idle: no threads, no timers, no page scripts. A decision
   is a few map lookups.

Non-goals for this work: any agent feature, the MCP server, model access,
agent workspaces (isolated profiles). The model is shaped for them.

## 2. Vocabulary

**Principal**: who is asking. Either a **site** (an origin such as
`https://meet.example.com`) or an **agent** (an id such as `ion`, `claude-code`,
`codex`, or a user-defined `research`). The person using Ion is never a
principal; their own clicks are not checked.

**Capability**: what is asked for.

- Site capabilities mirror QtWebEngine's `QWebEnginePermission::PermissionType`:
  camera, microphone, camera+microphone, screen share (with and without audio),
  pointer lock, notifications, location, clipboard, local fonts.
- Agent actions, each with a **tier** that sets how careful Ion is:

| Action | What it covers | Tier |
| --- | --- | --- |
| `readPage` | accessibility tree, text, screenshot of a tab | read |
| `listTabs` | titles and URLs of open tabs | read |
| `navigate` | load a URL, back/forward, reload | act |
| `openTab`, `closeTab` | including background tab groups | act |
| `interact` | click, type, scroll, select | act |
| `devTools` | console, network, DOM, running script | act |
| `download` | save a file to disk | act |
| `submit` | send, submit, post, publish, any form submission | commit |
| `uploadFile` | hand a local file to a page | personal |
| `enterPersonalData` | type name, address, phone, email, IDs | personal |
| `purchase` | anything that spends money | purchase |
| `useConnector:<name>` | an MCP server or other connector Ion holds | connector |
| `readCredentials` | saved passwords, payment details, cookies, tokens | forbidden |

Tiers are ordered read < act < commit < personal < purchase. `connector` sits
apart (it is not about a site). `forbidden` can never be allowed.

**Target**: where the action lands: a site origin, a connector name, or
nothing (for `listTabs`). Agent requests also carry the tab id they touch.

**Rule**: a stored answer. `principal + action pattern + site pattern ->
effect` with a **lifetime**.

- Action pattern: one action, a tier (`tier:commit`), or any.
- Site pattern: an exact origin (`https://github.com`), a domain and its
  subdomains (`github.com`), or any (`*`).
- Effect: allow, ask, deny.
- Lifetime: once (not stored), tab (until that tab closes), session (until Ion
  quits), forever (saved to disk).
- Source: the user (from a prompt or the settings page) or config (Nix/TOML).

## 3. Deciding

`decide` runs these steps in order and stops at the first that answers. The
answer always carries a **reason** (which step, which rule) for the log and for
the "why was this blocked?" text in the UI.

1. **Hard limits** (deny, no prompt):
   - `readCredentials` is denied for every agent, always.
   - Agents may not act on Ion's own pages (`ion:` and `chrome:` schemes, the
     settings and permission pages), so an agent can never change its own
     trust. Reading them is denied too.
   - An action about a site must name the site, and an action on a tab must
     name the tab (so taking a tab back can't be sidestepped); requests that
     don't are refused. Actions not about a site (listing tabs, connectors)
     must not name one, so a made-up site can't borrow a trusted site's trust.
   - Agent and connector ids must be plain identifiers (`[A-Za-z0-9._-]`, at
     most 64 characters), because prompts show them.
2. **Stop and take-over** (deny): when the global stop is engaged, every agent
   request is denied until the person resumes agents. A tab the person took
   back denies agent actions in that tab until handed back. A paused agent is
   denied everywhere.
3. **Rules**, most specific first. Specificity is action (exact > tier > any)
   then site (origin > domain > any; deeper domains beat shallower ones). At
   equal specificity deny beats ask beats allow. Config rules and user rules
   share this ordering, so a narrow user "allow" can open a hole in a broad
   config "deny" and vice versa, which matches what people expect from a
   firewall.
4. **Pages without a host** (`file:`, `data:`, `about:`) ask agents at every
   trust level; only a rule (an earlier "allow", kept for the session) lets an
   agent use them without asking.
5. **Trust level** of the agent (below), or for sites the site default.
6. Otherwise **ask**.

### Trust levels (per agent)

From the spec, made exact:

| Level | read / act | commit | personal, purchase | connectors |
| --- | --- | --- | --- | --- |
| `ask` **(default for every agent)** | allowed on sites the person approved for this agent, asks on any other site | asks | asks | asks on first use |
| `trustedSites` | allowed on the agent's trusted sites, else as `ask` | allowed on trusted sites, else asks | asks | asks on first use |
| `full` | allowed everywhere | allowed | allowed | allowed |
| `custom` | rules only; anything no rule covers asks | | | |

"Approving a site" for an agent is a rule `agent, tier:act, site -> allow`,
which covers read and act but not commit and above. `full` still obeys the
hard limits, stop and take-over, and asks for pages without a host.

`tier:<tier>` in a rule means that tier and the site tiers below it, so
`tier:act` covers read and act, and `tier:purchase` covers every site action.
`tier:connector` covers all connectors. To keep an agent off a site entirely,
use `action = "*"`.

### Site permissions

Default **(default)**: every site capability asks, and an answer is remembered
per origin (forever) unless the person picks "only this time". The engine
already refuses powerful features on insecure origins, so Ion does not repeat
that. Pointer lock and local fonts follow the same path; nothing is granted
silently.

## 4. Prompts

When `decide` says ask, the caller gets a **prompt**: id, principal, action,
target, tab, a sentence ("Ion Agent wants to submit a form on github.com"),
and the choices allowed for it:

| Tier | Choices |
| --- | --- |
| site capability | Allow, Allow this time, Block |
| read, act (agent on a new site) | Allow on this site, Allow once, Don't allow |
| commit | Allow once, Always allow on this site, Don't allow |
| personal, purchase | Allow once, Don't allow **(default: never remembered)** |
| connector | Allow, Allow once, Don't allow |

Answers become rules with the matching lifetime and source `user`. "Allow on
this site" for an agent covers read and act on that host and its subdomains;
"Always allow" for commit covers that one action there. Approvals for local files
last the session only; `data:` and `about:` pages share one origin, so for
them only "Allow once" is offered. `blob:` and `filesystem:` URLs take the
origin of the page that made them. "Don't
allow" on an agent prompt is remembered for the session only **(default)**,
so a misclick does not silently cripple an agent forever; site blocks are
remembered forever, like other browsers.

Prompt rules for the UI (QML follows these; the Rust side enforces what it
can):

- Prompts are drawn by Ion, outside the page, anchored to the tab's address
  bar **(default, follows the agent-feel proposal)**. Pages cannot draw over
  them or fake them.
- Buttons arm after a short delay (`Theme` duration, about 400 ms) so a click
  already in flight cannot land on "Allow".
- One prompt at a time per tab, oldest first. A navigation cancels the page's
  pending site prompts without recording an answer.
- An unanswered agent prompt blocks only that agent's step, never the page or
  other tabs. Prompts expire with their tab.
- An answer is checked against the policy again before it counts: if the tab
  closed, agents were stopped, the agent paused or the tab taken back while
  the prompt was up, the request is denied and nothing is remembered.

## 5. Untrusted content

"Page content is treated as data, never as instructions." The model enforces
the part that can be enforced:

- Only the agent runtime can create requests, and only the person (through
  Ion's own UI) or config can create rules. Nothing a page or an agent says can
  add a rule; there is no API for it.
- Prompt sentences are built from Ion's own words plus the origin's host, never
  from page titles or agent-supplied text, so a page cannot write the prompt.
- Agents cannot reach Ion's own pages (hard limit), so they cannot approve
  their own prompts or edit trust.

Prompt-injection defenses inside the agent loop (separating tool output from
instructions) belong to the agent runtime, built later.

## 6. Activity log

Every agent decision and every agent action outcome is written to an
append-only log, as are site permission answers and any rule change.

- Format: JSON Lines, one file per UTC day,
  `<data dir>/safety/audit/YYYY-MM-DD.jsonl` (`ION_DATA_DIR` honored).
- Entry: time, principal, action, target (origin or connector), tab, task id,
  decision, reason, and a short detail.
- Privacy: no page content, no screenshots. Typed text is kept only for
  ordinary fields; anything marked sensitive (password or personal fields) is
  stored as `[redacted, N chars]`. URLs drop their query and fragment.
- The log directory and files are private to the user (0700/0600 on Unix).
- Retention: 30 days **(default)**, configurable with
  `safety.auditRetentionDays`; old files are pruned at startup.
- The activity timeline UI (later) reads this log; screenshots for replay will
  be kept separately and shorter.

## 7. Storage and config

- Remembered rules: `<data dir>/safety/rules.json`, written atomically. Session
  and tab rules live in memory only.
- Config, in TOML or Nix:

```toml
[safety]
auditRetentionDays = 30

[agents.ion]
trust = "ask"                      # ask | trustedSites | full | custom
trustedSites = ["github.com"]      # used by trustedSites
connectors = ["github"]            # connectors this agent may use without asking

[[agents.ion.rules]]
action = "submit"                  # action id, "tier:<tier>", or "*"
site = "example.com"               # origin, domain, or "*"
effect = "allow"                   # allow | ask | deny
```

Agents not listed use `ask`. Config rules are read-only in the UI (the
settings page shows them with a "from config" badge), just like the rest of
Ion's layered config.

## 8. Stop everything

- One shortcut stops every agent at once: `Ctrl+Shift+Escape`
  **(default; Cmd+Shift+Escape on macOS)**, remappable as
  `shortcuts.stopAgents`. Also a command in the palette.
- Stopping denies every agent request immediately and stays on until the person
  resumes agents. It does not touch site permissions.
- Every tab an agent is driving shows an indicator; taking a tab back is the
  per-tab version of stop.

## 9. How it is built

- `crates/ion-safety` (no Qt):
  - `capability`: principals, site capabilities, agent actions, tiers, targets.
  - `pattern`: site patterns and matching (origin, domain, any).
  - `rule`: rules, lifetimes, effects, specificity.
  - `policy`: trust levels, agent profiles, hard limits, `decide`.
  - `prompt`: prompt sentences and allowed choices; answers to rules.
  - `audit`: log entries, redaction, daily files, pruning.
  - `store`: remembered rules on disk.
  - `Safety`: the whole thing behind one object (`decide`, `answer`, `stop`,
    `take_over`, `hand_back`, `log_action`), used from a mutex by the bridge
    and later by the agent runtime and MCP server.
- `ion-config` gains `[agents.*]` and `[safety]` sections; the bridge converts
  them into `ion-safety` profiles and reloads them live.
- `bridge/safety.rs`: `Safety` QML singleton: site decisions and answers for
  the permission prompt, the stop state and shortcut, and a list of remembered
  site permissions for a later settings page.

### Delivery

1. PR: this design plus `ion-safety` with full unit tests. No UI change.
2. PR: config sections, the `Safety` singleton, stop shortcut and palette
   command.
3. PR (after daily-driver basics #7 merges): route the existing site
   permission prompt through `Safety` (`decide` before prompting, answers
   recorded and logged), switch the profile to ask-every-time so Ion's store
   is the single source of truth, and add "Allow this time".

## 10. Open questions for Mischa

- Is `Ctrl+Shift+Escape` right for "stop all agents"? (Some Linux desktops use
  it for the system monitor.)
- Should purchases ever be rememberable below full trust? Current answer: no.
- Should "Don't allow" for agents be remembered forever, like site blocks?
  Current answer: session only.
- 30-day log retention, or longer?
