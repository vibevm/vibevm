# Six agent packages: retained acquisition and native-platform outcomes

Read-only reconciliation at `106cc5c8fe3389c26c7acc1644cc06dc935df9af`, 2026-11-20. Scope is the six identities required by `CORP-AGENTS-01.md`, not aliases counted as additional agents. No existing aggregate six-by-three package outcome table was found in the bounded CORP report set; this joins the existing accepted per-agent receipts without changing their outcomes. No downloads, supplier executions, other-OS commands, provider/auth access, global state changes or legal disposition changes were performed.

## Outcomes, not inferred platform support

`PASS` below means the retained exact Linux x86_64 supplier-package install → public version → repeat → uninstall sequence, with expected corrupt-digest refusal. It is not conversation, helper/sandbox completeness, online Vibe acquisition, current-binary rerun or redistribution clearance. `UNRUN` means no admitted corresponding native package lifecycle receipt in this bounded input set, not a failed product test or proof the supplier lacks that platform. Supplier metadata is retained historical data, not a current/latest availability lookup.

| Identity / pinned version | Linux x86_64 | Linux arm64 | macOS x86_64 / arm64 | Windows x86_64 / arm64 | Retained acquisition / package option | Redistribution disposition |
|---|---|---|---|---|---|---|
| Claude Code 2.1.328 (`claude` harness; package `claude-code`) | PASS, actual version `2.1.328 (Claude Code)` | UNRUN | UNRUN / UNRUN | UNRUN / UNRUN | Exact official npm parent/platform tarballs, SHA-512 verified; native payload, `linux-x64-native`; no npm/Node target prerequisite | Not approved for corporate bundling/hosting/redistribution; commercial/terms decision remains separate |
| Codex 0.170.0 | PASS, `codex-cli 0.170.0` | UNRUN | UNRUN / UNRUN | UNRUN / UNRUN | Exact supplier musl CLI archive, official size/SHA-256; `linux-x64-musl`; helper assets not included by inference | Unapproved component/notice closure; root Apache-2.0 and retained NOTICE do not settle bundled binary obligations |
| OpenCode 1.19.6 | PASS, `1.19.6` | UNRUN | UNRUN / UNRUN | UNRUN / UNRUN | Exact supplier default Linux x64 archive, official size/SHA-256; root executable, `linux-x64-standalone` | HOLD corporate drop/redistribution; retain component/notices uncertainty |
| Qwen Code 0.28.0 (`qwen`, one identity) | PASS, `0.28.0` | UNRUN | UNRUN / UNRUN | UNRUN / UNRUN | Exact **CLI** supplier archive, official size/SHA-256; complete bundled Node/resources, `linux-x64-standalone` | Unapproved composition/notice closure despite included notices and retained root license |
| Hermes 0.23.2 | Ordinary public CLI positive UNRUN; actual provenance refusal. Separate library-composition install/version/resource/repeat/uninstall PASS | UNRUN | UNRUN / UNRUN | UNRUN / UNRUN | Vibe-built private source+wheels archive, not fictitious upstream binary; `linux-x64-core-cp314`, system CPython3.14/ABI required; no target pip/uv | Unapproved source/wheel/native component closure and producer/distribution qualification |
| Pi 1.5.2 | PASS, `1.5.2` | UNRUN | UNRUN / UNRUN | UNRUN / UNRUN | Exact supplier standalone archive, official size/SHA-256; complete resources, `linux-x64-standalone`; not npm route | Unapproved component/notices closure; source MIT is not complete compiled-runtime clearance |

All six current canonical manifests still have **exactly one installer option: Linux / x86_64**. No macOS/Windows/arm64 Vibe package option or native lifecycle acceptance is manufactured from upstream asset names. GNU/musl and CPython ABI distinctions are not interchangeable architectures or portability proofs.

## Retained supplier metadata versus acquired runnable inputs

- Claude exact parent `parent-exact.json` lists x64/arm64 for Linux, Darwin and win32, plus Linux musl variants. Only the Linux x64 child is bound to the accepted package lifecycle here.
- Codex exact `rust-v0.170.0` release metadata lists CLI x86_64/aarch64 Linux musl, Darwin and Windows artifacts, plus separate helper/package families. Those names do not prove acquisition, layout or helper qualification.
- OpenCode exact `v1.19.6` metadata lists Linux x64/arm64 (including distinct musl/baseline variants), Darwin x64/arm64 and Windows x64/arm64. Desktop artifacts are not substituted for CLI assets.
- Qwen exact `metadata/cli-release.json` is `v0.28.0`: standalone Linux/Darwin x64/arm64 and Windows x64. Its generic `release.json` is the separately excluded SDK release. Windows arm64 CLI asset is not established by this retained CLI list; this is not a claim of global supplier absence.
- Pi exact `v1.5.2` metadata lists Linux/Darwin/Windows x64 and arm64. Only Linux x64 full bytes and lifecycle are accepted here.
- Hermes Linux private producer binds supplier commit `ae2a81ccc00e9b7c06301523e5bb09608397149c`, source archive, exact uv lock and 65 selected wheels to CPython3.14.4 / `cpython-314-x86_64-linux-gnu`. No macOS/Windows/arm64 producer or runtime closure is established by that Linux selection.

For the five supplier binaries, historical full acquisition/integrity evidence is distinct from the actual Vibe operation: Vibe installed via a private digest-keyed loopback remap serving retained exact bytes, with external networking disabled. **Actual online Vibe supplier acquisition remains UNRUN** for those receipts. None was copied into a distributable corporate drop by these qualifications.

