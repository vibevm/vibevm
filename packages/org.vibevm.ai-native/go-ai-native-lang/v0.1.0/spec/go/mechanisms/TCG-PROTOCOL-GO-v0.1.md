# TCG-PROTOCOL-GO v0.1 — the go relay's wire contract {#root}

<status stage="spec" state="done"/>

##status-line **Status: v0.1 — authored with GO-AI-NATIVE-PLAN v0.1 (Phase 3),
implemented by its Phase 7.** @impl/done

##companion-document The process model is
[`TCG-ORACLE-GO-v0.1`](TCG-ORACLE-GO-v0.1.md). @impl/done

##DOCUMENT-OWNS-THE-OUTER-HOP-GRAMMAR This document owns the
OUTER hop's message grammar — host (vibe-tcg / one-shot CLI) ⇄
`go-ai-native-tcg serve` — and the mapping of each operation onto the
INNER hop, which is not a bespoke protocol but LSP 3.17 spoken to the
consumer's gopls. @impl/done

## 1. Wire parity with the TS and Rust protocols {#parity}

##kind-line-parity `req r1` @impl/done

##wire-parity-lead The outer hop is WIRE-PARITY with TCG-PROTOCOL v0.1 §1 (TS) and
TCG-PROTOCOL-RUST v0.1 §1: @impl/done

- ##PARITY-NDJSON-DUPLEX NDJSON duplex, one JSON object per line, @impl/done
- ##PARITY-FRAME-SHAPES `{proto, id, op, params}` requests and `{proto, id, ok, result |
  error}` responses, @impl/done
- ##PARITY-CALLER-CHOSEN-IDS caller-chosen correlation ids, @impl/done
- ##PARITY-ORACLE-PROTOCOL-CONSTANT `ORACLE_PROTOCOL = 1`
  on every frame, @impl/done
- ##PARITY-ADDITIVE-ONLY-EVOLUTION additive-only evolution within a proto (new optional
  params, new response fields — non-breaking; renames/semantic changes
  bump the constant). @impl/done

##ONE-PRODUCT-CLIENT-DRIVES-ALL-THREE-RELAYS One language-generic product client (`vibe-tcg`'s
`OracleRegistry` link) drives all three relays with the same frames;
the parity is pinned per-package by outer-frame replay goldens. @impl/done

##RESTATES-RATHER-THAN-INCLUDES-THE-SIBLING-TEXTS This
document restates rather than includes the sibling texts
(cross-package spec inclusion is not a mechanism we have); every
DELIBERATE delta from the shared shape is listed in §3–§4, and
anything not listed there is parity by definition — drift outside that
list is a bug. @impl/done

## 2. Operations {#ops}

##kind-line-ops `req r1` @impl/done

##POSITIONS-ARE-ONE-BASED-LINE-ZERO-BASED-CHARACTER Positions are `{line, character}`, 1-based line, 0-based character —
UNCHANGED from the shared shape for parity; the bridge converts to
LSP's 0-based lines and, when utf-8 encoding was not granted, to
UTF-16 code units through the line's text. @impl/done

##PATHS-ARE-ROOT-RELATIVE-WITH-FORWARD-SLASHES Paths are
project-root-relative with forward slashes. @impl/done

- ##OP-INIT **`init`** `{}` → `{gopls_version, position_encoding,
  pull_diagnostics, ready}` — resolves and spawns gopls (ORACLE-GO §1),
  negotiates capabilities (§2), applies §3 config, waits for readiness
  bounded by a deadline. Re-`init` on a live session restarts the
  child; overlays are cleared. The relay self-inits at `serve` start,
  so a host's first frame may be any op. The relay serves ONE project,
  so `init` takes no parameters — the root is `serve`'s own process
  root. @impl/done
- ##OP-UPDATE **`update`** `{file, content | null}` → `{version}` — set/replace an
  overlay (`didOpen`/`didChange`, monotonic version) or clear it
  (`didClose`). @impl/done
- ##OP-VALIDATE **`validate`** `{file, content?}` → `{diagnostics: [{code, category,
  message, line, character}], facts: [/* serde `Fact` records */],
  markers: [{tag, uri, reason, symbol, line}], conform_findings:
  [{rule, message, line, baselined}], advice: [string], degraded}` —
  diagnostics for the ONE document per ORACLE-GO §2; facts and markers
  from the go-extract fact vocabulary over the effective text
  (`item`, `import`, `go_unsafe`, `file_metrics`; markers are the
  `//spec:` directive stream — Go fills the field the Rust relay
  reserves empty, because the extractor already emits them);
  `conform_findings`/`advice` per §3. @impl/done
