# TCG-PROTOCOL v0.1 — the oracle wire protocol, both hops {#root}

<status stage="spec" state="done"/>

##status-line **Status: v0.1 — authored with AGENTIC-TCG-TS-PLAN v0.1 (Phase 1),
implemented by its Phases 2–4.** @impl/done

##companion-document The process model is
[`TCG-ORACLE-v0.1`](TCG-ORACLE-v0.1.md). @impl/done

##DOCUMENT-OWNS-THE-MESSAGE-GRAMMAR This document owns the MESSAGE
GRAMMAR: framing, envelopes, every operation's request/response shape,
the enrichment fields the Rust middle layer adds, and the error
taxonomy. @impl/done

##ONE-SHAPE-SERVES-BOTH-HOPS One shape serves both hops — vibe-mcp ⇄ `typescript-ai-native-tcg
serve` and `typescript-ai-native-tcg` ⇄ `node oracle.ts` — the middle layer ADDS
fields, it never reshapes. @impl/done

## 1. Framing and envelopes {#framing}

##kind-line-framing `req r1` @impl/done

##NDJSON-DUPLEX-FRAMING NDJSON duplex: one JSON object per line, UTF-8, `\n`-terminated, both
directions. @impl/done

##REQUESTS-AND-RESPONSES-CORRELATE-BY-ID Requests and responses correlate by caller-chosen `id`
(number, unique per in-flight request). @impl/done

##version-rides-every-frame-lead The protocol version rides
every frame: @impl/done

```jsonc
// request
{"proto": 1, "id": 7, "op": "validate", "params": { /* per-op */ }}
// success
{"proto": 1, "id": 7, "ok": true, "result": { /* per-op */ }}
// failure (op-grain, session survives)
{"proto": 1, "id": 7, "ok": false,
 "error": {"kind": "<taxonomy §4>", "detail": "…", "recipe": "…?"}}
```

##ORACLE-PROTOCOL-IS-INDEPENDENT-OF-EXTRACT-PROTOCOL `ORACLE_PROTOCOL = 1` is independent of ts-extract's `PROTOCOL = 1`
(different channel, different message set; the constants version
independently). @impl/done

##PROTO-MISMATCH-AND-UNKNOWN-IDS A `proto` mismatch is a `protocol` error; responses to
unknown `id`s are a bridge bug and dropped with a stderr note. @impl/done

##REQUESTS-MAY-BE-PIPELINED Requests
MAY be pipelined; responses come in completion order (the oracle is
single-threaded per op today, so in practice FIFO — callers must still
match by `id`, not order). @impl/done

## 2. Operations {#ops}

##kind-line-ops `req r2` @impl/done

##POSITIONS-ARE-ONE-BASED-LINE-ZERO-BASED-CHARACTER Positions are `{line, character}`, 1-based line, 0-based character (the
TypeScript convention surfaced honestly). @impl/done

##PATHS-ARE-ROOT-RELATIVE-WITH-FORWARD-SLASHES Paths are project-root-
relative with forward slashes. @impl/done

- ##OP-INIT **`init`** `{root, cells_dir?, seam?}` →
  `{ts_version, config_file, root_files}` — builds the service (ORACLE
  §2–3). `cells_dir`/`seam` are policy-derived DATA the Rust layer
  passes down (the node side never reads `conform.toml` itself); they
  feed the `scope` op's cell/seam/branded context and default to
  none/`"index"`. Re-`init` on a live oracle rebuilds config and
  policy; overlays are cleared. @impl/done
- ##OP-UPDATE **`update`** `{file, content | null}` → `{version}` — set/clear an
  overlay (ORACLE §3). @impl/done
- ##OP-VALIDATE **`validate`** `{file, content?}` →
  `{diagnostics: [{code, category, message, line, character}],
    facts: [/* ts-extract fact shapes */],
    markers: [/* §9 marker shapes */], degraded}` — the fact/marker
  arrays reuse the ts-extract record vocabulary verbatim (`ts_unsafe`,
  `import`, `item`, `file_metrics`; `{tag, uri, reason, symbol, line}`)
  so one serde vocabulary serves both tools. @impl/done
- ##OP-SCOPE **`scope`** `{file, position?}` →
  `{symbols: [{name, kind, type_text}], cell, seam_file,
    branded: [{name, seam, heuristic}]}`. @impl/done
- ##OP-COMPLETE **`complete`** `{file, position, content?, prefix?, max?}` →
  `{entries: [{name, kind, type_text, unsafe}]}` — `prefix` filters by
  name prefix and `max` caps the set (default 50) BEFORE the per-entry
  checker details are computed: type text and the `unsafe` flag are
  entry-grain checker work, affordable only after the cut. A caller
  that wants the raw thousand-entry universe passes no prefix and a
  large `max`, and pays for it knowingly. @impl/done
- ##OP-TYPE **`type`** `{file, position, content?}` →
  `{display, documentation}`. @impl/done
- ##OP-SHUTDOWN **`shutdown`** `{}` → `{}` then exit 0. @impl/done

## 3. The enrichment hop (Rust adds, never reshapes) {#enrichment}

##kind-line-enrichment `req r3` @impl/done

##serve-widens-two-responses-lead `typescript-ai-native-tcg serve` speaks §1–§2 upward unchanged and widens two
responses with policy-derived fields (policy = the project's
`conform.toml`, read at init; ORACLE §4 keeps the node side
policy-free): @impl/done

- ##ENRICHMENT-VALIDATE-CONFORM-FINDINGS-AND-ADVICE `validate.result` gains
  `conform_findings: [{rule, message, line, baselined}]` — the REAL
  rule set (`ts-unsafe-in-domain`, `ts-cell-isolation`, file budget)
  run over the returned facts via `conform_core::check`, each finding
  flagged against the project's frozen ratchet baseline — and
  `advice: [string]` (Class-F strings citing `spec://` REQs). @impl/done
