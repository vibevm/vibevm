# CARD: rule-position-is-a-resource — Position is a resource {#root}

<status stage="spec" state="done"/>

##status-line **Discipline v0.2 · BETA · T2 · Rust** @impl/done

## Band 1 — Identity & Recognition {#band-one-identity}

##CLASSIFICATION Classification: layer=D (context & repo) + H (weak-reader); mechanism=rule. @impl/done

##INTENT Intent: critical invariants stated in comments belong at a file's edges — its top or bottom — where they survive a skim; a marker buried in the middle third is paged past by a reader (human or weak agent) who never reaches it, and the next edit violates it unseen. @impl/done

##ALSO-KNOWN-AS Also Known As: invariant-position; bury-the-invariant; "the comment nobody reads"; R3-003. @spec/done

##APPLICABILITY-RECOGNITION Applicability / Recognition: a source file of at least `invariant_comment_min_file_lines` (default 120) carries a comment whose normalized marker is in the configured `invariant_comment_markers` vocabulary (default the five labeled markers `INVARIANT:` / `WARNING:` / `PANICS:` / `MUST:` / `NEVER:`), and that comment's line `l` satisfies `lines/3 < l <= 2·lines/3` (integer-divided; for a 120-line file, lines 41–80). *Detector seed:* scan for the configured markers, then test each hit's line against the file's physical line count — a hit in the middle third of a file over the floor fires. @impl/done

## Band 2 — Justification & Tradeoffs {#band-two-justification}

##MOTIVATION Motivation: a reader skims a file's top and bottom; the middle is where attention thins. An `INVARIANT:` stating "this counter must never wrap" sitting at line 600 of 900 is read by no one who later edits line 600, so the invariant conditions nothing. Lifting it to the top (beside the contract) or the bottom makes it conditioning-order-first — autoregression makes reading order conditioning order (R3-002's logic, applied to position). @spec/done

##STRUCTURE-AND-PARTICIPANTS Structure & Participants: the conform fact `Fact::InvariantComment { marker, line, in_test }` (emitted by every frontend's comment walk — the Rust frontend's raw-text scan, the TS/Go sidecars) plus `Fact::FileMetrics { lines }` as the denominator → the `invariant-comment-position` rule computes the middle third and emits one finding per buried marker. Fingerprints key on `(file, marker, ordinal)`, **never line** — a line-keyed fingerprint rots on any edit above the comment, and a baseline that rots on unrelated edits is a checker that lies. @impl/done

##COLLABORATIONS Collaborations: rides the same conform gate, SARIF output, and ratchet baseline as `file-length`; the marker vocabulary is a root `conform.toml` key shared across Rust/TS/Go (one vocabulary, not three); the rule re-checks the vocabulary itself rather than trusting the extractor, so a marker dropped from the config (or a stale cached fact) does not red a frozen baseline; the split-the-file remedy is the SWEEP responsibility-split idiom (guide §14). @impl/done

##GOALS-AND-NON-GOALS Goals / Non-Goals: *Goals:* surface invariants a skim misses; keep both the vocabulary and the floor configurable per project. *Non-Goals:* NOT a prose-truth judge — it keys on the marker, not whether the comment is true (lying prose is the sibling `antipattern-lying-prose`, R2C-004); NOT for test-context comments (`in_test` is out of scope); NOT for files below the floor, where a «third» means nothing. @impl/done

##CONSEQUENCES Consequences: (+) invariants that matter sit where they are seen; (+) the floor and the third are explicit and tunable. (−) a marker the author meant block-local but tagged with a file-level token will fire — the vocabulary is curated to prevent exactly this; (−) the rule is near-vacuum in tidy trees (see Evidence), so its value is prospective. @spec/done

##ALTERNATIVES Alternatives: a flat "no comments past line N" rule (too blunt — loses the invariant/not-prose distinction this card exists to make); relying on rustdoc / `@invariant` placement alone (un-enforced, so it drifts). @spec/done

##RISKS-AND-ASSUMPTIONS Risks & Assumptions: assumes the marker vocabulary marks genuine **file-level** invariants, not block-local notes — which is why `SAFETY:` is excluded: in Rust a `SAFETY:` must hug its `unsafe` block (language convention), so it is block-local justification, not a file-level invariant, and moving it to a file edge would be a defect, not a fix; and why a marker is a **labeled tag, not a bare word** — the colon is the markup signal, so bare `NEVER` (emphasis inside an ordinary sentence) is not a marker. Assumes a third is a meaningful "middle", which is false below the floor. *Sunset:* if invariants become machine-bound to their site (a `#[spec(invariant)]` conform already checks at the site), position becomes redundant and this card retires with its checker (R-050). @impl/done

##EVIDENCE-AND-TRANSFER-STRENGTH Evidence & Transfer-strength: checker shipped (`invariant-comment-position`, doctested in `core-ai-native-conform/src/rules/position.rs`); R3-003. Class: built + unit-tested, not yet exercised on buried host invariants. Tag: **[E-mid]**. Honest vacuum: in this tree the markers are near-absent (measure: `SAFETY:` 6, `INVARIANT:` 0, `PANICS` 0, rustdoc `# Safety` 0, TS `@invariant` 0), so the rule is demonstrated on fixtures, not host code — a fact about our tidiness, not an argument against a rule the discipline forbids weakening for disuse. @spec/done

## Band 3 — Operation {#band-three-operation}

```card-ops
trigger: WHEN a comment whose marker is in invariant_comment_markers sits at line l with lines/3 < l <= 2*lines/3 of a file >= invariant_comment_min_file_lines, and not in test context, THEN apply
mode: gate            # a conform rule: runs over the scanned file set at conform time (per-merge), not per-edit (inline) or on a schedule (raid) — the warning arrives through the normal gate
routine:
  1. Read the finding's file + line; confirm the comment carries a configured marker (not a bare word).
  2. Pick an edge: the file top (module/item contract) or the bottom (appendix / legacy note).
  3. Move the comment to that edge, preserving its marker and any #[spec] binding.
  4. If no single edge fits, split the file along its responsibility seam so the invariant lands at an edge of the new file.
  5. Re-run `rust-ai-native conform check --scope <crate>`; the finding clears and the baseline only shrinks.
  6. If the marker is genuinely block-local (e.g. a SAFETY: hugging an unsafe block), leave it — out of the vocabulary by design; never weaken the rule for it.
checker: invariant-comment-position (core-ai-native-conform, T-syn over FileMetrics + InvariantComment facts)
raid_role: layer=conform; order=after:cell-closure; batch=crate
budget: active_rules=1; first_signal=conform scan (content-addressed, cached; <1s/file warm)
```
