# PROP-044: Change-native formats — surviving perpetual evolution {#root}

<status stage="spec" state="work" comment="commissioned by the owner 2026-08-09 (mandate quoted in §1); synthesised from the schema-evolution discovery corpus, this session's tree measurements, and a separately convened Fable verdict; awaiting the owner's ratification — the TZ that builds it gates on his word"/>

@fact:PURPOSE **What this document is.** The standing ideology for every durable
data format this project reads or writes — package format, manifests, catalog,
lockfile, configs, CLI JSON, MCP tool schemas. It is written to be handed to an
AI agent as context: it states the laws, the machinery they require, the policy
for each format, and the machine gates that make the laws enforceable against
authors weaker than their reviewers. The build plan that first implements it is
`campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md` (a plan, and
therefore disposable; this document is the contract). @status:spec/plan

## 1. The environment, in the owner's words {#mandate}

@fact:MANDATE-VERBATIM **The owner's mandate (2026-08-09, verbatim, the frame
every rule below serves):** «Совершенно все пакеты будут в ближайшем будущем
опубликованы наружу на миллионы людей. И эволюция будет максимально быстрая —
мы ещё 10 раз сменим формат и манифеста, и пакетов, и чего угодно. И да, у
людей всё будет ломаться и им придётся перекачивать. Разрушения БУДУТ — это
неотъемлемая часть идеи move fast and break things. Ты не можешь выпускать по
10 версий продукта в день (реально 10!) и ничего не сломать. Через год вообще
всё будет другое. Нам нужна система, которая переживает такой процесс. Maven,
RPM и так далее — они не про это, они рассчитаны на стабильность. У нас нет и
не будет никогда никакой настоящей стабильности, хотя очень круто иметь хотя бы
ПЕРИОДЫ СТАБИЛЬНОСТИ. Все обязательства типа "никогда не переименовывать" будут
держаться ровно до тех пор, пока мы их не нарушим — а однажды мы их точно
нарушим, ЛЮБОЕ из обязательств. Эта система должна воспринимать рефакторинги
как нормальную часть жизни, а не как редкое катастрофическое событие.» @status:spec/done

@fact:THE-INVERSION **The inversion this forces.** Stability-era ecosystems
minimise the *frequency* of breaking change — they make a break so expensive it
almost never happens (additive-only forever, never rename, five-year windows).
A change-native system minimises the *cost* of each break: detection, migration
and recovery are cheap, routine, and rehearsed. The machinery budget moves from
prevention to recovery. Every technique borrowed from the stability world must
be re-derived under this inversion or discarded; several invert outright. @status:spec/plan

@fact:TWO-CLOCKS **Product clocks and wire clocks are different clocks.** Ten
product releases a day is the product clock. The wire — bytes a foreign parser
reads — changes only by the deliberate acts this document regulates. The whole
point of the machinery below is that a wire change can never again be a *side
effect* of a refactoring: it is always a named, gated, recorded act. That
separation, not slowness, is what makes ten releases a day survivable. @status:spec/plan

## 2. The two laws, and the short list of the forbidden {#laws}

@fact:LAW-NO-LYING **Law 1 — break freely; never lie.** A break that announces
itself — a parse refusal with a recipe, a skipped record with a reason, a
tombstone — is normal life; users re-fetch and continue. Forbidden is the
family of changes where a *wrong answer looks like a right one*: silent
misinterpretation is the single failure mode that re-downloading cannot cure,
because the error propagates into derived state before anyone knows. @status:spec/plan

@fact:LAW-NO-UNRECOVERABLE **Law 2 — delete freely; never make state
unrecoverable.** Every published artifact must be reconstructible, byte for
byte, from the authoritative core (§3). A fact that would be lost by deleting a
derived artifact is a secretly-authoritative fact, and it is promoted into the
journal the moment it is found. This law is what *purchases* the freedom to
break: we may discard any format because nothing of record lives in it. @status:spec/plan

@fact:forbidden-lead From these two laws, the complete list of the absolutely
forbidden — note how short it is, and note what is NOT on it (renaming,
deleting, narrowing, restructuring are all permitted): @status:spec/plan

