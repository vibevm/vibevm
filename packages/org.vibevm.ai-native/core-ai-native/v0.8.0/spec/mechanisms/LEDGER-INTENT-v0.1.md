# LEDGER — the intent ledger, v0.1 {#root}

<status stage="spec" state="done"/>

##status-line **Status.** Design, beta. @impl/done

##riskiest-mechanism-gets-its-own-document The riskiest mechanism in the package, so it gets its own document. @spec/done

##IMPLEMENTS-A2-UNDER-CONTEXTUAL-ROT Implements Charter A2 ("never pay twice for the same understanding") under the constraint that broke the naive version: **meaning rots contextually even when content hashes match.** @impl/done

##hash-valid-cache-becomes-confidently-wrong A dependency upgrades, a neighboring REQ is reinterpreted, the world moves — and a hash-valid cached explanation becomes confidently wrong. @spec/done

##STALE-KNOWLEDGE-VIOLATES-A1-WORSE-THAN-RECOMPUTE Confidently-served stale knowledge violates A1 worse than an honest recompute. @impl/done

##ledger-designed-around-that-objection The ledger is designed around that objection, not despite it. @spec/done

---

## 1. What the ledger is {#what}

##LEDGER-IS-A-STORE-OF-MEMOIZED-QUERIES A persistent, content-addressed store of **memoized queries about the project**: `get_or_compute(query) -> entry`. @impl/done

##HIT-IS-FREE-MISS-RUNS-THE-PRODUCER Cache hit ≈ $0; miss runs the producer (algorithm below the floor, LLM above it) and materializes the result. @impl/done

##EXECUTION-BORROWED-PERSISTENCE-IS-OURS Execution model borrowed from Salsa / the rustc query system (MIT/Apache-2.0 — ideas and, where useful, code); persistence and cross-process reach are ours, because our queries span tools, sessions, and machines, which in-memory incremental frameworks do not. @impl/done

##what-it-is-not-lead What it is **not**: @impl/done

- ##NOT-GROUND-TRUTH not ground truth (authored truth lives in code tags and spec units — PROP-014); @impl/done
- ##NOT-COMMITTED-TO-GIT not committed to git (regenerable derived data); @impl/done
- ##NOT-A-VECTOR-DATABASE not a vector database (embeddings are a possible later producer, not the store's identity). @impl/done

## 2. Two storage classes — the load-bearing taxonomy {#classes}

| Class | Examples | Key | Rots? |
|---|---|---|---|
| ##ROW-CLASS-FACTS **Facts** @impl/done | parsed items, import edges, spans, hashes, lint findings @impl/done | `(file content-hash, producer id + version)` @impl/done | **No** — purely syntactic; invalid only when the file or the producer changes. By construction never stale. @impl/done |
| ##ROW-CLASS-INTERPRETATIONS **Interpretations** @spec/done | designed: item summaries, explanation renders, legacy-unit classifications, link proposals, overlap judgments; shipped: explanation renders alone @spec/done | designed: `(subject hashes, spec revs touched, **epoch**, producer id, prompt rev, model id)`; shipped: `(subject, epoch, producer id)`, where the subject is the serialised explain subgraph and so carries pinned spec revisions inside it rather than as its own key component — `prompt rev` and `model id` occur nowhere in the crates @spec/done | **Yes** — hence the epoch in the key. @impl/done |

##CONFORM-FACT-STORE-IS-THE-FACTS-CLASS The conform engine's fact store (ENGINE §3) is the facts class instantiated. @impl/done

##LLM-OUTPUT-IS-ALWAYS-INTERPRETATIONS Everything an LLM produces is interpretations class, no exceptions. @impl/done

## 3. Epochs — contextual invalidation {#epochs}

```
epoch = H( dependency lockfiles (Cargo.lock, vibe.lock)
         , toolchain version
         , discipline-package versions in effect
         , metamodel schema version )
```

##EPOCH-CHANGES-WITH-THE-CONTEXT-OF-MEANING Epoch changes when the *context of meaning* changes, even though no subject file did. @impl/done

##OLD-EPOCH-INTERPRETATIONS-ARE-NOT-SERVED Interpretations keyed under an old epoch are **not served** — hard invalidation, no serve-while-stale for A1-critical surfaces. @impl/done

##RECOMPUTE-DECISION-HAPPENS-ABOVE-THE-FLOOR Designed: the recompute decision then happens *above the floor* — the producer may read the old entry as a draft input ("here is what was previously believed; re-verify against the new epoch"), which converts most invalidations into cheap re-affirmations rather than from-scratch work, A2 preserved in weakened, honest form: *never pay full price twice.* Not built: the one shipped producer renders from scratch on every miss and never reads a prior-epoch entry, so no re-affirmation path exists yet. @spec/done

## 4. Provenance — every entry confesses its origin {#provenance}

##ENTRY-CARRIES-ITS-PROVENANCE-FIELDS Designed: each entry carries `{producer, model_id?, prompt_rev?, inputs (hashes + spec URIs ~r), epoch, cost, created_at, confidence}`. Not built: there is no entry type at all — a stored object is the rendered text itself, and of those fields only `producer`, `epoch` and the cited spec URIs survive, inside the trailing provenance line. `model_id`, `prompt_rev`, `cost`, `created_at` and `confidence` occur nowhere in the crates. @spec/done

##RENDERED-EXPLANATION-DISPLAYS-PROVENANCE Every rendered explanation **displays** its provenance line ("computed at PROP-003#conditional-deps~r2, epoch 7f3a, model …"). @impl/done

