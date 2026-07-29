# TCG-PROTOCOL-RUST v0.1 — the rust relay's wire contract {#root}

<status stage="spec" state="done"/>

##status-line **Status: v0.1 — authored with AGENTIC-TCG-RUST-PLAN v0.1 (Phase 1),
implemented by its Phases 3–4.** @impl/done

##companion-document The process model is
[`TCG-ORACLE-RUST-v0.1`](TCG-ORACLE-RUST-v0.1.md). @impl/done

##DOCUMENT-OWNS-THE-OUTER-HOP-GRAMMAR This document owns
the OUTER hop's message grammar — host (vibe-tcg / one-shot CLI) ⇄
`rust-ai-native-tcg serve` — and the mapping of each operation onto the INNER
hop, which is not a bespoke protocol at all but LSP 3.17 spoken to the
consumer's rust-analyzer. @impl/done

## 1. Wire parity with the TS protocol {#parity}

##kind-line-parity `req r1` @impl/done

##outer-hop-is-wire-parity-lead The outer hop is WIRE-PARITY with the TS stack's TCG-PROTOCOL v0.1 §1: @impl/done

- ##PARITY-NDJSON-DUPLEX NDJSON duplex, @impl/done
- ##PARITY-ONE-JSON-OBJECT-PER-LINE one JSON object per line, @impl/done
- ##PARITY-REQUEST-AND-RESPONSE-ENVELOPES `{proto, id, op, params}`
  requests and `{proto, id, ok, result | error}` responses, @impl/done
- ##PARITY-CALLER-CHOSEN-CORRELATION-IDS caller-chosen
  correlation ids, @impl/done
- ##PARITY-ORACLE-PROTOCOL-ON-EVERY-FRAME `ORACLE_PROTOCOL = 1` on every frame, @impl/done
- ##PARITY-ADDITIVE-ONLY-EVOLUTION additive-only evolution within a proto (new optional params, new response fields —
  non-breaking; renames/semantic changes bump the constant). @impl/done

##ONE-PRODUCT-CLIENT-DRIVES-BOTH-RELAYS One
language-generic product client (`vibe-tcg`'s `OracleRegistry` link)
drives BOTH relays with the same frames — the parity is enforced at
the product level by the two live-chain tests, and pinned per-package
by outer-frame replay goldens. @impl/done

##RESTATES-RATHER-THAN-INCLUDES-THE-TS-TEXT This document restates rather than
includes the TS text (cross-package spec inclusion is not a mechanism
we have); every DELIBERATE delta from the TS shape is listed in §3–§4,
and anything not listed there is parity by definition — drift outside
that list is a bug. @impl/done

## 2. Operations {#ops}

##kind-line-ops `req r2` @impl/done

##POSITIONS-ARE-ONE-BASED-LINE-ZERO-BASED-CHARACTER Positions are `{line, character}`, 1-based line, 0-based character —
UNCHANGED from the TS shape for parity; the bridge converts to LSP's
0-based lines and, when utf-8 encoding was not granted, to UTF-16 code
units through the line's text. @impl/done

##PATHS-ARE-PROJECT-ROOT-RELATIVE Paths are project-root-relative with
forward slashes. @impl/done

- ##OP-INIT **`init`** `{root}` → `{ra_version, ra_path, toolchain, root_files,
  quiescent}` — resolves and spawns the analyzer (ORACLE-RUST §1),
  negotiates capabilities (§2), applies §3 config, waits for
  quiescence bounded by a deadline. Re-`init` on a live session
  restarts the child; overlays are cleared. The relay self-inits at
  `serve` start, so a host's first frame may be any op (client init
  frames remain re-init). @impl/done
- ##OP-UPDATE **`update`** `{file, content | null}` → `{version}` — set/replace an
  overlay (`didOpen`/`didChange`, monotonic version) or clear it
  (`didClose`). @impl/done
- ##OP-VALIDATE **`validate`** `{file, content?}` → `{diagnostics: [{code, category,
  message, line, character}], facts: [/* serde `Fact` records */],
  markers: [], conform_findings: [{rule, message, line, baselined}],
  advice: [string], degraded}` — diagnostics from the pull channel
  (`textDocument/diagnostic`) for the ONE document; facts from the
  in-process `RustFrontend::extract` over the effective text;
  `conform_findings`/`advice` per §4. `markers` is always present and
  always empty in v0.1 — reserved for parity with the TS shape (Rust's
  marker analog, the specmark tag stream, lives in the specmap engine
  and is not re-extracted here; the field exists so consumers written
  against either relay see one shape). @impl/done
- ##OP-SCOPE **`scope`** `{file, position?}` → `{symbols: [{name, kind,
  type_text}], cell, seam_file, branded: [{name, seam, heuristic}]}` —
  symbols via a completion sweep at the position (or a neutral
  top-level position); `cell` is the module path derived from the
  file's location under `src/` (there is no `[rust]` cells topology in
  conform.toml — derivation, never invented policy); `seam_file` is
  the enclosing `mod.rs`/`lib.rs`; `branded` carries the RUST brand
  analog — seam NEWTYPES (pub tuple structs with a single private
  field), syn-detected, every entry `heuristic: true`. @impl/done
- ##OP-COMPLETE **`complete`** `{file, position, content?, prefix?, max?}` →
  `{entries: [{name, kind, type_text, unsafe}]}` — LSP completion;
  `prefix` filters and `max` caps (default 50) BEFORE per-entry detail
  work; `unsafe: true` flags entries that would land a §6-banned form
  in domain code (v0.1: `unwrap`/`expect` on `Option`/`Result`
  receivers outside test files; name+receiver heuristic, honestly
  labelled in the brief). @impl/done
- ##OP-TYPE **`type`** `{file, position, content?}` → `{display,
  documentation}` — LSP hover, markdown stripped to text. @impl/done
- ##OP-SHUTDOWN **`shutdown`** `{}` → `{}` then exit 0 (the LSP shutdown/exit dance
  toward the child, kill-on-drop as backstop). @impl/done

## 3. The enrichment fields (in-process, same engine as the gate) {#enrichment}

##kind-line-enrichment `req r3` @impl/done

##RELAY-READS-CONFORM-TOML-AND-BUILDS-THE-GATES-RULES `rust-ai-native-tcg serve` reads the project's `conform.toml` once per init
(`rust_ai_native_conform::load_config_or_default`, origin printed to
stderr) and assembles THE GATE'S OWN rule set through the
`rust_ai_native_conform::build_rules` pub seam. @impl/done

##VALIDATE-PIPELINE-AND-ADVICE On `validate`: the effective
text → `RustFrontend::extract(file, crate, module, text)` →
`conform_core::check` → `conform_findings`, each flagged `baselined`
against the project's frozen ratchet baseline (the same file
`run_check` reads), plus `advice` strings in Class-F form citing GUIDE
REQs (`.unwrap()` in domain → §6 + the `#[spec(deviates)]` recipe;
missing doctest on a new pub seam → §3 Class G; ambient `std::env`
reads outside `env_roots` → the R-001 rule; a file over
`max_file_lines` → §2). @impl/done