## Exact receipt reconciliation

Canonical manifest SHA-256 values were rechecked against the existing reports: Claude `c0c98485d5c92c7411ea8730855b2a147bffa82a5b3d8c559fdc9b0c4598644a`; Codex `c458249222c61ffcabcec5641a192c5679466a680c36a906354d0fe0ea672936`; OpenCode `b6cb04ea47d4400ccc4c57f94bad67927756a6c28f381382b54bd17365196349`; Qwen `a8b4879a425563d9d2e8caf01cb024ee829bc468589292ccc275fbdfc8bf3823`; Hermes `69aa3de07b9c58d5014368976bf045e7b6438500a4605835a6fa145b247c127c`; Pi `18c1a6330abc6282d44fa7efe3c7d2b4562445cb7cbde450a5d3e93e221526b5`.

The five named `steps.json` files each retain actual exits install0/version0/repeat0/uninstall0/corrupt1. Their installed receipts exist. Their terminal lifecycle log SHA-256 values independently match the accepted per-agent reports:

| Identity | Lifecycle log SHA-256 |
|---|---|
| Claude | `7543b969e2dfd3427eb6fa3628dcefcbfeae8893175e2efc9535841c52b5e0d6` |
| Codex | `67bc5eb0d59871c39d9f966ff7d7250db4f4d9bf3aa86f53a901d6b1c34cbc95` |
| OpenCode | `a6b6a48adba84bb7feef47f1f1ca11f35189b950a4c7905bcdfb3c43d9e7c5c9` |
| Qwen | `00f4465ea68f36e7bad1ff9e5ddeb15fb04037496d5e0e7ef21fbe622f8256f5` |
| Pi | `161111e011efe765ebd64280a350590d0bb40552c2156dab96a73ec6dad9e373` |

Hermes is deliberately not a sixth ordinary CLI PASS. Its `consumer-lifecycle-r3/steps.json` retains actual CLI exit2 and separate outer-provenance library install/version; `continuation-steps.json` retains provider-free `plugins show chronos`, library repeat and uninstall. Latest public prerequisite `summary.json` preserves missing-Python and ownership refusals and explicitly labels ABI incompatibility, interpreter drift and real positive UNRUN. The namespace ownership failure is not ABI-incompatibility evidence. Private receipt paths/hashes are sealed in the candidate manifest, not broadened into arbitrary HOME/auth searches.

## Corrections to stale summaries, without rewriting history

The per-agent November19 package reports list proxy/API setup as then-remaining work. Later November20 proxy reports establish their own bounded native routing results; they do not change package-platform or redistribution cells. Metadata-only setup also has its separately accepted post-bootstrap adapter, not full onboarding. Do not replay those accepted atoms to make this matrix green.

The earlier OpenCode archive-review card's pre-execution wording describes its review-stage time. Later accepted supplier-package execution is the authoritative lifecycle evidence; it does **not** lift the card's redistribution HOLD. The unrelated dirty review card is untouched.

Pi's later npm inventory found 123 locked rows without admitted reconstruction artifacts in its named input roots and an actual public `--installer npm` `no_option` refusal. That is not a standalone package failure; actual npm-backed agent install remains UNRUN. Existing Node remediation/defer/Cancel tests are separate synthetic/prerequisite evidence, not six npm agent installations.

## What can proceed locally, and what needs a new resource

**Executable without a new external resource:** retained-source/receipt/hash reconciliation (performed here), existing credential-free synthetic admission/refusal/package-layout tests when a specific unproved cell warrants them, and separately contracted Linux requalification using admitted exact payloads and enforced isolation. The five accepted supplier lifecycles do not need repetition merely for this documentation join. No supplier code was executed in this atom.

**Needs a precise additional resource/decision:**

1. Hermes ordinary CLI positive/ABI/drift needs an admitted isolated runner preserving protected system-Python provenance; do not weaken the guard or substitute library composition.
2. Every other OS/architecture cell needs its exact admitted supplier/producer input, Vibe package option/layout and native isolated platform runner. Existing Vibe installer Windows/macOS proofs are not agent package proofs; prior elevated Windows SSH evidence is not ordinary-user acceptance.
3. Online Vibe acquisition needs its separately approved finite network/source-policy route. Private loopback installation is not that proof.
4. Hermes other-platform archives need native producers and exact interpreter/wheel/ABI closures; Pi npm needs admitted reconstruction artifacts and an explicit ordinary consumer option before installation tests.
5. Redistribution/drop/publication needs architect-owned component/notice/terms decisions for exact inventories plus the owner's applicable target/action authority. This report performs no new legal research or approval and grants no publication.

USER-PROJECT's unavailable authenticated runner does not block this noncredentialed reconciliation. Its mandatory behavioral gate remains RUNNER-BLOCKED/UNRUN, not failed, waived or replaced by a synthetic result. No parent states or NST completion changed.

Evidence navigation: existing `{CLAUDE,CODEX,OPENCODE,QWEN,PI}-SUPPLIER-PACKAGE-RESULT-20261119.md`, `HERMES-ARCHIVE-RESULT-20261119.md`, `HERMES-INTERPRETER-QUALIFICATION-20261120.md`, `PI-NPM-OFFLINE-CLOSURE-20261120.md`, and the separate November20 proxy/setup reports. This matrix is a bounded retained-evidence join, not six-by-three runtime acceptance.