- @fact:FORBID-IDENTIFIER-REUSE Never reuse an identifier — a field name, a
  vocabulary value, a `name@version` coordinate — with a changed meaning and no
  declaration. Old data would parse successfully and be understood wrongly:
  the one unfixable failure. (For coordinates this is already standing law:
  `spec://org.vibevm.world/qualified-naming/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL#root`.) @status:spec/plan
- @fact:FORBID-BYTE-MUTATION Never change bytes at an existing address —
  including republishing a version with different content. A correct client
  cannot even detect this break; it silently poisons every hash, cache and
  lockfile downstream. @status:spec/plan
- @fact:FORBID-SILENCE Never answer silence for a name that ever existed. Every
  once-valid name resolves to the current thing, a forwarding pointer, or a
  tombstone with a reason. Silence is indistinguishable from network failure;
  it converts a clean break into a riddle, and riddles — not breaks — are what
  actually strand users. @status:spec/plan
- @fact:FORBID-SECRET-TRUTH Never create unrecoverable state: an authoritative
  fact living only in a derived artifact, or an unreproducible build. This
  forfeits the right to delete-and-rebuild, i.e. the foundation of every other
  freedom here. @status:spec/plan
- @fact:FORBID-HANDWRITTEN-WIRE Never hand-write a parser or writer for one of
  our own wire formats inside this tree. Not a goal in itself: it is the
  mechanism by which the first four violations happen unnoticed. @status:spec/plan

## 3. Truth and projection {#truth}

@fact:TRUTH-KERNEL **The authoritative core is small, and everything else is
disposable.** Authoritative: (1) package sources at immutable git commits;
(2) the authored `vibe.toml` inside those commits; (3) the **registry facts
journal** — append-only records of what sources cannot carry: publication,
yank, rename, removal, ownership, security notice; (4) the schemas, hash
recipes and the generator in this repository — the contract *is* source code;
(5) the format registry and break notes — the ledger of what we broke. @status:spec/plan

@fact:DERIVED-IS-DISPOSABLE **Derived and deletable at any moment:** the entire
catalog (every wire type it serves), `vibe.lock`, caches, generated types,
clients, validators and docs, published projections, CLI JSON outputs, all
aggregates (`latest_stable`, counters, sizes), and `content_hash` values —
derived, but stamped with their recipe (§4.7). @status:spec/plan

@fact:MANIFEST-IS-THE-CENTRE **How this squares with "we manage packages that
have manifests": the manifest is the centre of this architecture, not a
casualty of it.** "Disposable worlds" never refers to packages — package
content at a tag is immutable truth, and a lockfile pinned to a
`content_hash` reproduces an install across any number of catalog rebuilds.
What is disposable is every *serving form* of the metadata. The manifest
leads a double life, and the split is the design: as an **authored file in
the package repo** it is part of the authoritative kernel — the contract
between the author and vibe, strict, epoch-marked, migrated by codemod; as
**data consumed at scale** it is never served raw — the indexer (the one
place holding the wide multi-epoch reading window) reads it at the tagged
ref and projects it into the current-epoch catalog and the published
projection. So when the manifest format changes for the tenth time, no
published package is re-authored: its old-epoch manifest stays readable by
one frozen reader in one codebase, and its *entry* is simply re-projected.
The system manages manifest-bearing packages precisely by refusing to let
the hand-written manifest double as the wire format for millions. @status:spec/plan

@fact:MEMBERSHIP-IS-TESTED **Membership in "derived" is machine-tested, not
argued:** delete the artifact, rebuild it from a pinned input snapshot, compare
bytes (`rebuild --check`). A difference means a secret truth was found, and its
home is the journal. This one test closes an entire class of architecture
drift, which is why it is built early, not last. @status:spec/plan

@fact:ONE-ETERNAL-FILE **Exactly one file is eternal: the handshake.** A tiny
document — `{vibe, worlds[], min_client, notice, successor}` — through which a
client of any age learns which worlds currently exist, where they live, what to
do when it understands none of them, and where the handshake itself moved
(`successor` is the in-band forwarding pointer; our transport is raw files in
git, so there are no HTTP redirects to lean on). Its keys never change meaning.
Everything else in the system is disposable precisely because this one thing is
not. This is the system's single eternal vow, and it is priced deliberately:
five keys and a pointer. @status:spec/plan

