# TCG-PROTOCOL v0.1 — the oracle wire protocol, both hops {#root}

**Status: v0.1 — authored with AGENTIC-TCG-TS-PLAN v0.1 (Phase 1),
implemented by its Phases 2–4.** The process model is
[`TCG-ORACLE-v0.1`](TCG-ORACLE-v0.1.md). This document owns the MESSAGE
GRAMMAR: framing, envelopes, every operation's request/response shape,
the enrichment fields the Rust middle layer adds, and the error
taxonomy. One shape serves both hops — vibe-mcp ⇄ `tcg-typescript
serve` and `tcg-typescript` ⇄ `node oracle.ts` — the middle layer ADDS
fields, it never reshapes.

## 1. Framing and envelopes {#framing}

`req r1`

NDJSON duplex: one JSON object per line, UTF-8, `\n`-terminated, both
directions. Requests and responses correlate by caller-chosen `id`
(number, unique per in-flight request). The protocol version rides
every frame:

```jsonc
// request
{"proto": 1, "id": 7, "op": "validate", "params": { /* per-op */ }}
// success
{"proto": 1, "id": 7, "ok": true, "result": { /* per-op */ }}
// failure (op-grain, session survives)
{"proto": 1, "id": 7, "ok": false,
 "error": {"kind": "<taxonomy §4>", "detail": "…", "recipe": "…?"}}
```

`ORACLE_PROTOCOL = 1` is independent of ts-extract's `PROTOCOL = 1`
(different channel, different message set; the constants version
independently). A `proto` mismatch is a `protocol` error; responses to
unknown `id`s are a bridge bug and dropped with a stderr note. Requests
MAY be pipelined; responses come in completion order (the oracle is
single-threaded per op today, so in practice FIFO — callers must still
match by `id`, not order).

## 2. Operations {#ops}

`req r2`

Positions are `{line, character}`, 1-based line, 0-based character (the
TypeScript convention surfaced honestly). Paths are project-root-
relative with forward slashes.

- **`init`** `{root, cells_dir?, seam?}` →
  `{ts_version, config_file, root_files}` — builds the service (ORACLE
  §2–3). `cells_dir`/`seam` are policy-derived DATA the Rust layer
  passes down (the node side never reads `conform.toml` itself); they
  feed the `scope` op's cell/seam/branded context and default to
  none/`"index"`. Re-`init` on a live oracle rebuilds config and
  policy; overlays are cleared.
- **`update`** `{file, content | null}` → `{version}` — set/clear an
  overlay (ORACLE §3).
- **`validate`** `{file, content?}` →
  `{diagnostics: [{code, category, message, line, character}],
    facts: [/* ts-extract fact shapes */],
    markers: [/* §9 marker shapes */], degraded}` — the fact/marker
  arrays reuse the ts-extract record vocabulary verbatim (`ts_unsafe`,
  `import`, `item`, `file_metrics`; `{tag, uri, reason, symbol, line}`)
  so one serde vocabulary serves both tools.
- **`scope`** `{file, position?}` →
  `{symbols: [{name, kind, type_text}], cell, seam_file,
    branded: [{name, seam, heuristic}]}`.
- **`complete`** `{file, position, content?, prefix?, max?}` →
  `{entries: [{name, kind, type_text, unsafe}]}` — `prefix` filters by
  name prefix and `max` caps the set (default 50) BEFORE the per-entry
  checker details are computed: type text and the `unsafe` flag are
  entry-grain checker work, affordable only after the cut. A caller
  that wants the raw thousand-entry universe passes no prefix and a
  large `max`, and pays for it knowingly.
- **`type`** `{file, position, content?}` →
  `{display, documentation}`.
- **`shutdown`** `{}` → `{}` then exit 0.

## 3. The enrichment hop (Rust adds, never reshapes) {#enrichment}

`req r3`

`tcg-typescript serve` speaks §1–§2 upward unchanged and widens two
responses with policy-derived fields (policy = the project's
`conform.toml`, read at init; ORACLE §4 keeps the node side
policy-free):

- `validate.result` gains
  `conform_findings: [{rule, message, line, baselined}]` — the REAL
  rule set (`ts-unsafe-in-domain`, `ts-cell-isolation`, file budget)
  run over the returned facts via `conform_core::check`, each finding
  flagged against the project's frozen ratchet baseline — and
  `advice: [string]` (Class-F strings citing `spec://` REQs).
- `scope.result.branded` is completed from seam files per the policy's
  `cells_dir`/`seam`, and `advice` may name the branded constructor a
  bare primitive at this seam should use.
- `complete.result.entries[].unsafe` is finalised against the policy
  (the node side flags candidates; policy decides).

A consumer that talks to the oracle directly (no Rust layer) gets
well-formed §2 responses with no enrichment fields — the fields are
additive, and their absence means "no policy layer", not an error.

## 4. Error taxonomy {#errors}

`req r4`

Five kinds, each actionable, each carried in the §1 error object (and
mirrored as typed variants in `tcg-oracle-bridge`):

| kind | meaning | recipe carried |
|---|---|---|
| `node-missing` | node not spawnable | install node >= 22.6 |
| `typescript-unresolvable` | consumer install absent (ORACLE §2) | `npm install -D typescript` |
| `oracle-crashed` | child died / stream closed mid-session | respawn guidance; the bridge may retry once |
| `protocol` | unparseable frame, `proto` mismatch, unknown op | version/upgrade note; unknown-op errors list the known ops |
| `timeout` | no response within the caller's budget | the op and budget, for tuning |

Errors are OP-GRAIN wherever possible (the session survives, ORACLE
§5); only `oracle-crashed` is session-grain.

## 5. Compatibility rules {#compat}

`req r5`

Additive evolution within a `proto`: new OPTIONAL request params, new
response fields, and new advice/finding entries are non-breaking; a
consumer ignores what it does not know. Renames, type changes, and
semantic changes to existing fields bump `ORACLE_PROTOCOL`, and the
bridge treats a mismatch as its own error class — the same
cache-retirement posture the extract bridge established. Replay
goldens on both sides (recorded streams checked into the package
tests) pin the CURRENT shape; the fact-parity test (ORACLE §1) pins
the vocabulary shared with ts-extract.
