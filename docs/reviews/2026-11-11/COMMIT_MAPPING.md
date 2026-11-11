# Commit mapping

Reviewed base: `7e6b747301142d9ae2001d380a59043db781d255`.
Published implementation head: `b82cccb4507163bbb24c21c906c9a09559508835`.

Publication through the authenticated GitHub API preserved five separate implementation commits. Commit metadata changed the IDs. Each tree below was verified to be identical to the corresponding reviewed local tree.

| Original local commit | Published implementation commit | Identical tree |
|---|---|---|
| `296b5c7fa1b639a135ca419475b69a401e92ff38` | [`d6630f6d8e8b8c221db420dace7fa9cead09d338`](https://github.com/Mischa-dev/ion/commit/d6630f6d8e8b8c221db420dace7fa9cead09d338) | `61d94a42fb3e309b948edf76a838f228eaff02bd` |
| `f188149de127a4e9c7e86eb17dff41655ec1e0c2` | [`46ba9aca640d633050b6351ab2b9b03fb6efe664`](https://github.com/Mischa-dev/ion/commit/46ba9aca640d633050b6351ab2b9b03fb6efe664) | `554709308539744102f752355f7a8f2e80903b6c` |
| `7783053eab048e755afd07e2ce7d8b78711ee519` | [`95bd92e0d2976bedaf40c4b6052e6a72dbcd9160`](https://github.com/Mischa-dev/ion/commit/95bd92e0d2976bedaf40c4b6052e6a72dbcd9160) | `9d587eb110c1667d2c44974f77e4520abdc9bf7f` |
| `653a6fef0c365b96d51a84bb6a826174338f8235` | [`7963e16fba3fddab7f93955f91dcf392e606369e`](https://github.com/Mischa-dev/ion/commit/7963e16fba3fddab7f93955f91dcf392e606369e) | `db604405777cab798412ee4cd902982e61414236` |
| `eb3733078822d404512e4ff62995a571e7f4db8a` | [`b82cccb4507163bbb24c21c906c9a09559508835`](https://github.com/Mischa-dev/ion/commit/b82cccb4507163bbb24c21c906c9a09559508835) | `50c6602e4dc04ea87d360495c8d467602550cf88` |

The first published commit has the exact reviewed base as its parent. Each later published commit has the previous published commit as its parent. GitHub comparison confirmed exactly these five changed implementation files, with the expected blob SHAs:

| Path | Reviewed and published blob |
|---|---|
| `crates/ion-agent/src/openai.rs` | `3271808e42221be6922001553b535d017e99a99a` |
| `crates/ion-panels/src/notes.rs` | `f387255c5d91fb21023c391d1958d00eead0745a` |
| `crates/ion-safety/src/timeline.rs` | `b6c0623a5feed71e2aef2a55f58ff618cb26a1e3` |
| `crates/ion-session/src/history.rs` | `a86c7186fb38e1610f4372584990da5c536a661a` |
| `crates/ion-session/src/paths.rs` | `04918ccbc2761bb2eb40c47aed69219ce8615c2b` |

The subsequent documentation-only commit adds this review directory to the dedicated branch. It has no original local implementation counterpart. The original local bundle and archive remain unchanged.

[Implementation comparison](https://github.com/Mischa-dev/ion/compare/7e6b747301142d9ae2001d380a59043db781d255...b82cccb4507163bbb24c21c906c9a09559508835)
[Review branch](https://github.com/Mischa-dev/ion/tree/review/ion-findings-20261111)
