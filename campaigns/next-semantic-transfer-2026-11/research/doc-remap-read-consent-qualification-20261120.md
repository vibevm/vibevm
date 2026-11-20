# Doc read and Remap consent — separate public qualification

Contract `NST-DOC-REMAP-DATA-QUALIFY-01` (architect `5aa22306a6eb5e8dba7912945caf5975ce3528ef`, locally integrated `28f426628`). Product `eb370713f8419ef28c4d9e86856e6f557ef47b2a`. Test/report only, no production input channel or installer.

## Actual distinctions

| Subject | Shared typed on/off choice | Actual public operation | Result / first owner |
|---|---|---|---|
| Valid in-tree Doc | off empty selected data; on `selected=true` | registered `read_doc`, valid page and provenance | Reads page, `source=in-tree`, `format=md`, no install effects |
| Doc with extra common input attempt | same independent kernel control | actual `tools/call read_doc` with on/off `input` | **silently ignored**, entire response unchanged, not choice consumption or explicit rejection |
| Seeded synthetic Remap offer | same data-only evaluator control | offline list/explain before consent | offer only, read-only, no consent persistence |
| Remap common input attempt | no public setter | `remap list --ordinary-answers ...` | CLI argument refusal; no state/consent change |
| Remap explicit accept/reject | independent of data-only choice | actual accept/reject verbs | owned hash-bound ledger/provider transitions, not common choice consumption |

Doc source contains a valid page and data-only configuration definition. Runtime `crates/vibe-mcp/src/tools/read_doc.rs:70` reads address/format/lang, not common input; its descriptor advertises `additionalProperties=false`, but actual dispatch does not enforce that descriptor against extra fields. The observed silent ignore is not presented as strict refusal or successful configuration. `vibe_doc::agent::read_page` receives page addressing/projection facts, no selected configuration argument. The typed evaluator control changes selected data without changing authored page bytes or granting read-request authority.

Remap `crates/vibe-cli/src/cli/remap.rs:24` and existing read/consent owners expose no common choice setter. A typed `InstalledRemaps` fixture seed is used only to make a local offer available without installing or probing anything; it is **not an actual install receipt**. The offered section does not carry a selected common configuration record. `offer_from_manifest` copies remap/policy data, not arbitrary evaluated choice effects. Explicit accept/reject uses existing owned state; list/explain never creates acceptance. Comparing offered and accepted explain results demonstrates consent activation separately from shared data choice.

## Evidence and limits

Initial actual Doc1 and Remap1 selectors passed in enforced bwrap isolation. Full project and complete synthetic UserScratch snapshots remain unchanged around reads; no package lock, slots, hooks or boot generation is invoked. Only explicit consent verbs may mutate synthetic home remap ledger/provider files. Final fixture adds explicit reject after accept and checks reads remain nonmutating after rejection. Full public108/108 passed with zero filtered (`read-consent-public-all-r1.log`), including actual tools/list descriptor and registered read_doc calls. Strict fixture lint/diff passed. Unchanged CLI SHA-256 `695ffc89e2470a7d8f58afdfccb4487f0e9e2cb923c87754d0004abc1a702177`; final fixture `528a48fee064c2894c8dfe16f9fe9087b9419ad136c0950db4a42d99f5bb45e0`. Independent disposition pending.

No service, server, compiler, tool, package install or target probe is run. Registered MCP server is the Vibe transport under test, not a package-authored MCP service. No automatic consent or real credentials. Kernel data evaluation, public read, offer availability and acceptance are four different claims. Neither ordinary Doc nor Remap public common-input consumption is established; these remain named gaps. No parent/NST/USER-PROJECT promotion; submitted publication-fault injection and broad-bin strict enum lint remain separate outstanding limits.
