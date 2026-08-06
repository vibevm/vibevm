# The map query language — А5b {#root}

<status stage="spec" state="work" comment="boss design for the programme's item А5b (the second query level), captured 2026-08-06 on the M-A5B measurement over the committed map; the owner's ruling of 2026-08-06 is to build BOTH levels, and the filter level shipped the same day as `471e3b1b` — this is the level above it and nothing else"/>

@fact:design-scope **Scope.** The owner ruled that map search is built at **two**
levels. The first — three independent filters joined by AND under a hard ceiling
— shipped on 2026-08-06 and is a **permanent** level, not a first draft.
[`BACKLOG.md`](../../BACKLOG.md) `##B018-PARTS` names the second: the filters
*plus graph traversal* — depth, and «has no edge of this kind», the one that
answers *«which rules does nothing verify»*. This document designs that level and
nothing else. @status:spec/done

@fact:design-stands-on-not-instead-of **It stands on the filter level, and the
library keeps them apart on purpose.** The ruling recorded at
`spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#map-query` is explicit:
the filters must work on their own and must never become a degenerate case of a
grammar, so in the library they are their own entry point — a broken parser
cannot take them down with it. This level therefore **calls** the filter level to
pick its seeds; it does not reimplement filtering, and it does not sit under it. @status:spec/done

@fact:design-the-price-was-named-in-advance **The price is a grammar that will
need versioning**, and it was named before the work was authorised rather than
discovered during it. That is the whole difference between this level and the
one below, and the design below spends most of its care on keeping the grammar
small enough that versioning it stays cheap. @status:spec/done

## 1. What was measured at design time {#measured}

@fact:m-readings-are-dated **Every reading in this section is dated 2026-08-06
and was taken BEFORE any code was written**, over the committed `specmap.json`
(schema 3) — the standing law that a plan is measured against the authored tree
before it is built. They are kept as they were taken; a later re-measurement goes
beside them rather than over them. @status:spec/done

@fact:m-the-graph-is-small-and-bipartite **The graph is bipartite, directed
code→spec, and small.** 5 825 spec units, 1 006 code items, **955 edges**, all of
provenance `authored`; 0 suspects, 208 warnings. Every edge's `from_symbol`
resolves to a code item (955 of 955); 934 of 955 resolve to a spec unit at the
far end. @status:spec/done

@fact:m-three-verbs-of-five-exist **Three of the five edge verbs exist in this
tree:** `implements` 716, `verifies` 225, `deviates` 14. `documents` and
`informs` are **zero**. A `lacks:documents` query would therefore return every
unit in the corpus — true, and useless. The grammar accepts them anyway, because
the vocabulary is the schema's and a filter that silently rejects a legal verb is
worse than one that answers «all of them». @status:spec/done

@fact:m-the-canonical-answer-is-enormous **The canonical question's raw answer is
5 742 rows against a ceiling of 200.** 98.6 % of spec units carry no incoming
`verifies` edge; 95.5 % carry no edge of any kind. The negative predicate on its
own is not a usable query — it is a statement that the corpus is mostly
unverified, which was already known. @status:spec/done

@fact:m-kind-cannot-narrow-a-spec-unit **The existing filters cannot narrow it,
and this is the design's load-bearing finding.** `kind` is carried by **0 of
5 825** spec units — every unit in this tree is legacy-unmarked, exactly as
`##MAP-QUERY-THE-KIND-VOCABULARY-IS-MEASURED-NOT-INVENTED` already records — and
`status` and `revision` are likewise 0. The `uri` filter is an **exact** match,
so it names precisely one unit. Composing the negative predicate with what
exists therefore narrows 5 742 to either 5 742 or 1, and neither is the question
anyone asks. @status:spec/done

@fact:m-scope-is-what-makes-it-answerable **A document scope is what turns the
question into an answer, and the numbers say by how much.** The map spans **72
distinct documents**; units per document run 358 at the largest, **57 at the
median**, 10 at the smallest. Scoping the canonical query to one document brings
**67 of the 72** inside the 200 ceiling. The five that still truncate are named
rather than hidden: `PROP-005` (349 of 358 lack `verifies`), `PROP-003` (308),
`PROP-002` (279), `PROP-019` (234), `PROP-000` (204). @status:spec/done

@fact:m-depth-has-a-subject-and-a-sample-said-otherwise **Depth is not vacuous,
and the first reading of it was wrong.** Sampling the three highest-degree nodes
showed depth 2 reaching exactly what depth 1 reached, which would have made a
depth dimension a feature with no subject. Measured exhaustively over all 1 205
edge-bearing nodes instead: **864 of them — 71.7 % — reach strictly more at depth
2 than at depth 1**, 71 reach more at depth 3, and component diameters run to
**8 hops**. The three sampled nodes were star centres whose leaves happen to
carry one edge each; a sample of the most connected nodes is not the graph. @status:spec/done

