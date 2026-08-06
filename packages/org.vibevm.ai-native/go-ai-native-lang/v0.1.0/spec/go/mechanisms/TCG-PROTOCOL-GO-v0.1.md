# TCG-PROTOCOL-GO v0.1 — the go relay's wire contract {#root}

<status stage="spec" state="done"/>

@fact:status-line **Status: v0.1 — authored with GO-AI-NATIVE-PLAN v0.1 (Phase 3),
implemented by its Phase 7.** @status:impl/done

@fact:companion-document The process model is
[`TCG-ORACLE-GO-v0.1`](TCG-ORACLE-GO-v0.1.md). @status:impl/done

@fact:DOCUMENT-OWNS-THE-OUTER-HOP-GRAMMAR This document owns the
OUTER hop's message grammar — host (vibe-tcg / one-shot CLI) ⇄
`go-ai-native-tcg serve` — and the mapping of each operation onto the
INNER hop, which is not a bespoke protocol but LSP 3.17 spoken to the
consumer's gopls. @status:impl/done

## 1. Wire parity with the TS and Rust protocols {#parity}

@fact:kind-line-parity `req r1` @status:impl/done

@fact:wire-parity-lead The outer hop is WIRE-PARITY with TCG-PROTOCOL v0.1 §1 (TS) and
TCG-PROTOCOL-RUST v0.1 §1: @status:impl/done

- @fact:PARITY-NDJSON-DUPLEX NDJSON duplex, one JSON object per line, @status:impl/done
- @fact:PARITY-FRAME-SHAPES `{proto, id, op, params}` requests and `{proto, id, ok, result |
  error}` responses, @status:impl/done
- @fact:PARITY-CALLER-CHOSEN-IDS caller-chosen correlation ids, @status:impl/done
- @fact:PARITY-ORACLE-PROTOCOL-CONSTANT `ORACLE_PROTOCOL = 1`
  on every frame, @status:impl/done
- @fact:PARITY-ADDITIVE-ONLY-EVOLUTION additive-only evolution within a proto (new optional
  params, new response fields — non-breaking; renames/semantic changes
  bump the constant). @status:impl/done

@fact:ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS One language-generic product client (`vibe-tcg`'s
`OracleRegistry` link) drives all three relays with the same frames;
the parity is pinned per-package by outer-frame replay goldens.
*Specified, not built (→ B-046) — the client named is gone by the owner's
own MCP-SOVEREIGNTY resolution (2026-07-07), and on the Go side neither
pinning mechanism exists. The `vibe-tcg` registry crate was retired with
the whole multiplexed-product topology and DELETED (PROP-026 in
vibe-mcp, `##TOPOLOGY-RETIRED` and `##TCG-CRATE-DELETED`);
`OracleRegistry` appears in no source file of any language. There is no
Go `live_chain.rs` (`go-ai-native-mcp`'s tests carry `server_replay.rs`
only) and no outer-frame golden (`##REPLAY-GOLDENS-PIN-BOTH-HOPS`). The
surviving posture is one layer up — `pub struct TcgSession`
(`go-ai-native-mcp/crates/go-ai-native-mcp/src/tools_tcg.rs:32`) — and
the planned successor to the one-client story is the multi-language
composition layer over the sovereign servers (`BACKLOG.md` B-046:
autodiscovery, autonomy preserved).* @status:impl/plan

@fact:RESTATES-RATHER-THAN-INCLUDES-THE-SIBLING-TEXTS This
document restates rather than includes the sibling texts
(cross-package spec inclusion is not a mechanism we have); every
DELIBERATE delta from the shared shape is listed in §3–§4, and
anything not listed there is parity by definition — drift outside that
list is a bug. @status:impl/done

## 2. Operations {#ops}

@fact:kind-line-ops `req r1` @status:impl/done

@fact:POSITIONS-ARE-ONE-BASED-LINE-ZERO-BASED-CHARACTER Positions are `{line, character}`, 1-based line, 0-based character —
UNCHANGED from the shared shape for parity; the bridge converts to
LSP's 0-based lines and, when utf-8 encoding was not granted, to
UTF-16 code units through the line's text. @status:impl/done