##CRATE-MODULE-MAPPING-AND-FINDING-PARITY-TEST The crate/module strings for the single file
are computed by a relay-local mapping mirroring the engine's, and a
finding-parity test diffs the relay's finding set against
`rust-ai-native-conform check` on the same demo file — drift is a red test, not
a silent lie. @impl/done

##ENRICHMENT-FIELDS-EXIST-ONLY-ON-THIS-HOP A consumer that talks to the oracle-side directly gets
LSP; the enrichment fields exist only on this hop, and their absence
in a §2 response means "no policy layer", not an error. @impl/done

## 4. Error taxonomy {#errors}

##kind-line-errors `req r4` @impl/done

##five-error-kinds-lead Five kinds, each actionable, mirrored as typed variants in
`rust-ai-native-tcg-bridge`; the two environment rows are the DELIBERATE
renames against the TS table (§1): @impl/done

| kind | meaning | recipe carried |
|---|---|---|
| ##ROW-ERROR-RUST-ANALYZER-MISSING `rust-analyzer-missing` @impl/done | no analyzer resolvable (ORACLE-RUST §1) @impl/done | `rustup component add rust-analyzer` @impl/done |
| ##ROW-ERROR-WORKSPACE-UNLOADABLE `workspace-unloadable` @impl/done | the project failed to load (no Cargo.toml, cargo metadata failed) @impl/done | check `cargo metadata` standalone @impl/done |
| ##ROW-ERROR-ORACLE-CRASHED `oracle-crashed` @impl/done | child died / stream closed mid-session @impl/done | respawn guidance; the host registry retries once @impl/done |
| ##ROW-ERROR-PROTOCOL `protocol` @impl/done | unparseable frame, proto mismatch, unknown op @impl/done | version note; unknown-op errors list the known ops @impl/done |
| ##ROW-ERROR-TIMEOUT `timeout` @impl/done | no response within the caller's budget @impl/done | the op and the budget, for tuning @impl/done |

##PRODUCT-LINK-LAYER-SPECIAL-CASES-ONLY-ORACLE-CRASHED The product's link layer special-cases only `oracle-crashed`
(session-grain) and passes every other kind through as a
recipe-carrying detail — verified against the shipped `vibe-tcg`
before this protocol was authored, so the renames ride WITHOUT a
product edit. @impl/done

## 5. Compatibility rules {#compat}

##kind-line-compat `req r5` @impl/done

##compatibility-is-identical-to-the-ts-protocol-lead Identical to the TS protocol's §5, restated: @impl/done

- ##COMPAT-ADDITIVE-EVOLUTION additive evolution within
  a `proto` (optional params, new response fields, new advice/finding
  entries — consumers ignore the unknown); @impl/done
- ##COMPAT-BREAKING-CHANGES-BUMP-THE-CONSTANT renames, type changes, and
  semantic changes bump `ORACLE_PROTOCOL` and the bridge treats the
  mismatch as its own error class. @impl/done

##REPLAY-GOLDENS-AND-RECORDED-TRANSCRIPTS-PIN-BOTH-HOPS Replay goldens pin the CURRENT outer
shape in this package's tests; recorded LSP transcripts pin the inner
hop the same way (both r-a-free in the unit suite). @impl/done

##MARKERS-RESERVATION-MAY-BE-FILLED-IN-A-FUTURE-MINOR The `markers: []`
reservation (§2) may be filled in a future minor by the specmark tag
stream — that is the additive path, planned for, not promised. @spec/done
