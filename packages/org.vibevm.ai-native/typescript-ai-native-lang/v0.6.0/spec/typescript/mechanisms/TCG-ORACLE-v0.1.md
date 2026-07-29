# TCG-ORACLE v0.1 — the TypeScript oracle process model {#root}

<status stage="spec" state="done"/>

##status-line **Status: v0.1 — authored with AGENTIC-TCG-TS-PLAN v0.1 (Phase 1),
implemented by its Phases 2–3.** @impl/done

##companion-documents The component brief is
[`tools/vibe-agentic-tcg-ts.md`](../tools/vibe-agentic-tcg-ts.md); the
message grammar both hops speak is
[`TCG-PROTOCOL-v0.1`](TCG-PROTOCOL-v0.1.md). @impl/done

##DOCUMENT-OWNS-THE-ORACLE-PROCESS This document owns the
oracle PROCESS: lifecycle, host semantics, overlays, degradation, and
the latency posture. @impl/done

## 1. The process and its delivery {#delivery}

##kind-line-delivery `req r1` @impl/done

##ORACLE-IS-ONE-SELF-CONTAINED-SOURCE The oracle is ONE self-contained erasable-syntax-only TypeScript source,
`tools/ts-oracle/oracle.ts`, run directly by the consumer's node
(>= 22.6, strip-types) — no build step, no runtime npm dependency of its
own. @impl/done

##DELIVERY-EMBEDDED-AND-CONTENT-ADDRESSED It is delivered EMBEDDED in the Rust bridge crate
(`include_str!`) and materialised content-addressed to
`<project>/target/tcg/ts-oracle/oracle-<hash16>.ts` before spawn — the
proven ts-extract delivery, so a consumer needs nothing beyond what the
tsc floor step already requires. @impl/done

##SOURCE-STAYS-IMPORT-FREE-OF-SIBLING-TOOL-FILES Because exactly one file is
materialised, the source MUST stay import-free of sibling tool files;
the ~120 lines of per-file fact/marker logic shared with
`ts-extract/extract.ts` are consciously duplicated, pointered both ways,
and held behaviourally aligned by a fact-parity package test (same
fixture in → same facts out, modulo record framing). @impl/done

## 2. The consumer's compiler, exactly as tsc sees it {#compiler}

##kind-line-compiler `req r2` @impl/done

##TYPESCRIPT-RESOLVED-FROM-THE-CONSUMER-ROOT `typescript` is resolved from the CONSUMER's project root
(`createRequire(<root>/package.json).resolve("typescript")`, dynamic
import) — never bundled. @impl/done

##RESOLUTION-FAILURE-IS-A-RECIPE-CARRYING-ERROR Resolution failure is a hard, recipe-carrying protocol error
(`typescript-unresolvable`; the recipe names
`npm install -D typescript`), never a silent skip. @impl/done

##CONFIG-READ-THROUGH-THE-SAME-PATH-TSC-USES The project
configuration is read through `ts.getParsedCommandLineOfConfigFile` —
the SAME path tsc uses — so option assembly cannot drift from the floor
step; the config file is `<root>/tsconfig.json` unless `init` names
another. @impl/done

##CONFIG-DIAGNOSTICS-DEGRADE Config diagnostics degrade per §5, they do not crash. @impl/done

## 3. The language-service host and overlays {#host}

##kind-line-host `req r3` @impl/done

##host-and-overlay-map-lead The oracle holds one `LanguageService` per `init` root, over a host
whose script set is (parsed config file names ∪ overlay names) and whose
snapshots come from an in-memory overlay map
`path → { content, version }` with disk fallthrough: @impl/done

- ##OVERLAY-UPDATE-SETS-AND-CLEARS `update {file, content}` sets/replaces an overlay and bumps its
  version; `update {file, content: null}` clears it (disk state shows
  through again). Overlay paths are normalised to forward slashes;
  matching is case-preserving with case-insensitive comparison on
  Windows. @impl/done
- ##INLINE-CONTENT-IS-A-ONE-SHOT-OVERLAY Every query op (`validate`, `scope`, `complete`, `type`) accepts an
  optional inline `content`, which acts as a one-shot overlay for the
  duration of that query (set, query, restore) so single-question
  callers need no update/clear dance. @impl/done
- ##OVERLAID-FILE-NEED-NOT-EXIST-ON-DISK An overlaid file need not exist on disk — a hypothetical new module
  participates in the program like any other root file. @impl/done
