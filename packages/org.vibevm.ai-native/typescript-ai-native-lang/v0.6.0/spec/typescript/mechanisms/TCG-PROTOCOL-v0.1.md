# TCG-PROTOCOL v0.1 — the oracle wire protocol, both hops {#root}

<status stage="spec" state="done"/>

@fact:status-line **Status: v0.1 — authored with AGENTIC-TCG-TS-PLAN v0.1 (Phase 1),
implemented by its Phases 2–4.** @status:impl/done

@fact:companion-document The process model is
[`TCG-ORACLE-v0.1`](TCG-ORACLE-v0.1.md). @status:impl/done

@fact:DOCUMENT-OWNS-THE-MESSAGE-GRAMMAR This document owns the MESSAGE
GRAMMAR: framing, envelopes, every operation's request/response shape,
the enrichment fields the Rust middle layer adds, and the error
taxonomy. @status:impl/done

@fact:ONE-SHAPE-SERVES-BOTH-HOPS One shape serves both hops — vibe-mcp ⇄ `typescript-ai-native-tcg
serve` and `typescript-ai-native-tcg` ⇄ `node oracle.ts` — the middle layer ADDS
fields, it never reshapes. @status:impl/done

## 1. Framing and envelopes {#framing}

@fact:kind-line-framing `req r1` @status:impl/done

@fact:NDJSON-DUPLEX-FRAMING NDJSON duplex: one JSON object per line, UTF-8, `\n`-terminated, both
directions. @status:impl/done

@fact:REQUESTS-AND-RESPONSES-CORRELATE-BY-ID Requests and responses correlate by caller-chosen `id`
(number, unique per in-flight request). @status:impl/done

@fact:version-rides-every-frame-lead The protocol version rides
every frame: @status:impl/done

```jsonc
// request
{"proto": 1, "id": 7, "op": "validate", "params": { /* per-op */ }}
// success
{"proto": 1, "id": 7, "ok": true, "result": { /* per-op */ }}
// failure (op-grain, session survives)
{"proto": 1, "id": 7, "ok": false,
 "error": {"kind": "<taxonomy §4>", "detail": "…", "recipe": "…?"}}
```

@fact:ORACLE-PROTOCOL-IS-INDEPENDENT-OF-EXTRACT-PROTOCOL `ORACLE_PROTOCOL = 1` is independent of ts-extract's `PROTOCOL = 1`
(different channel, different message set; the constants version
independently). @status:impl/done

@fact:PROTO-MISMATCH-AND-UNKNOWN-IDS A `proto` mismatch is a `protocol` error; responses to
unknown `id`s are a bridge bug and dropped with a stderr note. @status:impl/done

@fact:REQUESTS-MAY-BE-PIPELINED Requests
MAY be pipelined; responses come in completion order (the oracle is
single-threaded per op today, so in practice FIFO — callers must still
match by `id`, not order). @status:impl/done

## 2. Operations {#ops}

@fact:kind-line-ops `req r2` @status:impl/done

@fact:POSITIONS-ARE-ONE-BASED-LINE-ZERO-BASED-CHARACTER Positions are `{line, character}`, 1-based line, 0-based character (the
TypeScript convention surfaced honestly). @status:impl/done

@fact:PATHS-ARE-ROOT-RELATIVE-WITH-FORWARD-SLASHES Paths are project-root-
relative with forward slashes. @status:impl/done

- @fact:OP-INIT **`init`** `{root, cells_dir?, seam?}` →
  `{ts_version, config_file, root_files}` — builds the service (ORACLE
  §2–3). `cells_dir`/`seam` are policy-derived DATA the Rust layer
  passes down (the node side never reads `conform.toml` itself); they
  feed the `scope` op's cell/seam/branded context and default to
  none/`"index"`. Re-`init` on a live oracle rebuilds config and
  policy; overlays are cleared. @status:impl/done
- @fact:OP-UPDATE **`update`** `{file, content | null}` → `{version}` — set/clear an
  overlay (ORACLE §3). @status:impl/done
- @fact:OP-VALIDATE **`validate`** `{file, content?}` →
  `{diagnostics: [{code, category, message, line, character}],
    facts: [/* ts-extract fact shapes */],
    markers: [/* §9 marker shapes */], degraded}` — the fact/marker
  arrays reuse the ts-extract record vocabulary verbatim (`ts_unsafe`,
  `import`, `item`, `file_metrics`; `{tag, uri, reason, symbol, line}`)
  so one serde vocabulary serves both tools. @status:impl/done
- @fact:OP-SCOPE **`scope`** `{file, position?}` →
  `{symbols: [{name, kind, type_text}], cell, seam_file,
    branded: [{name, seam, heuristic}]}`. @status:impl/done
