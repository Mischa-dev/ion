# Ion finding ledger

Reviewed against main `7e6b747301142d9ae2001d380a59043db781d255`. Published implementation head `b82cccb4507163bbb24c21c906c9a09559508835`. Original-to-published commit IDs are recorded in [COMMIT_MAPPING.md](COMMIT_MAPPING.md).

All 23 assigned findings were independently checked. Verdicts: 5 fixed, 3 already fixed, 15 blocked. No finding was silently omitted or accepted from its label. Blocked means the source issue remains and no sufficiently tested fix is included. It does not mean the report is invalid.

## Index

| ID | PR | Finding | Verdict |
|---|---|---|---|
| ION-01 | #35 | Revalidate the target before running an approved read | blocked |
| ION-02 | #36 | Classify ordinary personal fields as personal data | blocked |
| ION-03 | #36 | Route script-driven commits through submit approval | blocked |
| ION-04 | #36 | Route purchase clicks through purchase approval | blocked |
| ION-05 | #59 | Revalidate cached tabs after the batch prompt | blocked |
| ION-06 | #63 | Keep persistent panels out of private windows | blocked |
| ION-07 | #36 | Resume every paused task when handing back a tab | blocked |
| ION-08 | #36 | Preserve the target URL in click and fill audit entries | blocked |
| ION-09 | #37 | Open agent tabs in the task's workspace | blocked |
| ION-10 | #38 | Reject incomplete event streams before finishing | fixed |
| ION-11 | #39 | Include the action target in detailed timeline lines | fixed |
| ION-12 | #39 | Keep the activity button visible during conversations | blocked |
| ION-13 | #40 | Apply conversation retention changes without another agent action | blocked |
| ION-14 | #41 | Retain the freshest non-empty duplicate title | fixed |
| ION-15 | #45 | Retry temp-name collisions before failing the save | fixed |
| ION-16 | #49 | Accept every key emitted by the shortcut recorder | blocked |
| ION-17 | #49 | Reset base-layer shortcuts to their actual defaults | already fixed |
| ION-18 | #50 | Clear external-app prompts after any new navigation | blocked |
| ION-19 | #50 | Rearm external-app prompts when they become visible | blocked |
| ION-20 | #51 | Place existing panes at the indicated drop position | already fixed |
| ION-21 | #51 | Resolve search results by stable tab identity | already fixed |
| ION-22 | #63 | Honor the WebEngine new-window request | blocked |
| ION-23 | #63 | Do not persist placeholder notes | fixed |

## ION-01: Revalidate the target before running an approved read

Verdict: blocked.

Confirmed. The waiting entry holds only tool and tab. start removes it and calls run on the current view; read_page reads the current document. The earlier fake-renderer baseline reproducer passes by observing replacement-page content. The candidate patch adds document tokens and navigation generations but also changes go_to semantics. It is withheld, not accepted or declared invalid.