## 4. The machinery {#machinery}

@fact:MACHINERY-LEAD Compatibility here is a property of machinery around
formats, never of formats themselves. The machinery, each piece multiplying the
others: @status:spec/plan

@fact:M-FORMAT-REGISTRY **4.1 The format registry.** `formats/REGISTRY.toml`
inventories every surface a foreign parser reads: id, epoch, schema path,
recoverable-or-not, independent-parser count, sunset date, golden-corpus path.
From it the `FormatId` enum is generated, and all wire I/O goes through
`wire::publish(FormatId, …)` / `wire::load(FormatId, …)` — an unregistered
format is *inexpressible in the type system*, not merely discouraged. An
unnumbered format is a format that will be broken without anyone noticing. @status:spec/plan

@fact:M-SCHEMA-DESCRIBES-FORM **4.2 Schemas describe form; the generator owns
policy.** One schema per format per epoch (`schemas/<format>/e<N>/…`). The
schema language (JTD today) is chosen for *poverty*, not power: an agent cannot
write a half-tagged union or a conditional schema in it. Everything the
language cannot express — strictness, open vocabularies with preserved unknown
values, skip-empty conventions, canonical ordering, determinism — is emitted by
**our own generator layer**, which is a first-class component of this project.
A third-party generator's expressiveness never decides our policy. Measured
2026-08-09: stock `jtd-codegen 0.4.1` handles tagged unions (`#[serde(tag)]`)
but emits closed vocabularies and `Option<Box<…>>` optionals — exactly the gap
the layer exists to close. @status:spec/plan

@fact:M-OPEN-ENUM-FROM-CLOSED **4.2a The vocabulary resolution.** From a
*closed* schema enum the generator emits an *open* Rust type:
`enum { …known…, Unknown(String) }`. Compiler-checked exhaustiveness survives
(the `Unknown` arm is mandatory), the original string is preserved through any
rewrite, and tolerance of future values is structural. The trilemma the
discovery corpus recorded — tagging vs fallback values vs codegen, mutually
incompatible — dissolves because the decision moved from the schema language
into the generator. @status:spec/plan

@fact:M-CANONICAL-BYTES **4.3 Canonical bytes, deterministic writers.** One
state — one byte sequence: sorted keys, injected clocks (a writer never calls
`now()`; time arrives as input), pinned encodings, deterministic compression.
Determinism is not cosmetics; it is the *measuring instrument*: it makes
"rebuild and compare" a real verification, an empty diff a real no-op, and a
wire-diff a quantitative measure of any break. @status:spec/plan

@fact:M-JOURNAL-NOT-RMW **4.4 Journal and projection — no read-modify-write.**
Mutations of derived artifacts are appends to the journal followed by
reprojection; a writer never accepts as input a value parsed from its own
published output (machine-enforced at module boundaries). This dissolves the
worst hazard the discovery found — "tolerant reading without capture silently
deletes foreign fields on the next rewrite" — not by adding capture machinery
but by removing the rewrite: there is nothing to preserve because nothing read
is ever written back. Reader strictness can then be dropped for free. @status:spec/plan