- @fact:OP-COMPLETE **`complete`** `{file, position, content?, prefix?, max?}` →
  `{entries: [{name, kind, type_text, unsafe}]}` — `prefix` filters by
  name prefix and `max` caps the set (default 50) BEFORE the per-entry
  checker details are computed: type text and the `unsafe` flag are
  entry-grain checker work, affordable only after the cut. A caller
  that wants the raw thousand-entry universe passes no prefix and a
  large `max`, and pays for it knowingly. @status:impl/done
- @fact:OP-TYPE **`type`** `{file, position, content?}` →
  `{display, documentation}`. @status:impl/done
- @fact:OP-SHUTDOWN **`shutdown`** `{}` → `{}` then exit 0. @status:impl/done

## 3. The enrichment hop (Rust adds, never reshapes) {#enrichment}

@fact:kind-line-enrichment `req r3` @status:impl/done

@fact:serve-widens-two-responses-lead `typescript-ai-native-tcg serve` speaks §1–§2 upward unchanged and widens two
responses with policy-derived fields (policy = the project's
`conform.toml`, read at init; ORACLE §4 keeps the node side
policy-free): @status:impl/done

- @fact:ENRICHMENT-VALIDATE-CONFORM-FINDINGS-AND-ADVICE `validate.result` gains
  `conform_findings: [{rule, message, line, baselined}]` — the REAL
  rule set (`ts-unsafe-in-domain`, `ts-cell-isolation`, file budget)
  run over the returned facts via `conform_core::check`, each finding
  flagged against the project's frozen ratchet baseline — and
  `advice: [string]` (Class-F strings citing `spec://` REQs). @status:impl/done
- @fact:ENRICHMENT-SCOPE-BRANDED-COMPLETION `scope.result.branded` is completed from seam files per the policy's
  `cells_dir`/`seam`, and `advice` may name the branded constructor a
  bare primitive at this seam should use. @status:impl/done
- @fact:ENRICHMENT-COMPLETE-UNSAFE-FINALISED `complete.result.entries[].unsafe` is finalised against the policy
  (the node side flags candidates; policy decides). @status:impl/done

@fact:ENRICHMENT-FIELDS-ARE-ADDITIVE A consumer that talks to the oracle directly (no Rust layer) gets
well-formed §2 responses with no enrichment fields — the fields are
additive, and their absence means "no policy layer", not an error. @status:impl/done

## 4. Error taxonomy {#errors}

@fact:kind-line-errors `req r4` @status:impl/done

@fact:five-error-kinds-lead Five kinds, each actionable, each carried in the §1 error object (and
mirrored as typed variants in `typescript-ai-native-tcg-bridge`): @status:impl/done

| kind | meaning | recipe carried |
|---|---|---|
| @fact:ROW-ERROR-NODE-MISSING `node-missing` @status:impl/done | node not spawnable @status:impl/done | install node >= 22.6 @status:impl/done |
| @fact:ROW-ERROR-TYPESCRIPT-UNRESOLVABLE `typescript-unresolvable` @status:impl/done | consumer install absent (ORACLE §2) @status:impl/done | `npm install -D typescript` @status:impl/done |
| @fact:ROW-ERROR-ORACLE-CRASHED `oracle-crashed` @status:impl/done | child died / stream closed mid-session @status:impl/done | respawn guidance; the bridge may retry once @status:impl/done |
| @fact:ROW-ERROR-PROTOCOL `protocol` @status:impl/done | unparseable frame, `proto` mismatch, unknown op @status:impl/done | version/upgrade note; unknown-op errors list the known ops @status:impl/done |
| @fact:ROW-ERROR-TIMEOUT `timeout` @status:impl/done | no response within the caller's budget @status:impl/done | the op and budget, for tuning @status:impl/done |

@fact:ERRORS-ARE-OP-GRAIN Errors are OP-GRAIN wherever possible (the session survives, ORACLE
§5); only `oracle-crashed` is session-grain. @status:impl/done

## 5. Compatibility rules {#compat}

@fact:kind-line-compat `req r5` @status:impl/done

@fact:COMPAT-ADDITIVE-EVOLUTION Additive evolution within a `proto`: new OPTIONAL request params, new
response fields, and new advice/finding entries are non-breaking; a
consumer ignores what it does not know. @status:impl/done

@fact:COMPAT-BREAKING-CHANGES-BUMP-THE-CONSTANT Renames, type changes, and
semantic changes to existing fields bump `ORACLE_PROTOCOL`, and the
bridge treats a mismatch as its own error class — the same
cache-retirement posture the extract bridge established. @status:impl/done

@fact:REPLAY-GOLDENS-PIN-BOTH-SIDES Scripted
doubles pin the INNER side in this package's tests (the no-node double,
`crates/typescript-ai-native-tcg-bridge/src/transport.rs:312`); the
fact-parity test (ORACLE §1) pins the vocabulary shared with
ts-extract. The OUTER shape is not yet pinned — no recorded stream is
checked into the package and no test constructs an outer frame. @status:spec/done