Base evidence: [crates/ion-app/qml/components/AgentTools.qml:371](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/AgentTools.qml#L371).

Remaining work: Complete QtWebEngine tests for cross-origin navigation, same-URL reload, same-document changes, renderer replacement, callback races, and ordinary navigation before accepting the earlier patch.

## ION-02: Classify ordinary personal fields as personal data

Verdict: blocked.

Confirmed for standard type=email/tel and autocomplete=name/email/address fields. The predicate recognizes passwords, card autocomplete tokens and payment names. tool::plan maps false to Interact, and the audit path can retain typed text. docs/SAFETY.md promises personal-field approval and redaction. No tested native change is included.

Base evidence: [crates/ion-app/qml/components/AgentTools.qml:55](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/AgentTools.qml#L55).

Remaining work: Define conservative metadata rules, including autocomplete token lists and plain field labels. Test both renderer classification and Rust approval/redaction. Unknown fields cannot be classified reliably from names alone.

## ION-03: Route script-driven commits through submit approval

Verdict: blocked.

Confirmed. ionSubmits requires el.form and native submit types. A button without a form or an ARIA button with a fetch handler is classified as Interact. Arbitrary JavaScript side effects cannot be inferred from the DOM predicate. Calling every link a submit would alter ordinary browsing and still miss input/change effects.

Base evidence: [crates/ion-app/qml/components/AgentTools.qml:48](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/AgentTools.qml#L48).

Remaining work: Choose and test an explicit conservative interaction policy or enforce commit approval at a broader effect boundary. This is a product/enforcement decision, not a keyword-only patch.

## ION-04: Route purchase clicks through purchase approval

Verdict: blocked.

Confirmed. The click planner only emits Submit or Interact. Element has submits/sensitive metadata but no purchase classification; Action::Purchase exists in ion-safety and is never produced by this browser planner. A label-only Buy regex would miss alternate labels and script-driven checkout.

Base evidence: [crates/ion-agent/src/tool.rs:453](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-agent/src/tool.rs#L453).

Remaining work: Decide how purchase intent is represented and enforced, then add end-to-end metadata, planner and renderer checks. Native validation and an explicit policy are required.

## ION-05: Revalidate cached tabs after the batch prompt

Verdict: blocked.

Confirmed. Reading.allowed stores tab IDs. authorize_tabs fills it before prompts and answer_tabs extends it without rechecking the originally allowed pages. QML read_tabs resolves those IDs to current views. The earlier fake-renderer baseline test observes reading a replacement page. This shares the lifecycle boundary with ION-01 but is separately tracked because the cached batch path must also be fixed.

Base evidence: [crates/ion-app/src/bridge/agent.rs:315](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/src/bridge/agent.rs#L315).

Remaining work: Carry and check page/document identity for every batch member through dispatch and callback. Test one allowed tab navigating while another waits. Do not only recheck newly approved prompts.

## ION-06: Keep persistent panels out of private windows

Verdict: blocked.

Confirmed. SidePanel is unconditional and references Panels. The Panels initializer reads panels.json/notes.json and its mutation paths save them via the ordinary data directory. ion-privacy/src/window.rs explicitly returns the base directory for private mode. A visibility-only fix would still initialize persistent state.

Base evidence: [crates/ion-app/qml/Main.qml:1031](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/Main.qml#L1031).

Remaining work: Add a private-mode storage boundary or avoid persistent singleton creation and all other entry points. Test private startup and mutations against sentinel profile files, including Clip to Notes and panel commands.

## ION-07: Resume every paused task when handing back a tab

Verdict: blocked.

Confirmed. hand_back clears safety take-over for all tabs belonging to the selected task, but takes paused and re-emits tool_requested only for that task. Other tasks sharing a handed-back tab retain paused calls. start and follow_up also hand back tab-wide safety state, so a complete fix must account for those callers.

Base evidence: [crates/ion-app/src/bridge/agent.rs:1420](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/src/bridge/agent.rs#L1420).

Remaining work: Test multiple tasks on one tab, tasks owning several tabs, closed calls and re-authorization. Resume every affected paused call exactly once without resuming unrelated tabs.

## ION-08: Preserve the target URL in click and fill audit entries

Verdict: blocked.

Confirmed. logged_url initially uses payload.url when planned.request.site exists. click/fill remove box and return no url. The authorized plan contains the origin, so successful action records lose a known target. ION-11 only fixes formatting when an audit target exists; it does not repair this emitter.

Base evidence: [crates/ion-app/src/bridge/agent.rs:1218](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/src/bridge/agent.rs#L1218).

Remaining work: Populate the logged target from the authorized plan for click/fill, and test the actual bridge success/failure audit path in the native build. Retain query/fragment redaction.

## ION-09: Open agent tabs in the task's workspace

Verdict: blocked.

Confirmed. The opener reference changes the insertion row only. Tabs::open_tab_at calls insert_at and TabList::open assigns current_workspace. Switching workspaces while the model runs therefore changes the new background tab workspace.

Base evidence: [crates/ion-app/qml/components/AgentTools.qml:314](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/AgentTools.qml#L314).

Remaining work: Add an opener/task-workspace-aware tab insertion API and test non-current workspaces, removed opener tabs, Essentials, and background focus behavior through the model bridge.

## ION-10: Reject incomplete event streams before finishing

Verdict: fixed.

Fixed in d6630f6. Stream::finish now returns Result and rejects any stream without DONE. reply_streaming propagates the error. Valid completed text/tool streams remain covered by existing tests; new tests cover partial text, partial calls, empty EOF and completed empty replies. The same public-API probe fails on base and passes on head. Partial text already sent to the UI callback cannot be retracted, but no successful partial reply/tool call is returned.

Base evidence: [crates/ion-agent/src/openai.rs:241](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-agent/src/openai.rs#L241).

## ION-11: Include the action target in detailed timeline lines

Verdict: fixed.

Formatter fixed in 7963e16. Detailed Interact, Submit, EnterPersonalData and Purchase entries append their known target host, for successful and failed outcomes. Missing targets keep the old wording; already self-describing read/navigation details are unchanged. Updated unit tests and a public-API probe pass. This does not reconstruct targets absent from historical entries or fix the separate ION-08 bridge emitter.

Base evidence: [crates/ion-safety/src/timeline.rs:65](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-safety/src/timeline.rs#L65).

## ION-12: Keep the activity button visible during conversations

Verdict: blocked.

Confirmed. The history IconButton is nested in the no-task ColumnLayout. AgentConversation has no activityRequested control and fills the sidebar while a task is selected.

Base evidence: [crates/ion-app/qml/components/AgentSidebar.qml:126](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/AgentSidebar.qml#L126).

Remaining work: Move the shared activity control into a header available to both states, then check layout, focus, close/new-conversation controls and minimum sidebar width in native QML.

## ION-13: Apply conversation retention changes without another agent action

Verdict: blocked.

Confirmed. history lazily opens the store and applies keep_conversations_days only when called. Agent has no retention subscription. Config change propagation updates Config properties and browser privacy settings but does not call this history synchronization. Disabling agents can leave stored conversations until another history action.

Base evidence: [crates/ion-app/src/bridge/agent.rs:395](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/src/bridge/agent.rs#L395).

Remaining work: Apply retention on configuration change/startup independently of agent activity. Test pre-existing disk history while agents stay disabled and retain thread-safe ordering. Avoid initializing or writing the normal profile history in private mode.

## ION-14: Retain the freshest non-empty duplicate title

Verdict: fixed.

Fixed in 46ba9ac. Deduplication tracks title time independently from last_visit, preserving the maximum visit timestamp and visits while selecting the freshest nonempty title. Regression covers all six orderings of Old/empty/Middle. The same public-API load probe fails on base and passes on head.

Base evidence: [crates/ion-session/src/history.rs:112](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-session/src/history.rs#L112).

## ION-15: Retry temp-name collisions before failing the save

Verdict: fixed.

Fixed in 95bd92e. Atomic save retries AlreadyExists with up to 128 fresh counter names and propagates other errors. It only cleans up a file it actually created. The original code also removed the pre-existing collision file on failure; this fix preserves it. Tests cover collision retry/exhaustion, existing-file preservation, atomic replacement, private permissions and symlink safety. The public write_atomic probe fails on base and passes on head.

Base evidence: [crates/ion-session/src/paths.rs:31](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-session/src/paths.rs#L31).

## ION-16: Accept every key emitted by the shortcut recorder

Verdict: blocked.

Confirmed by the boundary mismatch. cpp/keys.cpp emits QKeySequence::PortableText for Qt key events; normalize_press accepts a small named-key table, F1-F35 and individual characters, otherwise None. Merely accepting all strings would drop validation and canonical equality guarantees.

Base evidence: [crates/ion-input/src/keymap.rs:66](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-input/src/keymap.rs#L66).

Remaining work: Use Qt validation/canonicalization or a complete documented key representation through the recorder and core. Verify Media Play, Volume Up, international keys, modifiers and chords with the actual Qt version.

## ION-17: Reset base-layer shortcuts to their actual defaults

Verdict: already fixed.

The reported single-command reset bug is already handled in current main. reset assigns binding.defaults through save. save removes overrides, re-evaluates the effective keys, and writes explicit defaults if a base-layer remap still wins. It also clears command-id aliases. This code is already present in the merged #49 commit 81b47e5. No new patch. Scope caveat: Reset All still uses Store::reset and therefore restores configured base values; its UI semantics are a separate decision, not evidence that single-command reset lacks this fallback.

Base evidence: [crates/ion-app/src/bridge/keymap.rs:175](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/src/bridge/keymap.rs#L175).

## ION-18: Clear external-app prompts after any new navigation

Verdict: blocked.

Confirmed. LoadStarted rejects and clears only non-open requests. An open prompt for the prior page survives a new document load and can still call Qt.openUrlExternally.

Base evidence: [crates/ion-app/qml/components/ExternalAppPrompt.qml:114](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/ExternalAppPrompt.qml#L114).

Remaining work: Clear every prompt at new-page start, rejecting request objects only for handler/files kinds. Test redirect, cancellation, open prompts and native request lifetimes.

## ION-19: Rearm external-app prompts when they become visible

Verdict: blocked.

Confirmed. Current changes restart an unconditional Timer; its callback sets armed=true without checking effective visibility. Neither parent-hidden/background-tab state nor window activation resets the delay.

Base evidence: [crates/ion-app/qml/components/ExternalAppPrompt.qml:93](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/ExternalAppPrompt.qml#L93).

Remaining work: Require current request, effective visibility and intended active-window policy for arming, reset when hidden, and verify real Qt parent-visibility/timer ordering. Do not count a fake timer alone as native verification.

## ION-20: Place existing panes at the indicated drop position

Verdict: already fixed.

Already fixed by 8d1706e, the #65 follow-up. Main calls placePane with before/after determined from the drop side. TabList::place_pane removes the existing pane and reinserts it adjacent to the anchor while carrying its size. The existing core pane-placement regression passes in the full Qt-free suite. No new patch.

Base evidence: [crates/ion-app/qml/Main.qml:179](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/Main.qml#L179).

## ION-21: Resolve search results by stable tab identity

Verdict: already fixed.

The concrete missing-refresh report is already addressed by 8d1706e, the #65 follow-up. An enabled Connections listens for Tabs.sessionChanged while open and schedules refreshKeepingSelection. Tab insertion uses insert_with -> sync -> session_changed, including window.open via openTabNextToCurrent. refresh restores selection by kind/id. choose/closeRow still consume current snapshot indexes; this verdict covers the reported persistent stale-list condition, not a proof of every event-order race. A stable-ID activation API would be further hardening, not an included fix.

Base evidence: [crates/ion-app/qml/components/TabSearch.qml:47](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/TabSearch.qml#L47).

## ION-22: Honor the WebEngine new-window request

Verdict: blocked.

Confirmed. The panel discards the WebEngineNewWindowRequest and opens requestedUrl as a new navigation. Main already has a normal-tab request.openIn(tab) path with opener/autoplay setup, showing the integration precedent. POST state and script-created blank windows are not equivalent to URL-only navigation.

Base evidence: [crates/ion-app/qml/components/SidePanel.qml:145](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-app/qml/components/SidePanel.qml#L145).

Remaining work: Reuse/adapt the request-adoption path for panel openers. Test POST, window.open blank/script-population, background destination and profile/autoplay inheritance in QtWebEngine.

## ION-23: Do not persist placeholder notes

Verdict: fixed.

Fixed in b82cccb at the persistence boundary. Notes::to_json filters whitespace-only drafts from a cloned save snapshot while keeping in-memory IDs/editing state and next_id. Real notes and drafts subsequently filled with text still round-trip. This covers every bridge save, including saves caused by another note. New unit tests pass on head. The same public-API probe fails on base and passes on head. Live draft presentation in QML is unchanged.

Base evidence: [crates/ion-panels/src/notes.rs:50](https://github.com/Mischa-dev/ion/blob/7e6b747301142d9ae2001d380a59043db781d255/crates/ion-panels/src/notes.rs#L50).