@fact:M-MUST-UNDERSTAND **4.5 must-understand and record quarantine.** Each
record names the capabilities a reader must have to act on it. A reader lacking
them quarantines *that record* — the file loads, the server starts, and the
refusal surfaces at the point of use with a generated recipe ("unavailable
because X; run Y"). This is the exact inversion of additive-only: there the
*schema* promises ignorability forever; here the *writer* declares it per
record, addressably and revocably. Unknown fields outside must-understand are
ignored; unknown records are quarantined, never dropped silently. @status:spec/plan

@fact:M-EPOCHS **4.6 Epochs are parallel worlds, not mutations.** A heavy break
mints a new epoch at a new path (`/e4/…`); the old world keeps publishing until
its announced sunset, then freezes — free of charge, because the transport is
git. Worlds are physically unable to poison each other. Within an epoch,
obligations hold; epoch boundaries are where "we will one day break any vow"
happens in a controlled, rehearsed, announced way. Light breaks (a new
capability under must-understand) need no epoch at all. @status:spec/plan

@fact:M-BREAK-WINDOW **4.7 Breaking is a switched resource; recipes are data.**
`formats/EPOCHS.toml` carries `break_window_open`; while it is closed, CI
refuses any change under `schemas/**` or `formats/**` — a *period of stability*
is the state of a flag, visible in the git log of one file, not a virtue or a
hope. Any wire-visible change requires a break note
(`formats/breaks/NNN.md`: what · epoch · who fixes · sunset · user recipe), and
the golden-corpus wire-diff must be empty or covered by one — the gate does not
forbid breaks, it makes an *unannounced* break impossible. Every derived value
carries its recipe id (`content_hash` rides as `sha256-tree/1:…`, the exclusion
list and normalisation live in `hash_recipes/1.toml` as data, each recipe
frozen by a golden test) — changing a computation becomes a visible new recipe,
never a silent change of every value in the world. @status:spec/plan

## 5. Placement and computed policy {#placement}

@fact:PLACEMENT-LADDER **The placement ladder.** Cost of change grows
monotonically: config < lockfile < index export files < `by-name` projection <
journal < `vibe.toml` < identity/handshake. The one placement rule a weak
author can apply without understanding the system: **put every new fact on the
lowest rung that bears it.** A volatile fact in the manifest is a decade-long
debt; the same fact in the lockfile is free. @status:spec/plan

@fact:POLICY-IS-COMPUTED **A format's policy is computed, not chosen**, from
two axes: is it recoverable without a human, and how many independent parsers
exist. Recoverable + one parser → hard gate + silent rebuild (lockfile,
caches). Recoverable + many parsers → epochs, parallel publication, generated
clients, narrow projection (catalog, CLI JSON). Unrecoverable + one parser →
epoch-in-file + codemod (configs). Unrecoverable + many parsers is the worst
quadrant — minimise it, epoch it, migrate it by bot (`vibe.toml` is the only
resident, and creating a *new* format in this quadrant requires an explicit
owner decision recorded in the format registry). @status:spec/plan

@fact:MIGRATION-TAXONOMY **Migrations come in three kinds, and two must not
exist.** Derived data has *no migrations ever* — delete and rebuild; writing an
index migrator is an architecture error, because it asserts the index is
authoritative. Authored data gets *codemods*: idempotent, comment-preserving
(document-edit in place, never serialise-from-struct), rehearsed on a corpus of
real files, applied locally by `vibe migrate` and remotely by bot pull request
— the one scalable way to migrate data you do not own, and a capability a
git-organisation registry has that Maven Central structurally lacks. Runtime
compatibility shims are forbidden; the only exception is frozen per-epoch
*readers* of old manifests inside the indexer — readers, not shims: they branch
no logic and are never edited after freezing. @status:spec/plan

## 6. The formats of this project, by name {#our-formats}

@fact:FMT-CATALOG **6.1 The catalog is three things with three fates.** The
read-for-rewrite core (`by-name/`, `repomd.json`) stops being read at all —
server mutations become journal appends, `add`/`remove`/`reindex` become
reprojections; after that, reader strictness drops for free, `generated_at`
comes from the journal (event time, not process clocks), the catalog becomes
deterministic and rebuild-testable. The export files (`primary.jsonl(.gz)`,
`by-cap/`, `by-purl/`) are strict canonical exports, and every published
format must have a *generated reader used in a round-trip test* — publishing
what we cannot read back is how write-only formats drift. The narrow public
gate — name, version, hash+recipe, URL, yanked, tombstone — is a new, small,
slow-moving artifact: the one surface we actually defend before foreign tools,
so that everything richer behind it may churn weekly. The catalog repository
itself is *declared disposable*: pin content by hash, not by commit — a
permission we can actually keep, instead of a promise we cannot. @status:spec/plan

@fact:FMT-CATALOG-VERSION **The catalog's version stops being a number nobody
compares.** Its heavy role moves into the *path* (the epoch); its light role
into per-record must-understand. The old question "is the version a gate or a
declaration?" dissolved because it sought both answers inside one `u32`. @status:spec/plan

@fact:FMT-MANIFEST **6.2 `vibe.toml` is the most expensive format in the
system** — authored, unrecoverable, resident in foreign repositories, and an
external surface *de facto* because agents read files, not APIs. It is the one
place we honestly pay the stability tax: an **epoch marker in the file,
introduced before first publication** (absence ≠ "assume 1"; absence is the
distinct pre-epoch state, interpreted by a heuristic exactly once in history);
strictness with did-you-mean hints in our namespace (in a hand-written file an
unknown key is a typo and the author is present — the asymmetry with the
machine-written catalog is principled, not inconsistent); a named reserve
section we never touch; a **generated projection as the published copy**, so
foreign tools read a recoverable artifact while the hand-written original
remains our internal concern; migration by codemod + bot PR; and a minimal
surface — every new manifest key is a years-long obligation and requires a
break note, a codemod, and generated documentation. @status:spec/plan

@fact:FMT-LOCKFILE **6.3 `vibe.lock` is the cheapest format and becomes free by
construction.** "Schema version 5, reject on `!=`" is replaced by: the lockfile
is valid only for the exact (epoch, generator hash, recipe ids) that built it;
any mismatch → silently regenerate; `--frozen` turns that into a loud error for
CI. Deterministic by the same rules as the catalog. Consequence for design:
since this format's evolution is free, volatile facts belong here — the bottom
rung of the ladder. @status:spec/plan

@fact:FMT-CONFIGS **6.4 Configs are preferences, not data — failure is
structurally impossible.** `fn load() -> Config` with no `Result`: unknown key
→ warning; invalid value → default plus warning; the file is rewritten to
canonical form opportunistically. Blocking a build over a stale setting is the
worst possible trade at ten releases a day. Strict failure belongs where wrong
reading corrupts results; graceful degradation where it only changes comfort. @status:spec/plan

@fact:FMT-UNINVENTORIED **6.5 Surfaces nobody inventoried are formats too.**
The seven CLI `--json` reports (scripts and agents parse them), MCP tool
schemas (literally a contract with foreign agents), skill/subskill files, the
journal, the handshake — each is "something a foreign parser reads", each gets
a registry entry, a schema, an epoch and a corpus. @status:spec/plan

## 7. Obligations have types; stability is a mechanism {#obligations}

@fact:OBLIGATION-TYPES **Three types of obligation, and only two eternal ones.**
*Eternal* (both about honesty, not content): we never present a wrong answer as
right; we never answer silence for a name that existed. Both are keepable
forever because they forbid no change. *Expiring*: epoch support windows — they
do not get broken, they *expire on schedule*. *Revocable*: everything else —
names, fields, structures, vocabularies, paths. The owner's prediction «любое
обязательство однажды нарушим» is satisfied by construction: almost nothing
here is vowed. @status:spec/plan

@fact:OBLIGATION-PROCEDURE **An obligation without a written breaking procedure
is a hope, not an obligation.** Every obligation ships with its procedure
(announce → transform/reproject → publish in parallel → sunset → leave a
tombstone) and the procedure is *rehearsed*: the release pipeline replays a
cold upgrade from the oldest supported epoch, and a staged sunset is drilled
periodically. Early breaking is the same procedure compressed, plus a recorded
owner reason — friction is asymmetric by design: extending a sunset is free,
shortening one takes an explicit owner act. @status:spec/plan

@fact:STABILITY-IS-A-FLAG **A period of stability is a closed break window plus
a published expiry.** The honest move-fast contract with millions of users is
not "we won't break you" but **"you always know the date you break, and the fix
is one command."** The date travels inside the world it retires; clients warn
ahead of it; on the day, the refusal carries the recipe. Heavy breaks batch
into trains — parallel publication and support windows are fixed costs per
train, not per change. @status:spec/plan

## 8. Laws for AI agents — gates, not admonitions {#agents}

@fact:AGENT-PROSE-IS-BROKEN **Governing rule: a policy that exists only in
prose is already broken.** Every rule below lives at one of three levels,
preferring the highest: L1 — inexpressible in types (the agent physically
cannot write it); L2 — inexpressible in the schema language; L3 — refused by
CI. An agent's freedom is confined to three directories: schemas, codemods, and
application code; everything else is generated and drift-gated. @status:spec/plan

@fact:AGENT-GATES **The gate set.** G1: all wire I/O through
`wire::publish/load(FormatId,…)`; unregistered formats untypeable (L1). G2:
serde derives forbidden outside `**/generated/**` (L3). G3: regenerate +
`git diff --exit-code` (L3). G4: a writer must not accept a wire-parsed type as
input (L1). G5: double-build under different clocks/host/locale/paths must be
byte-identical; `now()` banned in writer modules (L1+L3). G6:
`rebuild --check` — tear down and reproject from a pinned input (L3). G7:
schema/format changes demand a break note; golden wire-diff empty or covered
(L3). G8: schema changes refused while the break window is closed (L3). G9: a
vocabulary exists in exactly one schema; both wire sides, Rust types, docs and
prose lists are generated from it (L1+L3). G10: every derived value carries a
recipe id; every recipe has a frozen golden (L3). G11: every published format
has a generated reader exercised in round-trip tests (L3). G12: clients must
read the corpora of supported epochs; on sunset the same test flips to
asserting the refusal text and recipe (L3). G13: schema/format edits never
share a commit with application code (L3). G14: config load returns no
`Result` (L1). G15: manifest codemods edit the TOML document in place;
serialising from a struct is refused (L3). @status:spec/plan

@fact:AGENT-MESSAGES **A gate's message is the only documentation that is
reliably read.** Every failure states what was violated, why the rule exists
(one sentence), and the exact next command — generated from the registry and
break notes, never hand-written. And the agent is left no decisions it gets
wrong: strict-or-tolerant, optional-or-required, open-or-closed, field-or-epoch
each have exactly one machine answer — policy from the generator, placement
from the ladder, epoch from the wire-diff. The agent brings *intent*; the
schema fixes form; the generator fixes policy. @status:spec/plan

## 9. Honest risks and revisit triggers {#risks}

@fact:RISK-RECOVERABILITY **The recoverability bet.** Upstream sources vanish —
deleted repositories, force-pushes, privatisation. If rebuild-from-truth stops
being possible and a content-addressed source archive is not built, the index
silently becomes authoritative and the whole system degrades into a
compatibility regime without its detector. *Trigger:* first observed
unreachable source that the journal references → the archive subsystem becomes
a mandate, not an idea. This is the likeliest way this document is wrong. @status:spec/plan

@fact:RISK-RMW-VOLUME **The journal/projection rework is a real rebuild, not an
edit.** Its cost gate is measured before commitment (the TZ's phase 0 measures
the three server mutation paths); if it exceeds the budget, the documented
fallback is temporary unknown-capture on the three read-modify-write types —
named here explicitly as the compromise it is, so it cannot masquerade as the
design. @status:spec/plan

@fact:RISK-RAW-PARSERS **Foreign `curl | jq` will happen regardless of
generated clients**, and there will never be a signal of it (raw files in git
give no telemetry, by design). The honest posture: assume raw parsing exists,
keep the narrow gate genuinely boring, and treat any change to it at the
highest severity class. @status:spec/plan

@fact:RISK-OVERENGINEERING **The degenerate-first guard.** The epoch machinery
must exist, but its first implementation may be degenerate — one live world and
`min_client` in the handshake; parallel publication is built when the second
world is actually minted. A gate heavier than the system it guards gets walked
around, and a walked-around gate is worse than none: it manufactures false
confidence. *Trigger for scaling up:* the first real epoch train, and no
earlier. @status:spec/plan

@fact:RISK-SECURITY-PARKED **Signature and authenticity remain parked by the
owner's standing ruling (`BACKLOG.md` B-015): mechanism slots only, no
cryptography.** What this document adds is ordering, not scope: the
must-understand set and the signature *slot* exist in the schema before any
tolerance ships, because a tolerant reader that can silently ignore a future
signature field is a downgrade attack waiting; slots are irreversible-window
work, cryptography is not. @status:spec/plan