@fact:PATHS-ARE-ROOT-RELATIVE-WITH-FORWARD-SLASHES Paths are
project-root-relative with forward slashes. @status:impl/done

- @fact:OP-INIT **`init`** `{}` → `{gopls_version, position_encoding,
  pull_diagnostics, ready}` — resolves and spawns gopls (ORACLE-GO §1),
  negotiates capabilities (§2), applies §3 config, waits for readiness
  bounded by a deadline. Re-`init` on a live session restarts the
  child; overlays are cleared. The relay self-inits at `serve` start,
  so a host's first frame may be any op. The relay serves ONE project,
  so `init` takes no parameters — the root is `serve`'s own process
  root. @status:impl/done
- @fact:OP-UPDATE **`update`** `{file, content | null}` → `{version}` — set/replace an
  overlay (`didOpen`/`didChange`, monotonic version) or clear it
  (`didClose`). @status:impl/done
- @fact:OP-VALIDATE **`validate`** `{file, content?}` → `{diagnostics: [{code, category,
  message, line, character}], facts: [/* serde `Fact` records */],
  markers: [{tag, uri, reason, symbol, line}], conform_findings:
  [{rule, message, line, baselined}], advice: [string], degraded}` —
  diagnostics for the ONE document per ORACLE-GO §2; facts and markers
  from the go-extract fact vocabulary over the effective text
  (`item`, `import`, `go_unsafe`, `file_metrics`; markers are the
  `//spec:` directive stream — Go fills the field the Rust relay
  reserves empty, because the extractor already emits them);
  `conform_findings`/`advice` per §3. @status:impl/done
- @fact:OP-SCOPE **`scope`** `{file, position?}` → `{symbols: [{name, kind,
  type_text}], cell, seam_file, branded: [{name, seam, heuristic}]}` —
  symbols via a completion sweep at the position (or a neutral
  top-level position); `cell` is the package path relative to the
  policy's `cells_dir`; `seam_file` is the seams package's directory;
  `branded` carries the GO brand analog — exported DEFINED TYPES over
  primitives declared in seam files (`type AccountID string`),
  go-extract-detected, every entry `heuristic: true`. @status:impl/done
- @fact:OP-COMPLETE **`complete`** `{file, position, content?, prefix?, max?}` →
  `{entries: [{name, kind, type_text, unsafe}]}` — LSP completion;
  `prefix` filters and `max` caps (default 50) BEFORE per-entry detail
  work; `unsafe: true` flags entries that would land a §7-banned form
  in domain code (v0.1: ambient-default identifiers — `os.Getenv`,
  `time.Now`, `http.DefaultClient`-class — offered inside a cell file;
  name-based heuristic, honestly labelled in the brief). @status:impl/done
- @fact:OP-TYPE **`type`** `{file, position, content?}` → `{display,
  documentation}` — LSP hover, markdown stripped to text. @status:impl/done
- @fact:OP-SHUTDOWN **`shutdown`** `{}` → `{}` then exit 0 (the LSP shutdown/exit dance
  toward the child, kill-on-drop as backstop). @status:impl/done

## 3. The enrichment fields (in-process, same engine as the gate) {#enrichment}

@fact:kind-line-enrichment `req r1` @status:impl/done

@fact:SERVE-ASSEMBLES-THE-GATES-OWN-RULE-SET `go-ai-native-tcg serve` reads the project's `conform.toml` once per
init (config-or-default, origin printed to stderr) and assembles THE
GATE'S OWN rule set through the `go_ai_native_conform::build_rules`
pub seam. @status:impl/done

@fact:VALIDATE-ENRICHMENT-PIPELINE On `validate`: the effective text → the go-extract sidecar
(`facts` + `markers`) → `conform_core::check` → `conform_findings`,
each flagged `baselined` against the project's frozen ratchet
baseline, plus `advice` strings in Class-F form citing GUIDE REQs
(an `init()` or ambient call in a cell → §2 + the capability-injection
recipe; a missing Example on a new exported seam item → §4 Class G; a
seam error type without a REQ-citing message → §5; a file over the
length budget → §3). @status:impl/done

