# LEDGER — the intent ledger, v0.1

**Status.** Design, beta. The riskiest mechanism in the package, so it gets its own document. Implements Charter A2 ("never pay twice for the same understanding") under the constraint that broke the naive version: **meaning rots contextually even when content hashes match.** A dependency upgrades, a neighboring REQ is reinterpreted, the world moves — and a hash-valid cached explanation becomes confidently wrong. Confidently-served stale knowledge violates A1 worse than an honest recompute. The ledger is designed around that objection, not despite it.

---

## 1. What the ledger is {#what}

A persistent, content-addressed store of **memoized queries about the project**: `get_or_compute(query) -> entry`. Cache hit ≈ $0; miss runs the producer (algorithm below the floor, LLM above it) and materializes the result. Execution model borrowed from Salsa / the rustc query system (MIT/Apache-2.0 — ideas and, where useful, code); persistence and cross-process reach are ours, because our queries span tools, sessions, and machines, which in-memory incremental frameworks do not.

What it is **not**: not ground truth (authored truth lives in code tags and spec units — PROP-014); not committed to git (regenerable derived data); not a vector database (embeddings are a possible later producer, not the store's identity).

## 2. Two storage classes — the load-bearing taxonomy {#classes}

| Class | Examples | Key | Rots? |
|---|---|---|---|
| **Facts** | parsed items, import edges, spans, hashes, lint findings | `(file content-hash, producer id + version)` | **No** — purely syntactic; invalid only when the file or the producer changes. By construction never stale. |
| **Interpretations** | item summaries, explanation renders, legacy-unit classifications, link proposals, overlap judgments | `(subject hashes, spec revs touched, **epoch**, producer id, prompt rev, model id)` | **Yes** — hence the epoch in the key. |

The conform engine's fact store (ENGINE §3) is the facts class instantiated. Everything an LLM produces is interpretations class, no exceptions.

## 3. Epochs — contextual invalidation {#epochs}

```
epoch = H( dependency lockfiles (Cargo.lock, vibe.lock)
         , toolchain version
         , discipline-package versions in effect
         , metamodel schema version )
```

Epoch changes when the *context of meaning* changes, even though no subject file did. Interpretations keyed under an old epoch are **not served** — hard invalidation, no serve-while-stale for A1-critical surfaces. The recompute decision then happens *above the floor*: the producer may read the old entry as a draft input ("here is what was previously believed; re-verify against the new epoch"), which converts most invalidations into cheap re-affirmations rather than from-scratch work — A2 preserved in weakened, honest form: *never pay full price twice.*

## 4. Provenance — every entry confesses its origin {#provenance}

Each entry carries `{producer, model_id?, prompt_rev?, inputs (hashes + spec URIs ~r), epoch, cost, created_at, confidence}`. Every rendered explanation **displays** its provenance line ("computed at PROP-003#conditional-deps~r2, epoch 7f3a, model …"). Staleness thereby becomes detectable by the reader even across policy bugs — the last line of defense.

## 5. Storage and lifecycle {#storage}

- Layout: `.ledger/objects/<sha256[0..2]>/<sha256>` + a small index; sharded like git objects. Local per checkout; CI carries a shared warm copy (standard action-cache pattern, cf. Bazel/sccache — Apache-2.0 prior art, ideas only needed).
- GC: LRU with a pin set (entries referenced by the current release slice are pinned). Size budget configurable; eviction never affects correctness, only cost.
- Concurrency: entries are immutable values under content keys; last-write-wins on identical keys is benign.
- Telemetry (feeds the Charter's headline metric): hit rate, cost per query kind, **LLM-$ per merged change**, and the **contextual-rot rate** — fraction of epoch-invalidated entries whose re-verification *changed the answer*. Threshold from the design review: if rot among hash-valid entries exceeds ~10–15% per epoch window, the epoch formula is too coarse and gains inputs (e.g., per-subsystem epochs).

## 6. Query kinds shipped in v0.1 {#queries}

1. `facts.extract(file)` — frontends (algorithmic).
2. `explain.item(symbol)` — structured subgraph (algorithmic) + optional prose render (LLM, interpretations class).
3. `classify.legacy_unit(text)` — importer support (LLM).
4. `propose.links(crate, doc)` — Phase-2 mining (LLM; output lands in the proposals file, *never* directly in code — affirmation is a human diff, PROP-014 §2.7).

Everything else waits for demand: a query kind is added when two distinct consumers ask for it.

## 7. The release slice {#release-slice}

At tag time, a frozen subset — facts for the tagged tree + affirmed interpretations (item summaries, command explanations) — is exported, **signed**, and shipped with the package (the AI-native OSS artifact: agents debugging `vibe` at v0.3.2 query the v0.3.2 slice). Unsigned slices are not exposed remotely, full stop (PROP-014 §2.8.4). Signing scheme is Charter-level Open Question; until it lands, the slice exists for local use only.

## 8. Failure modes, named {#failures}

- **Confident staleness** → epochs + hard no-serve + provenance display (§3–4).
- **Cache poisoning** (a producer writes wrong values at scale) → producer id + prompt_rev in keys make wholesale invalidation of a bad producer one predicate; release slices are re-derivable from source.
- **Key under-specification** (two different questions colliding on one key) → query kinds are a closed enum with reviewed key schemas; adding a kind is a PR, not a string.
- **Ledger worship** (treating renders as truth) → renders cite spec URIs; the `--json` raw subgraph is always available; A4 keeps the human the accountability point.

---

*Any query kind, key field, or policy here not exercised by Playbook Phase 5 is removed from this document rather than carried as aspiration.*