- ##ENRICHMENT-SCOPE-BRANDED-COMPLETION `scope.result.branded` is completed from seam files per the policy's
  `cells_dir`/`seam`, and `advice` may name the branded constructor a
  bare primitive at this seam should use. @impl/done
- ##ENRICHMENT-COMPLETE-UNSAFE-FINALISED `complete.result.entries[].unsafe` is finalised against the policy
  (the node side flags candidates; policy decides). @impl/done

##ENRICHMENT-FIELDS-ARE-ADDITIVE A consumer that talks to the oracle directly (no Rust layer) gets
well-formed §2 responses with no enrichment fields — the fields are
additive, and their absence means "no policy layer", not an error. @impl/done

## 4. Error taxonomy {#errors}

##kind-line-errors `req r4` @impl/done

##five-error-kinds-lead Five kinds, each actionable, each carried in the §1 error object (and
mirrored as typed variants in `typescript-ai-native-tcg-bridge`): @impl/done

| kind | meaning | recipe carried |
|---|---|---|
| ##ROW-ERROR-NODE-MISSING `node-missing` @impl/done | node not spawnable @impl/done | install node >= 22.6 @impl/done |
| ##ROW-ERROR-TYPESCRIPT-UNRESOLVABLE `typescript-unresolvable` @impl/done | consumer install absent (ORACLE §2) @impl/done | `npm install -D typescript` @impl/done |
| ##ROW-ERROR-ORACLE-CRASHED `oracle-crashed` @impl/done | child died / stream closed mid-session @impl/done | respawn guidance; the bridge may retry once @impl/done |
| ##ROW-ERROR-PROTOCOL `protocol` @impl/done | unparseable frame, `proto` mismatch, unknown op @impl/done | version/upgrade note; unknown-op errors list the known ops @impl/done |
| ##ROW-ERROR-TIMEOUT `timeout` @impl/done | no response within the caller's budget @impl/done | the op and budget, for tuning @impl/done |

##ERRORS-ARE-OP-GRAIN Errors are OP-GRAIN wherever possible (the session survives, ORACLE
§5); only `oracle-crashed` is session-grain. @impl/done

## 5. Compatibility rules {#compat}

##kind-line-compat `req r5` @impl/done

##COMPAT-ADDITIVE-EVOLUTION Additive evolution within a `proto`: new OPTIONAL request params, new
response fields, and new advice/finding entries are non-breaking; a
consumer ignores what it does not know. @impl/done

##COMPAT-BREAKING-CHANGES-BUMP-THE-CONSTANT Renames, type changes, and
semantic changes to existing fields bump `ORACLE_PROTOCOL`, and the
bridge treats a mismatch as its own error class — the same
cache-retirement posture the extract bridge established. @impl/done

##REPLAY-GOLDENS-PIN-BOTH-SIDES Replay
goldens on both sides (recorded streams checked into the package
tests) pin the CURRENT shape; the fact-parity test (ORACLE §1) pins
the vocabulary shared with ts-extract. @impl/done
