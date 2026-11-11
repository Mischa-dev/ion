# Validation summary

Checks ran with official stable rustc 1.99.1 in an isolated review environment. The repository lockfile was retained. No test or dependency was removed to obtain a pass. API publication preserved every reviewed implementation tree.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Passed |
| `git diff --check` | Passed |
| `cargo test -p ion-agent -p ion-session -p ion-safety -p ion-panels` | Passed, 248 unit tests |
| `cargo clippy --workspace --exclude ion-app --all-targets -- -D warnings` | Passed |
| Base `cargo test --workspace --exclude ion-app --no-fail-fast` | 714 passed, 3 failed |
| Reviewed implementation, same Qt-free command | 718 passed, the same 3 failed |
| Identical external public-API probes on pristine base | All 5 failed as expected |
| The same probes on reviewed implementation | All 5 passed |
| Earlier fake-renderer navigation baseline reproducer | Both tests observed unauthorized replacement-page reads |
| `cargo test --workspace` | Blocked during cxx-qt build, missing Qt |
| `cargo clippy --workspace --all-targets -- -D warnings` | Blocked during cxx-qt build, missing Qt |
| `nix flake check -L` | Could not run, nix command absent, exit 127 |
| Portable bundle and five-patch imports | Passed in a separate checkout; patch import tree matched original head |
| GitHub publication readback | Every implementation tree matched local review; exact base, five-commit chain and changed blobs verified |

The Qt-free totals include two doc tests. Four new unit tests account for the increase from 714 to 718; an existing timeline test was also extended.

## Baseline failures

Both pristine base and reviewed implementation fail exactly these tests:

- `ion-platform::instance::tests::second_launch_hands_over_to_the_first`
- `ion-platform::instance::tests::simultaneous_launches_elect_one_primary`
- `ion-platform::instance::tests::stale_socket_is_replaced`

Unix-domain socket creation is rejected by the review environment with `Operation not permitted`. These failures were run and compared, not skipped or masked. They are unrelated to the five edited implementation files.

## Native and platform gaps

Full workspace tests and lint stop before ion-app can build with `Could not find Qt installation: Could not find Qt`. The qmake/qmllint commands present on PATH are wrappers pointing at missing Qt 5 binaries, not a working Qt 6 installation. The build also warns about missing mold/lld/gold. Nix is unavailable.

Full native compilation, Qt signal ordering, GUI layout, WebEngine request adoption, packaging, macOS and the declared Rust 1.88 MSRV were not verified. No passing CI result is inferred from publishing this branch.

The navigation reproducer executes QML functions using fake local renderers. It supports the source diagnosis for ION-01 and ION-05; it does not validate a Qt fix. The earlier navigation candidate is excluded from this branch, and its candidate native tests remain unrun.

## Regression scope

The five external probes use public APIs to check incomplete stream rejection, duplicate history title selection, preservation of pre-existing temporary files, known action targets in detailed timeline lines, and omission of placeholder notes after persistence. The identical probe source compiled against both revisions, including a compatibility adapter for the intentional Stream::finish return-type change. Unit tests in the implementation commits also cover legitimate completed streams, title orderings, collision retries/exhaustion, private file permissions and symlink behavior, missing-target wording, and note editing after draft creation.

The separately delivered portable handoff retains complete logs and the external probe runner. Only this concise repository-specific summary is committed here. The documentation publication changes Markdown files only; it does not alter or retest implementation behavior.
