# G3-B056-REGISTRY — evidence rows for the B-056 fact registry

**Perimeter read.** Design: `spec/design/multiple-sources-and-plugins.md` (66 lines),
`spec/modules/vibe-workspace/PROP-035-spec-compiler.md` §7.3 (lines 145–172).
Crate `crates/vibe-spec/src/`, read in full: `pipeline/fold.rs` (261),
`pipeline.rs` (401), `gate.rs` (146), `merge.rs` (510), `use_graph.rs` (590),
`resolver.rs` (549), `resolver/glob.rs` (475), `embed.rs` (1–90, the
`SectionSource` trait), and the test files `pipeline/fold_tests.rs` (501),
`pipeline/collision_tests.rs` (156), `pipeline/tests.rs` (484). ~4178 lines of
source examined; grep confirmed the pre-B-056 single-source `.find(...Source)`
is gone and that `source_addresses` is the single `#source`-edge reader shared
by the guard and the fold.

Conventions: every `verdict` is `PENDING` — the boss writes the verdict, never
this file. `match` is the worker's measurement only. Where a fact has several
pinning tests, `test` names the narrowest one that pins the whole assertion and
the others are listed in the impl/note as one-liners (Decision B).

---

## rulings-left
- **claim** — "Left to the build": an N-input fold, the `:replace` flag's threading, a cycle guard + a dedup for the fold, and enumeration in the resolver. (A plan-of-work fact, not a code-site fact.)
- **impl** — NO single code site; the four plan items each landed here: (1) N-input fold → `merge.rs:169` `fn fold_sources(contract, sources: &[&DocTree])`; (2) flag threading → `merge.rs:197` `let any_replace = members.iter().any(...)` and `merge.rs:200-203` (drop contract, emit all members); (3) cycle guard + dedup → `use_graph.rs:84` `fn source_fold_order` (guard) + `pipeline/fold.rs:97` `let mut included: HashSet` (the inclusion-guard dedup); (4) resolver enumeration → `resolver/glob.rs:60` `fn FileResolver::expand_pattern`.
- **test** — НЕТ (no one test pins a plan; each item's own test is named under its fact below).
- **quote** — `pub fn fold_sources(contract: &DocTree, sources: &[&DocTree]) -> String` (`merge.rs:169`)
- **match** — NO-CODE
- **note** — Decision A: NO-CODE, because the assertion is explicitly a "left to build" plan, and the packet's own definition puts work-order / to-be-built claims in NO-CODE. The four landing addresses are listed above so the boss can see the plan is fully landed, not deferred.
- **verdict** — PENDING

## fold-signature
- **claim** — New entry point `fold_sources(contract, sources: &[&DocTree]) -> String`; `fold_source(c, s)` kept as its degenerate case `fold_sources(c, &[s])`; the old fold tests pass through the new path unchanged.
- **impl** — `merge.rs:169` `pub fn fold_sources(...)`; degenerate retained at `merge.rs:242` `pub fn fold_source(contract, source)` → body `merge.rs:243` `fold_sources(contract, &[source])`. The recursive driver `fold_source_closure` (`pipeline/fold.rs:65`) calls `fold_sources` per level (`pipeline/fold.rs:151`).
- **test** — `fold_source_equals_fold_sources_singleton` `merge.rs:484` (asserts byte-for-byte equality of the two over the matched/add/contract-only/source-only branches).
- **quote** — `pub fn fold_source(contract: &DocTree, source: &DocTree) -> String { fold_sources(contract, &[source]) }`
- **match** — SUPPORTS
- **verdict** — PENDING

## fold-per-section
- **claim** — The per-section rule: `S(a)` = sources carrying anchor `a` in declaration order; empty → contract section whole; any `:replace` member → drop contract text, emit members in order; otherwise → contract text minus every fact any member redeclares, then members in order.
- **impl** — `merge.rs:194` `let pieces = if members.is_empty() { vec![contract.text(child)] }` (empty branch); `merge.rs:197-199` `let any_replace = members.iter().any(...)`; `merge.rs:200-203` `if any_replace { members.iter().map(|&(s,sid)| s.text(sid)).collect() }` (replace branch); `merge.rs:204-212` else branch: `overridden_facts(...)` then `contract.text_without(child, &dropped)` then members. Source-only append: `merge.rs:224-234`.
- **test** — `fold_sources_add_two_in_slice_order` `merge.rs:408` pins the `:add` order; replace branch pinned by `fold_sources_replace_on_second_keeps_both_sources` `merge.rs:421`; union-override branch by `fold_sources_add_union_of_overridden_facts` `merge.rs:438`; empty slice by `fold_sources_empty_slice_emits_contract_whole` `merge.rs:502`.
- **quote** — `let any_replace = members.iter().any(|&(s, sid)| MergeMode::from_trailing(&s.node(sid).trailing) == MergeMode::Replace);`
- **match** — SUPPORTS
- **verdict** — PENDING

## fold-per-section-delta
- **claim** — The delta from the old single-source law is two words, "any" and "every": one source becomes a sequence; fact-override becomes a union over that sequence.
- **impl** — "any": `merge.rs:197` `members.iter().any(... :replace)` (one `:replace` suffices). "every"/union: `overridden_facts` `merge.rs:142`, whose source-id set is the union `members.iter().flat_map(...).map(...).collect::<HashSet>` (`merge.rs:147-151`); applied at `merge.rs:207`.
- **test** — `fold_sources_replace_on_second_keeps_both_sources` `merge.rs:421` pins "any" (a `:replace` on the second source alone drops the contract text); `fold_sources_add_union_of_overridden_facts` `merge.rs:438` pins "every" (s1 redeclares fact-a, s2 redeclares fact-b, both contract versions vanish).
- **quote** — `let source_ids: HashSet<&str> = members.iter().flat_map(|&(s, sid)| s.facts_under(sid)).map(|(_, a)| a).collect();`
- **match** — SUPPORTS
- **verdict** — PENDING

## fold-override-is-a-union
- **claim** — The dropped fact set is the union over all sources; a fact redeclared by two sources at once is not a fold problem — both redeclarations survive into the merged text and the compiler's anchor-uniqueness recheck fails on the duplicate.
- **impl** — `merge.rs:142-158` `overridden_facts` (union via `flat_map`+`HashSet`, "one id, one unit"); used at `merge.rs:207`. The "two redeclarations survive → gate fails" half: `fold_sources` does NOT dedup source-only sections between sources (`merge.rs:224-234`), so two sources' redeclarations both reach the merged text, and `gate::first_duplicate` (`gate.rs:54`) flags the surviving duplicate.
- **test** — `fold_sources_add_union_of_overridden_facts` `merge.rs:438` (union drop). The "two sources, same id, gate trips" half is pinned by `a_fact_duplicate_between_two_sources_fails_the_build` `pipeline/fold_tests.rs:182` (expects `CompileError::DuplicateId`).
- **quote** — `contract.facts_under(cid).into_iter().filter(|(_, a)| source_ids.contains(a))` (`merge.rs:152-155`)
- **match** — SUPPORTS
- **verdict** — PENDING

## fold-source-only-collision
- **claim** — A source-only anchor declared by two sources IS an error; the asymmetry (declaration idempotent, definition not) is deliberate. [In this revision the last phrase — "the post-merge uniqueness gate catches it" — was removed; verify the catcher separately.]
- **impl** — The catcher is in the FOLD, not the post-merge gate: `pipeline/fold.rs:173` `if let Some(anchor) = first_source_section_collision(&contract_tree, &member_refs)` → `CompileError::DuplicateSourceSection` (`pipeline.rs:60-66`). The detector itself: `first_source_section_collision` `pipeline/fold.rs:211`, counting distinct members per source-only anchor and flagging `>= 2` (`pipeline/fold.rs:223`, `:243`). The post-merge gate deliberately does NOT catch a pure heading repeat (`gate.rs:64`), confirming the removal of the old "gate catches it" phrase.
- **test** — `two_sources_defining_one_source_only_section_fails_even_without_a_fact` `pipeline/collision_tests.rs:24` (no fact inside, still `DuplicateSourceSection`); the "matching a contract anchor is a legal :add, not a collision" mirror is `two_sources_matching_a_contract_section_is_a_legal_add_not_a_collision` `pipeline/collision_tests.rs:61`; the merge-layer "no dedup between sources" is `fold_sources_source_only_no_dedup_between_sources` `merge.rs:462`.
- **quote** — `if let Some(anchor) = first_source_section_collision(&contract_tree, &member_refs) { return Err(CompileError::DuplicateSourceSection { addr: key.clone(), anchor }); }`
- **match** — SUPPORTS
- **note** — Verified the catcher question the packet posed: the fold catches it (`pipeline/fold.rs:173`), because the post-merge gate (`gate.rs:64`) skips pure heading-vs-heading repeats. Stale residue: the prose comment at `merge.rs:25-27` and `merge.rs:219-223` still claims "the duplicate trips the post-merge anchor-uniqueness check by design" — that is the refuted sentence (see fact `fold-collision-catcher-was-wrong`); the comment was not updated when the fold-level catcher was added.
- **verdict** — PENDING

## fold-order-is-declaration-order
- **claim** — Order is declaration order; a glob is sorted before it joins; the composed document is a pure function of (tree, lockfile), independent of filesystem enumeration order.
- **impl** — Declaration order: `use_graph.rs:200-217` `source_addresses` iterates `Directives::parse(text).directives` top-to-bottom and flattens each in place. Glob sorted before joining: `resolver/glob.rs:109` `matched.sort_by(...)` (then `dedup_by` `:113`) before addresses are built. Fold order comes from `source_fold_order` (`use_graph.rs:84`), whose DFS posts nodes deepest-first independent of read order.
- **test** — Declaration order: `sources_fold_in_declaration_order_not_alphabetical` `pipeline/fold_tests.rs:120` (s2-before-s1 ⇒ s2 first) and `source_fold_preserves_declaration_order` `use_graph.rs:457`. Sort independent of creation/read order: `g2_sort_independent_of_creation_order` `resolver/glob.rs:282`.
- **quote** — `matched.sort_by(|a, b| match a.0.cmp(&b.0) { Ordering::Equal => a.1.cmp(&b.1), ord => ord });`
- **match** — SUPPORTS
- **verdict** — PENDING

## recursion-what-to-extend
- **claim** — The fold needs its own traversal over `#source` edges with the same three colours and the same `is_contract` predicate; reuse the existing walker rather than writing a second one.
- **impl** — The existing walker is reused: `source_fold_order` `use_graph.rs:84` calls the SAME `visit` (`use_graph.rs:102`) that `topo_order_from` (`use_graph.rs:57`) uses, parameterised by `EdgeKind` (`use_graph.rs:46-51`, `Color` Gray/Black/None at `:37-40`). `is_contract` is shared (`use_graph.rs:223`, keyed on a `contract` path segment). The fold's recursion is driven over this order by `fold_source_closure` `pipeline/fold.rs:65`.
- **test** — `source_edges_and_use_edges_do_not_mix` `use_graph.rs:526` (one traverser, two disjoint edge sets). The `is_contract` predicate's cycle rulings: `a_source_cycle_between_contracts_is_admitted` `use_graph.rs:482`, `a_source_cycle_touching_an_impl_is_rejected` `use_graph.rs:501`.
- **quote** — `fn visit(addr: &SpecAddress, source: &impl SectionSource, state: &mut HashMap<String, Color>, order: &mut Vec<String>, path: &mut Vec<String>, kind: EdgeKind) -> Result<(), UseGraphError>`
- **match** — SUPPORTS
- **verdict** — PENDING

## plugins-only-enumeration-is-new
- **claim** — The glob adds exactly one capability: the resolver must enumerate installed packages matching a pattern instead of resolving one coordinate; everything downstream is the sequence fold of §3.
- **impl** — `resolver/glob.rs:60` `fn FileResolver::expand_pattern` scans `vibedeps/` slots, name-matches (`name_matches` `resolver/glob.rs:141`), keeps those carrying the doc, sorts, and returns concrete addresses. The downstream fold is the ordinary sequence fold: a glob's members are flattened by `source_addresses` (`use_graph.rs:196`) and folded by `fold_source_closure` like any point `#source`.
- **test** — `g1_two_matches_in_name_order` `resolver/glob.rs:264` (enumerates two, sorted). End-to-end into the fold: `a_glob_source_folds_all_its_members_in_sorted_order` `pipeline/tests.rs:344`.
- **quote** — `pub fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, ResolveError>`
- **match** — SUPPORTS
- **verdict** — PENDING

## cost-order
- **claim** — Build order, each step landable alone: (1) `fold_sources` over an explicit list; (2) the pipeline stops taking `.find(...)` and passes every `#source` in declaration order; (3) the fold's cycle guard and dedup over the extended `use_graph` walker; (4) resolver enumeration for the glob, sorted.
- **impl** — NO single code site (a build-order plan); landings: (1) `merge.rs:169` `fold_sources`; (2) the `.find(|d| d.kind == DirectiveKind::Source)` is GONE (grep over `crates/vibe-spec/src/` returns NONE) — the pipeline now folds the whole closure via `fold_source_closure` (`pipeline.rs:155`) and the per-level members come from `source_addresses` in declaration order (`use_graph.rs:200`); (3) cycle guard `source_fold_order` `use_graph.rs:84` + inclusion-guard dedup `pipeline/fold.rs:97`; (4) `expand_pattern` `resolver/glob.rs:60` with sort at `:109`.
- **test** — НЕТ (a plan, not a single assertion). The four steps' own tests: `fold_source_equals_fold_sources_singleton` (`merge.rs:484`), `two_sources_both_folded_in_declaration_order` (`pipeline/fold_tests.rs:88`), `a_diamond_includes_the_shared_source_once` (`pipeline/fold_tests.rs:315`), `a_glob_source_folds_all_its_members_in_sorted_order` (`pipeline/tests.rs:344`).
- **quote** — `let folded = fold_source_closure(&text, &addr, source)?;` (`pipeline.rs:155`) — the pipeline no longer `.find`s one source.
- **match** — NO-CODE
- **note** — Decision A: NO-CODE, same rationale as `rulings-left` — the assertion is a work-order / build-sequence claim. The grep measurement (`.find(...Source)` → 0 hits) is the concrete evidence step (2) actually landed.
- **verdict** — PENDING

## fold-collision-catcher-was-wrong
- **claim** — NEW fact: `gate::first_duplicate` deliberately tolerates a repeated heading, so two sources declaring one source-only section passed SILENTLY when no fact sat inside; the gate cannot be the catcher (provenance is gone by then); the fold must catch it. Asks: (a) test pinning the gate's heading tolerance; (b) the fold site that catches it; (c) a test on the collision.
- **impl** — (a) gate tolerance: `gate.rs:64` `if tree.node(prev).kind == NodeKind::Fact || tree.node(nid).kind == NodeKind::Fact { ... }` — a pure heading-vs-heading repeat falls through and is skipped. (b) fold catcher: `pipeline/fold.rs:173` `first_source_section_collision(...)` → `CompileError::DuplicateSourceSection` (`pipeline.rs:60`); the detector `pipeline/fold.rs:211` runs pre-fold where each source's tree is still separate, and only as a fallback AFTER `first_duplicate` (`pipeline/fold.rs:157`) so a colliding fact still names its more specific id. (c) collision test below.
- **test** — (a) `a_repeated_section_heading_is_not_flagged` `gate.rs:131` (asserts `first_duplicate(...).is_none()` on a repeated `# API {#root}`). (c) `two_sources_defining_one_source_only_section_fails_even_without_a_fact` `pipeline/collision_tests.rs:24` (expects `DuplicateSourceSection` with no fact present).
- **quote** — `if tree.node(prev).kind == NodeKind::Fact || tree.node(nid).kind == NodeKind::Fact { return Some(DuplicateId { ... }); }` — only a fact-on-either-side repeat fires.
- **match** — SUPPORTS
- **note** — The packet's three asks are all satisfied: (a) `gate.rs:131`, (b) `pipeline/fold.rs:173`, (c) `pipeline/collision_tests.rs:24`. The stale `merge.rs:219-223` comment is the on-disk residue of the refuted sentence this fact names.
- **verdict** — PENDING

## recursion-dedup-is-two-things
- **claim** — NEW fact: the walker dedups NODES, the fold is textual INCLUSION, and the first does not imply the second; in a diamond both parents inline the shared source, so the seed would carry its body twice; therefore the fold carries an INCLUDE GUARD — a node's body enters the document once, by the first path in the deterministic fold order.
- **impl** — Include guard: `pipeline/fold.rs:97` `let mut included: HashSet<String> = HashSet::new()`; the skip `pipeline/fold.rs:140-141` `if included.contains(&mk) { return None; // (2) inclusion guard: body already inlined }`; the first-inline record `pipeline/fold.rs:144` `included.insert(mk);`. The two-reason skip is documented inline at `pipeline/fold.rs:118-129` (forward-declaration ancestor vs already-included) so the two dedups are not conflated. (Node dedup in the walker is the `Color::Black` short-circuit at `use_graph.rs:112`.)
- **test** — Diamond guard: `a_diamond_includes_the_shared_source_once` `pipeline/fold_tests.rs:315` (asserts `d-body` count == 1); the load-bearing case with a fact: `a_diamond_with_a_shared_fact_compiles` `pipeline/fold_tests.rs:353` (asserts `##shared` count == 1). Walker node-dedup: `source_fold_deduplicates_a_diamond` `use_graph.rs:432`.
- **quote** — `if included.contains(&mk) { return None; // (2) inclusion guard: body already inlined }`
- **match** — SUPPORTS
- **note** — The current diamond test asserts ONE copy (`pipeline/fold_tests.rs:345-349`), i.e. the guard is in force; the design's "first asserted two copies because that is what the code did" describes the pre-guard state the guard was added to fix.
- **verdict** — PENDING

## sequence-lead
- **claim** — The heading line of the §7.3 block: «Several sources, and the plugin form (B-056, owner-ruled 2026-08-04, built 2026-08-05)».
- **impl** — `spec/modules/vibe-workspace/PROP-035-spec-compiler.md:164` — `##sequence-lead **Several sources, and the plugin form** *(B-056, owner-ruled 2026-08-04, built 2026-08-05)*:`. This is a spec-prose heading; there is no code site in `crates/vibe-spec/src/**` that "implements" a heading string.
- **test** — НЕТ
- **quote** — `##sequence-lead **Several sources, and the plugin form** *(B-056, owner-ruled 2026-08-04, built 2026-08-05)*:`
- **match** — NO-CODE
- **note** — Decision: NO-CODE — the assertion is about the existence of a prose heading in the spec, not about code behaviour (the packet's NO-CODE definition: "утверждение не про код"). The heading is verbatim at the cited spec line; the seven code facts it introduces (14–19) are measured below.
- **verdict** — PENDING

## SOURCE-SEQUENCE
- **claim** — A contract may declare more than one `#source`, and every one is honoured, in declaration order; before this the compiler took the FIRST `#source` and dropped the rest silently.
- **impl** — All `#source` directives are gathered: `use_graph.rs:196` `fn source_addresses` iterates EVERY `DirectiveKind::Source` (`use_graph.rs:201-204`) and flattens them in declaration order (`use_graph.rs:192-195`); the fold passes the full member slice to `fold_sources` (`pipeline/fold.rs:130-151`). The old first-only `.find(...Source)` is GONE (grep → 0 hits).
- **test** — `two_sources_both_folded_in_declaration_order` `pipeline/fold_tests.rs:88` (both bodies survive, index-checked contract < s1 < s2). Also `no_source_directive_lines_remain_with_two_sources` `pipeline/fold_tests.rs:251` (every `#source` line stripped), `sources_fold_in_declaration_order_not_alphabetical` `pipeline/fold_tests.rs:120`.
- **quote** — `for d in Directives::parse(text).directives { if d.kind != DirectiveKind::Source { continue; } ... out.extend(expanded); }`
- **match** — SUPPORTS
- **verdict** — PENDING

## SOURCE-REPLACE-IS-A-FLAG
- **claim** — `:replace` from ANY source discards only the contract text; the sources still sum among themselves, in order; with one source the result is byte-identical to the prior single-source replace.
- **impl** — `merge.rs:197-199` `any_replace = members.iter().any(... :replace)`; `merge.rs:200-203` on any-replace: `members.iter().map(|&(s, sid)| s.text(sid)).collect()` — contract text dropped, ALL members emitted in slice order. The byte-identical-to-single-source guarantee is structural: `fold_source` delegates to `fold_sources(c, &[s])` (`merge.rs:243`), and the singleton path is pinned by `fold_source_equals_fold_sources_singleton` (`merge.rs:484`).
- **test** — `fold_sources_replace_on_second_keeps_both_sources` `merge.rs:421` (replace on the 2nd source drops contract, keeps both sources in order). Pipeline-level: `replace_in_second_source_drops_contract_keeps_both_sources` `pipeline/fold_tests.rs:149`.
- **quote** — `members.iter().map(|&(s, sid)| s.text(sid)).collect()` — contract side dropped, members summed.
- **match** — SUPPORTS
- **verdict** — PENDING

## SOURCE-FACT-OVERRIDE-IS-A-UNION
- **claim** — A contract fact is dropped when its id is redeclared by ANY member of `S(a)` (the override set is the union); two sources redeclaring one id is not a fold question — both survive and the uniqueness recheck fails on the duplicate.
- **impl** — `merge.rs:142-158` `overridden_facts`: the source-id set is the union `members.iter().flat_map(|&(s,sid)| s.facts_under(sid))...collect::<HashSet>` (`merge.rs:147-151`); contract facts whose id is in that set are dropped (`merge.rs:152-157`); applied at `merge.rs:207`. The "two redeclare one id ⇒ gate fails" half: source-only sections are not deduped between sources (`merge.rs:224-234`), and `gate::first_duplicate` (`gate.rs:54`) trips on the surviving duplicate.
- **test** — `fold_sources_add_union_of_overridden_facts` `merge.rs:438` (s1 redeclares fact-a, s2 redeclares fact-b → both contract versions gone, each id once). The two-sources-same-id gate trip: `a_fact_duplicate_between_two_sources_fails_the_build` `pipeline/fold_tests.rs:182`.
- **quote** — `let source_ids: HashSet<&str> = members.iter().flat_map(|&(s, sid)| s.facts_under(sid)).map(|(_, a)| a).collect();`
- **match** — SUPPORTS
- **verdict** — PENDING

## SOURCE-ONLY-IS-A-DEFINITION
- **claim** — A source-only section declared by two sources is an error, and it is judged IN THE FOLD, not in the post-merge uniqueness gate (the gate sees provenance-free text and deliberately tolerates a repeated heading as the `:add` artifact).
- **impl** — Fold catcher: `pipeline/fold.rs:173` `first_source_section_collision(...)` → `CompileError::DuplicateSourceSection` (`pipeline.rs:60-66`); detector at `pipeline/fold.rs:211` (`>= 2` distinct members at `:243`), placed as a fallback AFTER `first_duplicate` (`pipeline/fold.rs:157`). The gate's deliberate heading tolerance that forces this into the fold: `gate.rs:64` (only a fact-on-either-side repeat fires).
- **test** — `two_sources_defining_one_source_only_section_fails_even_without_a_fact` `pipeline/collision_tests.rs:24` (fold fails naming the anchor). The gate's tolerance that the fold compensates for: `a_repeated_section_heading_is_not_flagged` `gate.rs:131`. Inner-level naming: `a_source_section_collision_at_an_inner_level_names_the_inner_node` `pipeline/collision_tests.rs:113`.
- **quote** — `// ...the post-merge first_duplicate gate cannot see this: by the time it runs, provenance is folded away... Caught pre-fold` (`pipeline.rs:51-59`, `pipeline/fold.rs:173`).
- **match** — SUPPORTS
- **verdict** — PENDING

## SOURCE-RECURSION
- **claim** — The fold follows `#source` recursively under §9's cycle law and carries an include guard; a contract cycle is legal (forward declaration), a cycle touching an implementation is a build error naming the path; the same walker, one more edge set, never a second traversal.
- **impl** — Recursion driver: `fold_source_closure` `pipeline/fold.rs:65`, iterating `source_fold_order` (`use_graph.rs:84`) deepest-first; guard = the shared `visit` (`use_graph.rs:102`) with `EdgeKind::Source`. Cycle law: `use_graph.rs:120` `if is_contract(&key) && loop_nodes.iter().all(|k| is_contract(k)) { return Ok(()); }` (legal contract cycle); else `use_graph.rs:123-125` `Err(UseGraphError::Cycle(cycle))` (impl-touching cycle; `is_contract` keyed on a `contract` path segment at `use_graph.rs:223-227`). Include guard: `pipeline/fold.rs:97`/`:140-141`/`:144`. Non-Unresolved graph errors map to `CompileError::UseGraph` (`pipeline/fold.rs:80-81`), whose Display carries the path.
- **test** — Contract cycle legal: `a_source_cycle_between_contracts_compiles` `pipeline/fold_tests.rs:392` (and `use_graph.rs:482`). Impl cycle error: `a_source_cycle_through_an_impl_fails` `pipeline/fold_tests.rs:413` (expects `CompileError::UseGraph`, and `use_graph.rs:501` checks the path). Include guard: `a_diamond_includes_the_shared_source_once` `pipeline/fold_tests.rs:315`. One-walker: `source_edges_and_use_edges_do_not_mix` `use_graph.rs:526`.
- **quote** — `if is_contract(&key) && loop_nodes.iter().all(|k| is_contract(k)) { return Ok(()); }`
- **match** — SUPPORTS
- **note** — The path IS named in the error (`UseGraphError::Cycle` formats `a -> b -> a`, `use_graph.rs:30`, surfaced via `CompileError::UseGraph` `pipeline.rs:39-40`). Cosmetic only: the label string reads "use cycle: …" for a `#source` cycle (the same `Display` serves both edge kinds) — the path is correct, only the word "use" is generic. Not a behavioural divergence.
- **verdict** — PENDING

## SOURCE-GLOB
- **claim** — `*` is allowed in the package-NAME half; it names every installed package whose name matches AND that carries the addressed document, expanded in sorted order; an empty match is a legal empty set; a pointed address still fails loudly; BOTH the fold and the guard read a document's `#source` edges through ONE function.
- **impl** — Name-only pattern: `resolver/glob.rs:47` `fn is_pattern` (`name.contains('*')`). Enumeration: `resolver/glob.rs:60` `expand_pattern` — scans slots (`:84`), first-hyphen split (`:94`), name match (`name_matches` `:141`), membership = name AND doc present (`:121-128`), sort (`:109`) + dedup (`:113`); non-pattern returns itself (`:68-70`); empty ⇒ `Ok(vec![])`. Pointed address fails loudly: `resolver.rs:126-130` `resolve_file` refuses a pattern with `PatternNotExpanded` (and a missing pointed source raises `Unresolved` in `visit`, `use_graph.rs:133-138`). ONE function for `#source` edges: `use_graph.rs:196` `fn source_addresses`, called by the guard (`use_graph.rs:168` `EdgeKind::Source => source_addresses(...)`) AND by the fold (`pipeline/fold.rs:130`); both expand the glob identically through `SectionSource::expand_pattern` (`embed.rs:45`).
- **test** — Enumeration+sort: `g1_two_matches_in_name_order` `resolver/glob.rs:264`; empty set legal: `g5_empty_set_is_legal` `resolver/glob.rs:330`; pointed resolve of pattern is loud: `g9_point_resolve_of_pattern_is_pattern_error` `resolver/glob.rs:396`; name-only star: `g_is_pattern_only_name_star` `resolver/glob.rs:458`. ONE-function proof (guard and fold agree through a glob edge): `the_fold_reaches_a_source_through_a_glob_expanded_edge` `pipeline/tests.rs:444`.
- **quote** — `EdgeKind::Source => source_addresses(&text, source)?,` — the guard and the fold share this one edge reader.
- **match** — SUPPORTS
- **verdict** — PENDING