@fact:m-the-blast-radius-is-bounded **The traversal cannot explode, measured
rather than hoped.** The edge-bearing subgraph falls into **255 connected
components**, the largest holding **44** nodes and the next four 25, 23, 22, 21.
An unbounded walk from any seed is therefore bounded by 44 — comfortably under
the ceiling — so depth is a precision control here, not a safety one. The ceiling
stays hard regardless: it protects against a future map, not against this one. @status:spec/done

@fact:m-twenty-one-edges-dangle **21 edges point at a URI that names no unit in
the map, over five distinct targets — and four of the five are correct.**
`ENGINE-CONFORM-v0.1#rules` (11 edges), `PROP-014#queries` (6), `PROP-014#index`
(1) and the fully-qualified `…/core-ai-native/mechanisms/PROP-014#addressing-code`
(1) all address **installed packages**, which are deliberately outside the
project map so its byte-reproducibility holds. The fifth is not: see
`##m-one-dangling-edge-is-a-host-defect`. @status:spec/done

@fact:m-one-dangling-edge-is-a-host-defect **The fifth dangling target is the
host citing an anchor of its own that has never existed.** Two `specmark::scope!`
declarations — `crates/vibe-cli/src/commands/tools.rs:13` and
`crates/vibe-workspace/src/tools.rs:16` — address
`spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#binaries`, and
PROP-025 carries no such anchor (`build`, `cross-package`, `dispatch`, `gc`,
`history`, `manifest`, `problem`, `root`, `security`, `staleness`, `v1-cut`).
This is the recorded «`vibe tools` shipped with no spec document» finding wearing
its other face, and no gate reports it. Filed separately; this document only
records that the measurement surfaced it. @status:spec/done

## 2. The grammar {#grammar}

@fact:g-one-conjunction-no-operators **A query is a conjunction of predicates and
nothing else.** No `OR`, no parentheses, no precedence, no grouping. Every one of
those is a permanent versioning liability, and the measured questions need none
of them: the corpus's real queries are «inside this document, units lacking this
edge» and «what does this reach». A grammar whose whole syntax is *«predicates
separated by whitespace»* can be extended by adding a predicate without ever
renumbering the language. @status:spec/plan

@fact:g-the-predicate-set **Seven predicates, and the first three are the filter
level reached from above rather than reimplemented:** @status:spec/plan

```
query      := predicate ( WS predicate )*
predicate  := "uri:"    <exact spec:// address>
            | "symbol:" <substring of a code symbol>
            | "kind:"   <item_kind or spec-unit kind>
            | "scope:"  <spec:// address prefix>
            | "has:"    <implements|verifies|documents|deviates|informs>
            | "lacks:"  <implements|verifies|documents|deviates|informs>
            | "depth:"  <0..3>
```

@fact:g-scope-earns-its-place-by-measurement **`scope:` is in the set because
`##m-scope-is-what-makes-it-answerable` measured that without it the level's own
canonical question is unanswerable.** It is a prefix match over the unit's URI,
which is the one axis this corpus actually carries — 72 documents, median 57
units. It applies to spec units only, exactly as `uri:` does. @status:spec/plan

@fact:g-has-and-lacks-are-seed-predicates **`has:` and `lacks:` select seeds; they
do not filter the traversal's result.** A node passes `lacks:verifies` when no
edge of that verb touches it — incoming for a spec unit, outgoing for a code
item, which is the only reading that makes one predicate serve both families of
a directed bipartite graph. Applying them after expansion would answer a
different question nobody asked, and would make `depth:0` differ from no depth
at all. @status:spec/plan

@fact:g-depth-expands-the-seed-set **`depth:N` expands the seed set along edges,
undirected, N hops, and the seeds stay in the answer.** `depth:0` is the
identity and is the default, so a query without `depth:` is exactly the seed
selection — which is what keeps this level a strict superset of the one below
rather than a different thing wearing its name. The upper bound is 3, chosen
against `##m-the-blast-radius-is-bounded`: an unbounded walk on this map reaches
at most 44 nodes, and 5.9 % of nodes gain anything at all past depth 2, so
depth 3 already exceeds what the corpus can use. @status:spec/plan

@fact:g-hops-are-reported **A traversal hit carries the hop count it was reached
at**, and a seed carries 0. Without it the caller cannot tell what it asked for
from what the walk dragged in, and an agent reading a 40-row answer has no way
to rank it. @status:spec/plan