@fact:FINDING-PARITY-TEST-CATCHES-DRIFT The package/cell strings for the single file are
computed by a relay-local mapping mirroring the engine's, and a
finding-parity test diffs the relay's finding set against
`go-ai-native-conform check` on the same demo file — drift is a red
test, not a silent lie. @status:impl/done

@fact:ENRICHMENT-FIELDS-EXIST-ONLY-ON-THIS-HOP A consumer that talks to gopls directly gets
LSP; the enrichment fields exist only on this hop, and their absence
means "no policy layer", not an error. @status:impl/done

## 4. Error taxonomy {#errors}

@fact:kind-line-error-taxonomy `req r1` @status:impl/done

@fact:five-error-kinds-lead Five kinds, each actionable, mirrored as typed variants in
`go-ai-native-tcg-bridge`; the two environment rows are the DELIBERATE
renames against the shared table (§1): @status:impl/done

| kind | meaning | recipe carried |
|---|---|---|
| @fact:ROW-ERROR-GOPLS-MISSING `gopls-missing` @status:impl/done | no gopls resolvable (ORACLE-GO §1) @status:impl/done | `go install golang.org/x/tools/gopls@latest` @status:impl/done |
| @fact:ROW-ERROR-WORKSPACE-UNLOADABLE `workspace-unloadable` @status:impl/done | the project failed to load (no go.mod, `go env` failed) @status:impl/done | check `go env` / `go list ./...` standalone @status:impl/done |
| @fact:ROW-ERROR-ORACLE-CRASHED `oracle-crashed` @status:impl/done | child died / stream closed mid-session @status:impl/done | respawn guidance; the host registry retries once @status:impl/done |
| @fact:ROW-ERROR-PROTOCOL `protocol` @status:impl/done | unparseable frame, proto mismatch, unknown op @status:impl/done | version note; unknown-op errors list the known ops @status:impl/done |
| @fact:ROW-ERROR-TIMEOUT `timeout` @status:impl/done | no response within the caller's budget @status:impl/done | the op and the budget, for tuning @status:impl/done |

@fact:RENAMES-RIDE-WITHOUT-A-PRODUCT-EDIT The product's link layer special-cases only `oracle-crashed`
(session-grain) and passes every other kind through as a
recipe-carrying detail — the same contract the TS and Rust relays
proved, so the renames ride WITHOUT a product edit. @status:impl/done

## 5. Compatibility rules {#compat}

@fact:kind-line-compat `req r1` @status:impl/done

@fact:identical-to-the-sibling-protocols-lead Identical to the sibling protocols' §5, restated: @status:impl/done

- @fact:COMPAT-ADDITIVE-EVOLUTION additive evolution
  within a `proto` (optional params, new response fields, new
  advice/finding entries — consumers ignore the unknown); @status:impl/done
- @fact:COMPAT-BREAKING-CHANGES-BUMP-THE-CONSTANT renames, type
  changes, and semantic changes bump `ORACLE_PROTOCOL` and the bridge
  treats the mismatch as its own error class. @status:impl/done

@fact:REPLAY-GOLDENS-PIN-BOTH-HOPS Replay goldens pin the
CURRENT outer shape in this package's tests; recorded LSP transcripts
pin the inner hop the same way (both gopls-free in the unit suite).
*Specified, not built — the outer half. The inner hop is real and
gopls-free (`crates/go-ai-native-tcg-bridge/src/client/tests.rs`,
`Script`); the outer hop — `host ⇄ go-ai-native-tcg serve`, the grammar
this document owns (`##DOCUMENT-OWNS-THE-OUTER-HOP-GRAMMAR`) — is pinned
by nothing: `run_serve` (`serve.rs:227`) is called from `main.rs` alone,
no test constructs a `{proto, id, op, params}` frame, and no recorded
stream is checked in.* @status:spec/done

@fact:MARKERS-FIELD-IS-FILLED-HERE The `markers` field is FILLED here (unlike the Rust relay's reserved
empty array) — that is a per-language capability difference inside the
shared shape, not a protocol fork. @status:impl/done