- ##OP-SCOPE **`scope`** `{file, position?}` → `{symbols: [{name, kind,
  type_text}], cell, seam_file, branded: [{name, seam, heuristic}]}` —
  symbols via a completion sweep at the position (or a neutral
  top-level position); `cell` is the package path relative to the
  policy's `cells_dir`; `seam_file` is the seams package's directory;
  `branded` carries the GO brand analog — exported DEFINED TYPES over
  primitives declared in seam files (`type AccountID string`),
  go-extract-detected, every entry `heuristic: true`. @impl/done
- ##OP-COMPLETE **`complete`** `{file, position, content?, prefix?, max?}` →
  `{entries: [{name, kind, type_text, unsafe}]}` — LSP completion;
  `prefix` filters and `max` caps (default 50) BEFORE per-entry detail
  work; `unsafe: true` flags entries that would land a §7-banned form
  in domain code (v0.1: ambient-default identifiers — `os.Getenv`,
  `time.Now`, `http.DefaultClient`-class — offered inside a cell file;
  name-based heuristic, honestly labelled in the brief). @impl/done
- ##OP-TYPE **`type`** `{file, position, content?}` → `{display,
  documentation}` — LSP hover, markdown stripped to text. @impl/done
- ##OP-SHUTDOWN **`shutdown`** `{}` → `{}` then exit 0 (the LSP shutdown/exit dance
  toward the child, kill-on-drop as backstop). @impl/done

## 3. The enrichment fields (in-process, same engine as the gate) {#enrichment}

##kind-line-enrichment `req r1` @impl/done

##SERVE-ASSEMBLES-THE-GATES-OWN-RULE-SET `go-ai-native-tcg serve` reads the project's `conform.toml` once per
init (config-or-default, origin printed to stderr) and assembles THE
GATE'S OWN rule set through the `go_ai_native_conform::build_rules`
pub seam. @impl/done

##VALIDATE-ENRICHMENT-PIPELINE On `validate`: the effective text → the go-extract sidecar
(`facts` + `markers`) → `conform_core::check` → `conform_findings`,
each flagged `baselined` against the project's frozen ratchet
baseline, plus `advice` strings in Class-F form citing GUIDE REQs
(an `init()` or ambient call in a cell → §2 + the capability-injection
recipe; a missing Example on a new exported seam item → §4 Class G; a
seam error type without a REQ-citing message → §5; a file over the
length budget → §3). @impl/done

##FINDING-PARITY-TEST-CATCHES-DRIFT The package/cell strings for the single file are
computed by a relay-local mapping mirroring the engine's, and a
finding-parity test diffs the relay's finding set against
`go-ai-native-conform check` on the same demo file — drift is a red
test, not a silent lie. @impl/done

##ENRICHMENT-FIELDS-EXIST-ONLY-ON-THIS-HOP A consumer that talks to gopls directly gets
LSP; the enrichment fields exist only on this hop, and their absence
means "no policy layer", not an error. @impl/done

## 4. Error taxonomy {#errors}

##kind-line-error-taxonomy `req r1` @impl/done

##five-error-kinds-lead Five kinds, each actionable, mirrored as typed variants in
`go-ai-native-tcg-bridge`; the two environment rows are the DELIBERATE
renames against the shared table (§1): @impl/done

| kind | meaning | recipe carried |
|---|---|---|
| ##ROW-ERROR-GOPLS-MISSING `gopls-missing` @impl/done | no gopls resolvable (ORACLE-GO §1) @impl/done | `go install golang.org/x/tools/gopls@latest` @impl/done |
| ##ROW-ERROR-WORKSPACE-UNLOADABLE `workspace-unloadable` @impl/done | the project failed to load (no go.mod, `go env` failed) @impl/done | check `go env` / `go list ./...` standalone @impl/done |
| ##ROW-ERROR-ORACLE-CRASHED `oracle-crashed` @impl/done | child died / stream closed mid-session @impl/done | respawn guidance; the host registry retries once @impl/done |
| ##ROW-ERROR-PROTOCOL `protocol` @impl/done | unparseable frame, proto mismatch, unknown op @impl/done | version note; unknown-op errors list the known ops @impl/done |
| ##ROW-ERROR-TIMEOUT `timeout` @impl/done | no response within the caller's budget @impl/done | the op and the budget, for tuning @impl/done |

##RENAMES-RIDE-WITHOUT-A-PRODUCT-EDIT The product's link layer special-cases only `oracle-crashed`
(session-grain) and passes every other kind through as a
recipe-carrying detail — the same contract the TS and Rust relays
proved, so the renames ride WITHOUT a product edit. @impl/done

## 5. Compatibility rules {#compat}

##kind-line-compat `req r1` @impl/done

##identical-to-the-sibling-protocols-lead Identical to the sibling protocols' §5, restated: @impl/done

- ##COMPAT-ADDITIVE-EVOLUTION additive evolution
  within a `proto` (optional params, new response fields, new
  advice/finding entries — consumers ignore the unknown); @impl/done
- ##COMPAT-BREAKING-CHANGES-BUMP-THE-CONSTANT renames, type
  changes, and semantic changes bump `ORACLE_PROTOCOL` and the bridge
  treats the mismatch as its own error class. @impl/done

##REPLAY-GOLDENS-PIN-BOTH-HOPS Replay goldens pin the
CURRENT outer shape in this package's tests; recorded LSP transcripts
pin the inner hop the same way (both gopls-free in the unit suite). @impl/done

##MARKERS-FIELD-IS-FILLED-HERE The `markers` field is FILLED here (unlike the Rust relay's reserved
empty array) — that is a per-language capability difference inside the
shared shape, not a protocol fork. @impl/done
