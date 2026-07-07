# TCG-ORACLE v0.1 — the TypeScript oracle process model {#root}

**Status: v0.1 — authored with AGENTIC-TCG-TS-PLAN v0.1 (Phase 1),
implemented by its Phases 2–3.** The component brief is
[`tools/vibe-agentic-tcg-ts.md`](../tools/vibe-agentic-tcg-ts.md); the
message grammar both hops speak is
[`TCG-PROTOCOL-v0.1`](TCG-PROTOCOL-v0.1.md). This document owns the
oracle PROCESS: lifecycle, host semantics, overlays, degradation, and
the latency posture.

## 1. The process and its delivery {#delivery}

`req r1`

The oracle is ONE self-contained erasable-syntax-only TypeScript source,
`tools/ts-oracle/oracle.ts`, run directly by the consumer's node
(>= 22.6, strip-types) — no build step, no runtime npm dependency of its
own. It is delivered EMBEDDED in the Rust bridge crate
(`include_str!`) and materialised content-addressed to
`<project>/target/tcg/ts-oracle/oracle-<hash16>.ts` before spawn — the
proven ts-extract delivery, so a consumer needs nothing beyond what the
tsc floor step already requires. Because exactly one file is
materialised, the source MUST stay import-free of sibling tool files;
the ~120 lines of per-file fact/marker logic shared with
`ts-extract/extract.ts` are consciously duplicated, pointered both ways,
and held behaviourally aligned by a fact-parity package test (same
fixture in → same facts out, modulo record framing).

## 2. The consumer's compiler, exactly as tsc sees it {#compiler}

`req r2`

`typescript` is resolved from the CONSUMER's project root
(`createRequire(<root>/package.json).resolve("typescript")`, dynamic
import) — never bundled. Resolution failure is a hard, recipe-carrying
protocol error (`typescript-unresolvable`; the recipe names
`npm install -D typescript`), never a silent skip. The project
configuration is read through `ts.getParsedCommandLineOfConfigFile` —
the SAME path tsc uses — so option assembly cannot drift from the floor
step; the config file is `<root>/tsconfig.json` unless `init` names
another. Config diagnostics degrade per §5, they do not crash.

## 3. The language-service host and overlays {#host}

`req r3`

The oracle holds one `LanguageService` per `init` root, over a host
whose script set is (parsed config file names ∪ overlay names) and whose
snapshots come from an in-memory overlay map
`path → { content, version }` with disk fallthrough:

- `update {file, content}` sets/replaces an overlay and bumps its
  version; `update {file, content: null}` clears it (disk state shows
  through again). Overlay paths are normalised to forward slashes;
  matching is case-preserving with case-insensitive comparison on
  Windows.
- Every query op (`validate`, `scope`, `complete`, `type`) accepts an
  optional inline `content`, which acts as a one-shot overlay for the
  duration of that query (set, query, restore) so single-question
  callers need no update/clear dance.
- An overlaid file need not exist on disk — a hypothetical new module
  participates in the program like any other root file.
- The service is INCREMENTAL by construction: versions only move when
  content moves, so the checker re-uses everything unchanged. The
  Phase-0 spike facts on a demo-sized tree: ~0.4 s first program build,
  ~22 ms warm re-validate, ~31 ms completions, ~21 ms quick info.

## 4. Query semantics {#queries}

`req r4`

- `validate` returns the target file's syntactic + semantic diagnostics
  (code, category, message, line, character) — file-grain, never
  whole-program sweeps — PLUS the per-file conform facts and §9 spec
  markers extracted from the same content, so the Rust layer can run
  discipline rules without a second parse.
- `scope` returns the in-scope symbols at a position (or the file's
  top level): name, kind, and type text; plus the file's cell and seam
  context and the branded types exported at reachable seams. Brand
  detection in v0.1 is a SYNTACTIC heuristic (exported type aliases
  whose declaration matches the intersection-brand shape) and every
  such answer carries `heuristic: true` — the honest label is part of
  the contract.
- `complete` returns the language service's completions at a position,
  each entry carrying name, kind, and type text, with an `unsafe` flag
  on entries whose insertion would introduce a §8-banned form.
- `type` returns quick info (display string + documentation) at a
  position.

## 5. Degradation, never crashes (B5 extended) {#degradation}

`req r5`

The extractor's B5 rule extends to the oracle: no input may kill the
process or poison the session.

- Unparseable overlay content → the op answers with the syntactic
  diagnostics it could get and `degraded: true` where facts are absent;
  the service survives.
- An op the oracle does not know → a protocol error naming the known op
  set (forward compatibility for older embedded oracles under newer
  bridges).
- An internal exception inside one op → an `{ok: false}` response for
  that op with the message, and the loop continues; the bridge decides
  whether to respawn.
- `shutdown` is the only sanctioned exit; EOF on stdin is treated as
  shutdown (the parent died — exit 0, leave nothing behind).

## 6. Process lifecycle and Windows discipline {#lifecycle}

`req r6`

The oracle is a LONG-LIVED child: spawned once per (root, session) by
the bridge, answering until `shutdown`/EOF. stdout carries protocol
frames ONLY; all human-facing logging goes to stderr (one line per op:
op, duration ms) so a `serve` session is debuggable without corrupting
the stream. The Rust side owns termination: kill-on-drop plus an
explicit `shutdown` on graceful paths, and the no-zombie property is
asserted by test (the Phase-0 spike proved spawn/roundtrip/kill with no
surviving pid on this box). Node is resolved from PATH by the spawning
bridge exactly as the extract bridge does; a missing node is the
bridge's `node-missing` error with its recipe, not an oracle concern.

## 7. Latency posture {#latency}

`req r7`

Targets are POSTED and MEASURED, never CI-gated (timing gates on shared
boxes generate flakes, not signal): warm `validate` p50 < 150 ms and
`complete` p50 < 200 ms on demo-class trees, cold init < 5 s. The
battery's bench harness records the distributions per run; a target
that moves, moves in a committed REPORT with a reason. Correctness
(the differential validate-vs-tsc corpus, completions goldens) IS
CI-gated — the split is deliberate: gate what cannot flake, record what
can.