##staleness-becomes-reader-detectable Staleness thereby becomes detectable by the reader even across policy bugs — the last line of defense. @spec/done

## 5. Storage and lifecycle {#storage}

- ##STORAGE-LAYOUT-IS-SHARDED-LIKE-GIT-OBJECTS Layout, built: `.ledger/objects/<sha256[0..2]>/<sha256>`, sharded like git objects, local per checkout, with `.ledger/telemetry.json` beside it. Designed and not built: the small index — nothing writes or reads one — and the CI shared warm copy (standard action-cache pattern, cf. Bazel/sccache — Apache-2.0 prior art, ideas only needed). @spec/done
- ##GC-IS-LRU-WITH-A-PIN-SET GC, designed: LRU with a pin set (entries referenced by the current release slice are pinned); size budget configurable; eviction never affects correctness, only cost. Not built: no collector, no eviction and no size budget exist, so `.ledger/objects/` grows without bound for the life of the checkout. @spec/done
- ##CONCURRENCY-LAST-WRITE-WINS-IS-BENIGN Concurrency: entries are immutable values under content keys; last-write-wins on identical keys is benign. @impl/done
- ##TELEMETRY-FEEDS-THE-HEADLINE-METRIC Telemetry, designed to feed the Charter's headline metric: hit rate, cost per query kind, **LLM-$ per merged change**, and the **contextual-rot rate** — fraction of epoch-invalidated entries whose re-verification *changed the answer*. Built: four counters — hits, misses, rot-checks, rot-changed — of which only the first two move; the rot pair is plumbing no producer increments yet. Not built: both cost measures, there being no cost field and no per-query-kind breakdown. Threshold from the design review, to apply once the rot rate is genuinely measured: if rot among hash-valid entries exceeds ~10–15% per epoch window, the epoch formula is too coarse and gains inputs (e.g., per-subsystem epochs). @spec/done

## 6. Query kinds in v0.1 {#queries}

1. ##QUERY-FACTS-EXTRACT `facts.extract(file)` — frontends (algorithmic). @impl/done
2. ##QUERY-EXPLAIN-ITEM `explain.item(symbol)` — structured subgraph (algorithmic) + optional prose render (LLM, interpretations class). @impl/done
3. ##QUERY-CLASSIFY-LEGACY-UNIT `classify.legacy_unit(text)` — importer support (LLM). Specified, not shipped: neither the query kind nor any importer path exists. @spec/done
4. ##QUERY-PROPOSE-LINKS `propose.links(crate, doc)` — Phase-2 mining (LLM; output lands in the proposals file, *never* directly in code — affirmation is a human diff, PROP-014 §2.7). Specified, not shipped: nothing produces a proposal and no proposals file is ever written; the only trace is the `proposed` edge-provenance variant the wire format can carry. @spec/done

##QUERY-KIND-ADDED-ON-TWO-CONSUMERS Everything else waits for demand: a query kind is added when two distinct consumers ask for it. @impl/done

## 7. The release slice {#release-slice}

##RELEASE-SLICE-IS-EXPORTED-SIGNED-AND-SHIPPED Designed: at tag time a frozen subset — facts for the tagged tree + affirmed interpretations (item summaries, command explanations) — is exported, **signed**, and shipped with the package (the AI-native OSS artifact: agents debugging `vibe` at v0.3.2 query the v0.3.2 slice). Not built: no export, freeze, sign or ship path exists anywhere in the crates; the ledger is local per checkout and git-ignored, which is what the shipped module says of itself. @spec/done

##UNSIGNED-SLICES-ARE-NEVER-EXPOSED-REMOTELY Unsigned slices are not exposed remotely, full stop (PROP-014 §2.8.4). @impl/done

##SIGNING-SCHEME-IS-AN-OPEN-QUESTION Signing scheme is Charter-level Open Question; until it lands, the slice exists for local use only. @impl/done

## 8. Failure modes, named {#failures}

- ##FAILURE-CONFIDENT-STALENESS **Confident staleness** → epochs + hard no-serve + provenance display (§3–4). @impl/done
- ##FAILURE-CACHE-POISONING **Cache poisoning** (a producer writes wrong values at scale) → designed mitigation: producer id + prompt_rev in keys make wholesale invalidation of a bad producer one predicate; release slices are re-derivable from source. Not built as stated: `prompt_rev` is not in the key and appears nowhere in the crates, and no predicate or command sweeps a bad producer's entries — today the only remedy is deleting `.ledger/`. @spec/done
- ##FAILURE-KEY-UNDER-SPECIFICATION **Key under-specification** (two different questions colliding on one key) → designed mitigation: query kinds are a closed enum with reviewed key schemas, so adding a kind is a PR, not a string. Not built: there is no such enum, and the one shipped kind is a producer-id string constant — which is the under-specification this entry was written to close off. @spec/done
- ##FAILURE-LEDGER-WORSHIP **Ledger worship** (treating renders as truth) → renders cite spec URIs; the `--json` raw subgraph is always available; A4 keeps the human the accountability point. @impl/done

---

##UNEXERCISED-POLICY-IS-REMOVED-NOT-CARRIED *Any query kind, key field, or policy here not exercised by Playbook Phase 5 is removed from this document rather than carried as aspiration.* Standing policy, unenforced: nothing checks it and the sweep has not been run against this revision. Until it is, the unexercised items above are marked `@spec/done` and named as designed-not-built rather than silently carried as aspiration. @spec/done