- ##SERVICE-IS-INCREMENTAL-BY-CONSTRUCTION The service is INCREMENTAL by construction: versions only move when
  content moves, so the checker re-uses everything unchanged. The
  Phase-0 spike facts on a demo-sized tree: ~0.4 s first program build,
  ~22 ms warm re-validate, ~31 ms completions, ~21 ms quick info. @impl/done

## 4. Query semantics {#queries}

##kind-line-queries `req r4` @impl/done

- ##QUERY-VALIDATE `validate` returns the target file's syntactic + semantic diagnostics
  (code, category, message, line, character) — file-grain, never
  whole-program sweeps — PLUS the per-file conform facts and §9 spec
  markers extracted from the same content, so the Rust layer can run
  discipline rules without a second parse. @impl/done
- ##QUERY-SCOPE `scope` returns the in-scope symbols at a position (or the file's
  top level): name, kind, and type text; plus the file's cell and seam
  context and the branded types exported at reachable seams. Brand
  detection in v0.1 is a SYNTACTIC heuristic (exported type aliases
  whose declaration matches the intersection-brand shape) and every
  such answer carries `heuristic: true` — the honest label is part of
  the contract. @impl/done
- ##QUERY-COMPLETE `complete` returns the language service's completions at a position,
  each entry carrying name, kind, and type text, with an `unsafe` flag
  on entries whose insertion would introduce a §8-banned form. @impl/done
- ##QUERY-TYPE `type` returns quick info (display string + documentation) at a
  position. @impl/done

## 5. Degradation, never crashes (B5 extended) {#degradation}

##kind-line-degradation `req r5` @impl/done

##B5-RULE-EXTENDS-TO-THE-ORACLE The extractor's B5 rule extends to the oracle: no input may kill the
process or poison the session. @impl/done

- ##DEGRADE-UNPARSEABLE-OVERLAY-CONTENT Unparseable overlay content → the op answers with the syntactic
  diagnostics it could get and `degraded: true` where facts are absent;
  the service survives. @impl/done
- ##DEGRADE-UNKNOWN-OP An op the oracle does not know → a protocol error naming the known op
  set (forward compatibility for older embedded oracles under newer
  bridges). @impl/done
- ##DEGRADE-INTERNAL-EXCEPTION An internal exception inside one op → an `{ok: false}` response for
  that op with the message, and the loop continues; the bridge decides
  whether to respawn. @impl/done
- ##SHUTDOWN-IS-THE-ONLY-SANCTIONED-EXIT `shutdown` is the only sanctioned exit; EOF on stdin is treated as
  shutdown (the parent died — exit 0, leave nothing behind). @impl/done

## 6. Process lifecycle and Windows discipline {#lifecycle}

##kind-line-lifecycle `req r6` @impl/done

##ORACLE-IS-A-LONG-LIVED-CHILD The oracle is a LONG-LIVED child: spawned once per (root, session) by
the bridge, answering until `shutdown`/EOF. @impl/done

##STDOUT-CARRIES-PROTOCOL-FRAMES-ONLY stdout carries protocol
frames ONLY; all human-facing logging goes to stderr (one line per op:
op, duration ms) so a `serve` session is debuggable without corrupting
the stream. @impl/done

##RUST-SIDE-OWNS-TERMINATION The Rust side owns termination: kill-on-drop plus an
explicit `shutdown` on graceful paths, and the no-zombie property is
asserted by test (the Phase-0 spike proved spawn/roundtrip/kill with no
surviving pid on this box). @impl/done

##NODE-IS-RESOLVED-FROM-PATH-BY-THE-BRIDGE Node is resolved from PATH by the spawning
bridge exactly as the extract bridge does; a missing node is the
bridge's `node-missing` error with its recipe, not an oracle concern. @impl/done

## 7. Latency posture {#latency}

##kind-line-latency `req r7` @impl/done

##TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED Targets are POSTED and MEASURED, never CI-gated (timing gates on shared
boxes generate flakes, not signal): @impl/done

- ##TARGET-WARM-VALIDATE-AND-COMPLETE warm `validate` p50 < 150 ms and
  `complete` p50 < 200 ms on demo-class trees (only the `validate` half
  is measured today: the bench harness never calls `complete`, and its
  report carries no complete-latency field), @spec/done
- ##TARGET-COLD-INIT cold init < 5 s. @impl/done

##BENCH-HARNESS-RECORDS-DISTRIBUTIONS The battery's bench harness records the distributions per run; a target
that moves, moves in a committed REPORT with a reason. @impl/done

##CORRECTNESS-IS-CI-GATED Correctness
(the differential validate-vs-tsc corpus, completions goldens) IS
CI-gated — the split is deliberate: gate what cannot flake, record what
can. @impl/done
