# M-16-B.2 selected verification

Observed affected owner checks at the local 1.1.0 checkout. Each retained command exited 0; the raw logs below are copied byte-for-byte. This is a focused B.2 panel, not a workspace-wide or native-platform run.

| Check | Passed | Raw log SHA-256 |
| --- | ---: | --- |
| `core` | 5 | `3ca018b72aa0a32e307a630ee4c823556a6cc7c43d99cdfe5acbc6da9b07b468` (`entity-view-b2-core.log`) |
| `wire-contract` | 3 | `b9d9846c5d8082ad2c6af3e50f7c44f030ebec534cfba7f45e78c38b8ecef50b` (`entity-view-b2-wire-contract.log`) |
| `wire-behaviour` | 6 | `bf4447a5f55dd32c3200e7ee227e330e3cab493d2f5fb15429365dd65d00dc99` (`entity-view-b2-wire-behaviour.log`) |
| `orchestrator` | 18 | `e43a62af36958f1d546c05ee216a22a638526f6a6fa3bdfb156dd0bb35fc92cf` (`entity-view-b2-orchestrator.log`) |
| `catalogue` | 4 | `95c7de8c1714b4dbfddbb801f59f1a5eb0cbad069020fe4e090df452cceff56e` (`entity-view-b2-catalogue.log`) |

Core cases cover repeated logical identity with distinct occurrence IDs, exact non-UTF8 resource octets and refusal to report projected Markdown as raw XML bytes. Wire cases cover four new generated roots, required-nullable/unknown-field refusal, canonical scalar bounds and client-URI status syntax. Orchestrator cases cover declared composite compatibility, process-local one-use page issuance and complete fixed-row reconstruction, explicit source-to-capture and occurrence joins, scoped URI lookup/publication, and forged/mixed/partial refusals. The catalogue case covers 214 operation IDs, 35 model IDs, source references and the one reviewed pure planner binding.

The final specmap producer exited 0 with a proper entity rule revision r1→r2, zero suspects, zero gated orphans and 35 warnings; no semantic unbumped-hash warning remained. Canonical JTD codegen ran for each actual schema change; generated leaves were not hand-edited. Initial schema-annotation and test-borrow/codegen-order failures were corrected and their scratch logs retained. No mutation test ran because no remaining concrete mutation suspicion was identified.

Limits: entity captures and URI mappings are owner-declared and source bytes remain unauthenticated; the page session is process-local over caller-supplied retained rows; there is no live URI resolver, raw XML reverse map, persistent transport cursor, watcher, disk mutation, public Git publication or native macOS/Windows proof.
