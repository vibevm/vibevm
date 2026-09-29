# M-16-B.1 finite affected-source review

Observed immutable endpoints: `77f6767f3c864029a64679e1f7e8796b877afddc` → `2292501f353bb3dde00e2777b8b4b652dff6480c`. The retained selected-file captures hold 3894 → 3939 files: 45 newly selected, 76 changed, 0 absent. Both have 48 canonical package objects, byte-equal as structured objects. Newly selected is a capture-set statement: it includes an existing Cargo manifest that was outside the earlier selected set.

The per-commit first-parent product union contains **122 paths** across 22 product-bearing commits. 8 additional commits changed only paths under inherited exclusions. The endpoint captures include 121 of these product paths; `vibevm/vibespecs/boot/90-user.xml` is outside both capture file lists and was separately bound to immutable Git blobs:

| Endpoint | Git blob | SHA-256 |
| --- | --- | --- |
| `77f6767f3c864029a64679e1f7e8796b877afddc` | `fb1c6386e2859a6e10b4085daf63ac60b055e234` | `aed268136df3c0f22eea34c864529397915e520af7bbd64c00a03f7b79ae54ec` |
| `2292501f353bb3dde00e2777b8b4b652dff6480c` | `7b945751676e7f7665dcb24c4757a5ac3c8feaab` | `bac556c5d121fda85ad9d14275f1f21d8b56e8d9396af4b8cdbce97c8f926549` |

The 122-path review is partitioned by exact changed-path membership. These groups overlap only where one commit/path really supports more than one interface; they are not an exclusive task ownership claim.

| Candidate change record | Commits | Product paths |
| --- | ---: | ---: |
| `machine-operation-contract-source` | 3 | 2 |
| `machine-operation-wire-and-reader` | 2 | 68 |
| `authored-extension-schema-and-selected-observation` | 3 | 37 |
| `pure-machine-service-and-envelope-tool` | 5 | 10 |
| `mcp-query-input-refusal` | 1 | 2 |
| `machine-capability-catalogue-and-gap-registry` | 5 | 5 |
| `machine-operation-traceability` | 4 | 1 |
| `machine-operation-operational-status-guidance` | 2 | 1 |

The source law and generated roots are separate from runtime implementation. The extension table and selected-world projection remain descriptor-only without a native load. The neutral service validates one pure migration planner route; all other catalogue operation/model entries remain typed extraction gaps. The strict MCP query repair is a separate existing-tool effect. Regenerated non-machine namespace wrappers and the specmap index are treated as source/traceability changes, not new callable operations. The user-owned boot status rule changes operator reporting only.

This is a finite endpoint and relationship review. It does not prove exhaustive semantic API coverage, physical source-byte authentication, all-platform execution, native macOS/Windows filesystem behavior, public publication, or Developer Preview 2 completion.