@fact:g-unknown-is-an-error-never-ignored **An unknown predicate, an unknown
verb, or a malformed value is a parse ERROR that names the offending token and
lists what was expected — never a silently ignored clause.** This is the same law
`##B068-UNKNOWN-TYPE-IS-AN-ERROR` states for the markup's typed fences, for the
same reason: a grammar that ignores what it does not understand promises
everything and checks nothing, and the promise is discovered by whoever trusted
it. @status:spec/plan

@fact:g-the-version-is-in-the-answer-not-the-query **The grammar carries a
version, and it is reported in every answer rather than demanded in every
query.** A query string stays free of ceremony; the structured answer states the
grammar version it was parsed under, so a consumer that cares can branch and one
that does not is unaffected. Demanding a version prefix in the query would put
the cost of versioning on every caller forever to buy nothing until the first
breaking change. @status:spec/plan

@fact:g-the-empty-query-is-refused **An empty query is refused, not treated as
«everything».** The filter level already answers «a bounded slice of the whole
map» for a bare call, and it is one function call away; a second spelling of it
here would be a second implementation of the same answer. @status:spec/plan

## 3. What this level does NOT get {#non-goals}

@fact:ng-no-or-no-not **No disjunction and no free negation.** `lacks:` is the
only negative form, and it is a predicate rather than an operator precisely so
that the grammar has no operator layer to version. A caller wanting a union runs
two queries; a caller wanting arbitrary negation is asking for a different tool. @status:spec/plan

@fact:ng-results-stay-nodes **Results stay nodes, never edges** —
`##MAP-QUERY-RESULTS-ARE-NODES` already rules this and traversal does not reopen
it. «Find me an edge» remains a question nobody asks; the hop count is how an
edge shows up in an answer. @status:spec/plan

@fact:ng-no-second-producer-here **The code-quality engine's findings do not join
here.** The owner's ruling that two engines must not merge their data, and that
any join happens at query time over `file`/`line`, is recorded at
`##B019-V-RECOMMENDATION` and remains unbuilt. This level adds no new obstacle to
it — the hit shape already carries a discriminated source field — and it builds
none of it. @status:spec/plan

@fact:ng-no-persistence **Nothing is cached or persisted.** The map is built
fresh per call exactly as the filter level and `explain` build it, for the reason
`##MAP-QUERY-BUILT-FRESH-LIKE-EXPLAIN` gives: a query answers for the tree as it
is. Traversal changes the cost of a call, not its freshness contract. @status:spec/plan

## 4. The cut {#cut}

@fact:cut-slice-1 **Slice 1 — the level, whole.** The parser, the seed
predicates, `scope:`, `has:`/`lacks:`, `depth:`, the hop count, both renderings,
and the two surfaces. It is one slice because the parser without the predicates
answers nothing and the predicates without the parser are the level below. @status:spec/plan

@fact:cut-slice-2 **Slice 2 — the contract moves into the specification.**
PROP-015 gains the level's section beside `#map-query`, and this document keeps
only the reasoning. Per the standing law, the facts that move are judged in the
same pass that moves them. @status:spec/plan

@fact:cut-what-is-deliberately-not-cut **The dangling-edge gate is NOT part of
this work**, though the measurement that designed this level is what found the
host's broken address. A query that can ask the question is not a gate that
answers it every commit, and conflating them would make this slice unfinishable. @status:spec/plan

## 5. Rejected {#rejected}

@fact:rej-flags-instead-of-a-grammar **Rejected: expressing the level as CLI
flags and MCP fields, with no parser at all.** It is genuinely cheaper and it is
what the deferral note recommended for the level below — but the owner ruled a
query LANGUAGE, the versioning cost was accepted in advance
(`##design-the-price-was-named-in-advance`), and a flag set cannot grow a
predicate without growing both surfaces. The grammar is one string both surfaces
pass through unread. @status:spec/plan

@fact:rej-replacing-the-filter-level **Rejected: folding the filters into the
grammar so there is one level.** The owner's ruling forbids it in as many words —
the filter level is permanent and separately reachable so that a broken grammar
cannot take it down. This is also why the parser lives in its own module with its
own entry point rather than inside the search function. @status:spec/plan

@fact:rej-unbounded-depth **Rejected: unbounded traversal, or a depth the caller
may raise freely.** The measured component ceiling is 44 today, which makes an
unbounded walk look safe — on this map, this week. A bound that is safe only
because the data is currently small is not a bound, and the ceiling exists for
the same reason. @status:spec/plan

@fact:rej-edges-in-the-answer **Rejected: returning edges when a traversal
predicate is present.** It would make the result type depend on the query, so
every consumer would carry two readers and the renderers would fork. The hop
count carries what an edge would have carried, at no cost to the shape. @status:spec/plan
