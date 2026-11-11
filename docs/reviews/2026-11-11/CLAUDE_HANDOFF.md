# Ion review handoff for Claude

Five reviewed implementation commits are published on `review/ion-findings-20261111` in Mischa-dev/ion. This directory contains the review record so the branch can be evaluated directly. No merge, auto-merge, PR creation or deployment was performed.

Reviewed base: `7e6b747301142d9ae2001d380a59043db781d255`.
Published implementation head: `b82cccb4507163bbb24c21c906c9a09559508835`.
Original local head: `eb3733078822d404512e4ff62995a571e7f4db8a`.
Implementation tree: `50c6602e4dc04ea87d360495c8d467602550cf88`.

The latest branch head adds this documentation only. The five implementation commits retain the exact reviewed source trees; API publication changed author/committer metadata and commit IDs. See [COMMIT_MAPPING.md](COMMIT_MAPPING.md).

## Start with the ledger

[FINDINGS.md](FINDINGS.md) accounts for all 23 assigned ion findings, with a verdict, source evidence and remaining work for each. Five are fixed, three already have a remedy on base, and fifteen remain blocked from a sufficiently tested fix. All six P1 findings remain open. This branch does not resolve the browser's agent safety findings as a group.

The five accepted changes reject incomplete OpenAI event streams, preserve the freshest nonempty duplicate history title, retry atomic-save temp collisions, show known targets in detailed activity lines, and omit whitespace-only note drafts from saved snapshots.

The timeline formatter only displays targets that exist in the audit entry. ION-08, which omits click/fill targets in the native bridge, remains open. The old navigation candidate was withheld because it changes navigation behavior and its Qt lifecycle behavior was not validated.

## Review from this branch

```sh
git fetch origin review/ion-findings-20261111
git switch --track origin/review/ion-findings-20261111
git diff 7e6b747301142d9ae2001d380a59043db781d255..b82cccb4507163bbb24c21c906c9a09559508835
git log --oneline 7e6b747301142d9ae2001d380a59043db781d255..b82cccb4507163bbb24c21c906c9a09559508835
```

If the branch is already checked out, fetch it and inspect its current head rather than creating a second tracking branch. Use the implementation head above to inspect the reviewed code separately from the documentation commit.

All 248 unit tests in affected packages pass. The same five external public-API probes fail on base and pass on the reviewed content. Strict Clippy passes for the Qt-free workspace. The full Qt-free run has 718 passes and the same three Unix-socket permission failures observed on pristine base. Full native tests/lint stop at missing Qt, and Nix checks could not run. [VALIDATION.md](VALIDATION.md) records commands, results and limitations.

Use the supported development environment before deciding whether to merge:

```sh
nix develop
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
nix flake check -L
```

The branch includes the implementation's unit regressions. The separately delivered portable handoff retains the external probes, detailed logs, original local patch series and bundle; those files are not needed to inspect this branch's source or ledger and are not copied into the repository.

## Decisions that remain open

ION-03 and ION-04 need an explicit conservative policy for script-driven effects and purchase approval. A few English button-label matches cannot enforce those boundaries. ION-16 needs a complete Qt key representation or native canonicalization. ION-17's verdict covers single-command Reset; Reset All semantics remain a separate product decision. ION-21's missing persistent refresh is addressed on base, but stable-ID activation could still strengthen event-order handling.

Native testing must cover approval/document binding, private panel storage, shared-tab task resumption, audit target emission, workspace-aware tab opening, retention changes while agents stay disabled, external prompt lifetimes and arming, and WebEngine new-window adoption. The ledger provides the source paths and next evidence for each remaining finding.

Final implementation acceptance and merging remain with the user and their Claude project. Publication does not imply that a blocked finding was fixed or that unavailable checks passed.
