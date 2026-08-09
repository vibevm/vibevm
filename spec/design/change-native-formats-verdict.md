# Change-native formats — the convened verdict {#root}

<status stage="spec" state="done" comment="design lore for PROP-044: the architectural verdict of a separately convened Fable council (2026-08-09), recorded near-verbatim the same day so the reasoning survives the session that produced it; the council read 10-MEGA-REPORT.md and the key sections of 11-fable-review-and-verdict.md before answering, and deliberately received none of the commissioning session's own hypotheses"/>

@fact:what-this-is **What this document is.** The owner reframed the
schema-evolution question on 2026-08-09 — the studied ecosystems are built for
stability; this system lives in permanent break — and proposed convening a
separate Fable instance for an independent verdict. This is that verdict,
recorded as design lore: it explains *why* PROP-044 says what it says, with the
rejected alternatives. The contract it explains is
[`spec/common/PROP-044-change-native-formats.md`](../common/PROP-044-change-native-formats.md);
the research it stands on is
[`spec/research/schema-evolution-2026-08/`](../research/schema-evolution-2026-08/).
Nothing here is normative — where this text and PROP-044 disagree, PROP-044
wins and this file gets corrected. @status:spec/done

## 0. The essence in one paragraph {#essence}

@fact:the-essence **Stable ecosystems pay for compatibility *inside the schema*** —
an eternal tax on every future design decision («only add», «never rename»),
and that tax is exactly what makes them unable to move fast. We cannot pay in
the schema, and we do not need to: our published data **is not truth** — it is
derived from package sources in git and from a small journal of registry
facts. So the right system is one where **the whole published world is
disposable and byte-recoverable**, and the only eternal thing is one tiny
handshake file through which a client of any age learns which worlds exist,
where they live, what to do when it understands none of them, and where the
handshake itself moved. Compatibility stops being a property of the format and
becomes a property of the **machinery around the format**: determinism (so
«rebuild and compare» is a real check), a generator that owns all policy (so an
agent cannot hand-write the wire), record quarantine instead of file refusal, a
writer-declared must-understand set, epochs as parallel worlds instead of
schema mutation, and the break window as a switched resource. Breaking
anything is allowed; forbidden is exactly one family of acts — those where **a
wrong answer looks like a right one**. The formula: **ломать можно, врать
нельзя; удалить мир можно, сделать мир невосстановимым нельзя.** @status:spec/done

## 1. Four architectures, compared as wholes {#architectures}

@fact:method **Each candidate was built as a coherent system** — its own source
of truth, its own client-survival story, its own machinery, its own way of
dying — and compared on *integration*: whether the parts multiply each other
or merely coexist. @status:spec/done

### 1.1 Architecture A — «Disposable worlds» (chosen) {#arch-a}

@fact:a-principle **Organising principle:** truth lives outside published
artifacts; everything published is a projection that can be deleted and
rebuilt byte-for-byte. A format lives a week, a format generation (epoch) a
quarter; only the handshake is eternal. @status:spec/done

@fact:a-how **How it works.** Truth = (package sources at immutable git
commits) ∪ (the registry facts journal: publication, yank, rename, removal,
ownership). The index is a pure function of truth and generator version. One
schema per format per epoch; types, readers, writers, validators, corpora and
docs are generated; policy (tolerance, canonicalisation, vocabulary openness,
strictness) lives in the generator, not in the schema and not in review. A
reader asks three questions: do I know this format and epoch (else — file-level
refusal with a recipe); does this record demand capabilities I lack (else —
**record quarantine**); everything else unknown — ignore. Breaks come in two
speeds: light (a new capability declared in must-understand; old readers lose
individual records and know why) and heavy (a new epoch at a new path `/e4/`;
the old world keeps publishing until its announced date, then freezes). @status:spec/done

@fact:a-integration **Integration — the parts multiply:** determinism → the
teardown-rebuild test is possible → it is *proven* that no truth hides in the
derived → the derived may be deleted → **breaking without migrations is
legal**; no read-modify-write → tolerance of unknown fields becomes free and
safe → the whole capture apparatus becomes unnecessary; the generator owns
policy → a cross-cutting wire policy change is one commit → **a cross-cutting
change stops being a project**; must-understand + quarantine → break frequency
can be high because each break's radius is one record; epoch-in-path → worlds
physically cannot poison each other → parallel publication costs disk, not
risk. @status:spec/done

@fact:a-death **How it dies.** If even one fact turns out to be authoritative
only in a derived artifact (a counter, a flag, a hand-edit of the index), the
rebuild test starts failing, someone disables it, and the system silently
degrades into Architecture C without C's protections. The second death: epochs
minted too often, until parallel publication and support windows eat the
gains. @status:spec/done

### 1.2 Architecture B — «Immutable herbarium» (rejected) {#arch-b}

@fact:b-principle **Principle:** nothing published is ever rewritten;
evolution is the *multiplication of objects*, not the changing of formats.
Everything is content-addressed; names are a thin mutable pointer layer over
immutable objects (the Nix / Unison / OCI family). A new format = a new object
kind; no migrations; old objects live and parse forever. @status:spec/done

@fact:b-rejection **Why rejected as a whole.** Very high integration inside
its idea, very narrow coverage: it solves 10 % of a package manager's job
perfectly and 90 % not at all. «Which versions exist», «what is latest
stable», «who provides capability X» are aggregates — mutable by nature, so
they have a format, and it breaks; the herbarium pushes the whole problem into
the mutable pointer layer and pretends it is not there. Worse, the client must
now understand an unboundedly growing zoo of object kinds, deciding at every
call site what to do with an unknown kind — the same problem, smeared across
places where no machine gate can check it. Exactly wrong for a system whose
authors are weak agents. @status:spec/done

@fact:b-taken **What was taken from it, as an absolute law:** bytes at an
existing address are immutable; new content is a new address. In A this does
not conflict with delete-and-rebuild precisely because of determinism: a
rebuild from the same inputs yields the same bytes, so to a reader a rebuild
is indistinguishable from no change. @status:spec/done

### 1.3 Architecture C — «The eternal line under machine supervision» (rejected) {#arch-c}

@fact:c-principle **Principle:** one format lives forever; compatibility is
not promised but *machine-enforced*: schema registry, field numbers forever,
reserved ranges, a breaking-change detector in CI (the Protobuf + Buf +
Confluent model). Exemplary internal integration — and the only candidate that
makes a weak agent structurally safe *today* at low cost. @status:spec/done

@fact:c-rejection **Why rejected.** (1) It contradicts the mandate directly:
compatibility becomes an eternal design tax; after ten breaks the format is a
graveyard of `name_v2`, `features2`, `kind_old`, and the *conceptual* model
rots even while the bytes stay compatible. (2) Three steps ahead the tax lands
on the agents themselves: to write correct code a weak agent must understand
five generations of vestiges and know which field is «real» — a system bought
to protect against weak authors ends up making weak authors more dangerous.
Self-undermining. (3) Its main mechanism is blind to the main danger: a
byte-compatibility detector passes *semantic* breaks (a field's meaning
changes, its shape does not) — and this tree's deadliest spots are exactly
semantic: `content_hash` with a hard-coded exclusion list, `latest_stable` as
a computation, `BindingSite` written twice under different serialisation
rules. @status:spec/done

@fact:c-taken **What was taken, with the verdict inverted.** The detector
itself is mandatory — but where Buf's verdict is «PR rejected», ours is «PR
requires a break note, an epoch decision and a sunset-calendar entry». The
same machine, the opposite sentence: it does not forbid a break, it makes an
*undeclared* break impossible. @status:spec/done

### 1.4 Architecture D — «Format as program» (rejected briefly) {#arch-d}

@fact:d-rejection **Data carries (or names) its own reader** — an artifact
pins an exact reader version/hash, the client pulls a WASM module and can read
anything of any age. In theory it dissolves everything; in practice it is an
eternal obligation to execute a decade of foreign code **on the package
manager** — the most sensitive link of the supply chain — plus the loss of all
static checkability. For a system whose code is written by agents,
«dynamically load a parser by a pointer from the data» is the worst available
answer. Its one sound grain — the client must be able to *update itself* on
instruction from the data — was taken into A as `min_client` in the handshake. @status:spec/done

### 1.5 The comparison that decided it {#comparison}

@fact:comparison-table **Behaviour under ten breaks a week:** A — routine (a
light break is free; heavy breaks batch into a train); B — decay (the client
must know ever more object kinds); C — ossification (ten breaks a week is what
its machine exists to forbid); D — not priced in formats at all. A's only
structural seam — «delete and rebuild» vs «bytes are immutable» — is sewn by
determinism. @status:spec/done

@fact:why-a-is-possible **The reason A is possible for us and was not for
Maven or RPM:** their published metadata IS authoritative and unrecoverable;
ours is not. That is not a stylistic difference but different physics — the
whole stability discipline of the neighbours is a *consequence* of publication
being truth. We do not inherit the consequence because we do not inherit the
cause. The one place we do inherit the cause — hand-written `vibe.toml` in
millions of foreign repositories — is the one place we honestly pay the
stability tax (PROP-044 §6.2). @status:spec/done

## 2. What the verdict dissolves from the earlier analysis {#dissolves}

@fact:dissolves-capture **The discovery's most valuable finding — «tolerance
without capture is silent deletion on six rewrite paths» — is true and lethal
*inside the read-for-rewrite frame*, and it diagnoses a symptom.** The real
defect is read-modify-write of an authoritative projection itself. Remove it
(journal + projection) and: capture (`flatten extra`) is unnecessary — nothing
read is ever written back; the serde incompatibility of `deny_unknown_fields`
with `flatten`, which forced per-type choices, disappears; dropping reader
strictness becomes free and safe *immediately*; and the ban on
`Unknown(String)` for `PackageKind` («one `add` and the truth is destroyed»)
falls together with the `add` that rewrote. A machine gate then makes the
return impossible: **a writer may not accept as input a type obtained by
parsing the wire** — checked at module and type boundaries, not by eyes. @status:spec/done

@fact:dissolves-version-question **The «is the version a gate or a
declaration?» question dissolved** because it sought both answers inside one
`u32`. The heavy role moves into the *path* (the epoch); the light role into
per-record must-understand. The measured absurdity it replaces: a version
stamped on every file and record, compared nowhere, and overwritten with the
writer's own constant on every write — a version describing «who last touched
this», not the data. @status:spec/done

@fact:dissolves-vocab-trilemma **The vocabulary trilemma** («tagging +
fallback values + codegen are mutually incompatible») dissolved by moving the
decision into the generator: from a *closed* schema enum the generator emits
an *open* Rust type `enum { …known…, Unknown(String) }` — exhaustiveness
survives as a mandatory match arm, the original string survives rewrites,
tolerance of future values is structural. Neither exhaustiveness was
surrendered nor freedom lost; the choice just stopped living in the schema
language. @status:spec/done

## 3. Answers the mandate demanded {#mandate-answers}

@fact:stability-mechanism **«A period of stability» is a mechanism, not a
virtue:** `break_window_open = false` in `formats/EPOCHS.toml`, under which CI
refuses any change beneath `schemas/**` and `formats/**`. «A month with
nothing broken» means nobody opened the window that month — a state visible in
one file's git log, not in feelings. Second layer: **trains** — heavy breaks
accumulate and ship as a batch, because parallel publication and support
windows are fixed costs per train, not per change. Third: **a published expiry
date** riding inside the world it retires; the honest move-fast contract is
not «we won't break you» but «you always know the date you break, and the fix
is one command» — planned maintenance being the only form millions of users
can actually digest. @status:spec/done

@fact:obligations-typed **Obligations are typed, and the types are three.**
Eternal — two, both about honesty, not content: never present a wrong answer
as right; never answer silence for a name that existed (even the handshake's
eternity is bounded by its own `successor` field — we vow to *answer*, not to
answer the same thing). Expiring — epoch support windows: they are not broken,
they expire on schedule. Revocable — everything else. And the central rule for
the mandate: **an obligation without a written breaking procedure is a hope,
not an obligation** — each ships with its procedure (announce → transform →
publish in parallel → sunset → tombstone), the procedure is *rehearsed* (a
cold-upgrade replay from the oldest supported epoch in the release pipeline; a
staged sunset drill), and friction is asymmetric: extending a sunset is free,
shortening one is an explicit owner act with a recorded reason. @status:spec/done

@fact:blast-radius **Blast-radius containment, eight mechanisms:** layers with
different recoverability never share code (share the schema, never the code —
DRY across a trust/lifetime boundary is a coupling bomb; it is precisely what
produced `BindingSite` written twice under different rules, agreeing today by
the accident of single-word values); epoch-in-path isolates worlds physically;
record quarantine instead of file refusal, failing at the point of use, not
load; must-understand puts the fatality decision on the writer, per record;
a narrow public projection gives most internal breaks an external radius of
zero; byte-immutability at addresses means a bad publication can be superseded
but cannot retro-poison caches; asymmetric reading windows (wide only in the
indexer — one codebase we own; narrow in clients) concentrate the
old-format-reading tax in one place; and the placement ladder (config <
lockfile < exports < projection < journal < manifest < identity/handshake)
prices every new fact by the lowest rung that bears it. @status:spec/done

@fact:forbidden-derivation **Why exactly the forbidden five are forbidden:**
identifier reuse with changed meaning is the only failure re-downloading
cannot cure (old data parses successfully and is understood wrongly; the error
propagates silently into derived state); byte mutation at an address is the
only break a *correct* client cannot even detect; silence turns a clean break
into a riddle, and riddles — not breaks — are what strand users; unrecoverable
state forfeits delete-and-rebuild, the foundation of every freedom here; and a
hand-written parser is the mechanism by which the first four are committed
unnoticed. Everything else — renames, deletions, narrowings, restructurings —
is legal, which is the whole point. @status:spec/done

## 4. Per-format placement — the computed grid {#grid}

@fact:the-computed-grid **Policy is computed from two axes** — recoverable without a human?
how many independent parsers? — giving: recoverable × one parser → hard gate +
silent rebuild (`vibe.lock`, caches); recoverable × many parsers → epochs,
parallel publication, generated clients, narrow projection (catalog, CLI
JSON); unrecoverable × one parser → epoch-in-file + codemod (configs);
unrecoverable × many parsers → **the worst quadrant: `vibe.toml`** — minimise
it, epoch it, bot-migrate it, and never create a new format in this quadrant
without an explicit owner decision recorded in the format registry. @status:spec/done

@fact:catalog-split **The catalog is three things with three fates:** the
read-for-rewrite core (`by-name/`, `repomd.json`) stops being read at all;
the export files (`primary.jsonl(.gz)`, `by-cap/`, `by-purl/`) become strict
canonical exports with a mandatory *generated reader in round-trip tests*
(publishing what we cannot read back is how write-only formats drift — their
zero-caller `primary::read`/`parse` today is the measured warning); and a
narrow public gate — name, version, hash+recipe, URL, yanked, tombstone — is
the one surface actually defended before foreign tools, so everything richer
behind it may churn weekly. The catalog repository itself is *declared
disposable* — pin content by hash, not commits: a permission we can keep
instead of a promise we cannot. @status:spec/done

@fact:manifest-tax **`vibe.toml` — the honest stability tax, paid in one
place:** an epoch marker in the file before first publication (its absence is
the *pre-epoch state*, not «epoch 1» — the distinction between «old» and
«foreign/broken» must not be erased); strictness with did-you-mean hints in
our namespace (in a hand-written file an unknown key is a typo and the author
is present — the principled asymmetry with the machine-written catalog); a
reserve section we never touch; the published copy generated, not hand-written
(the same authored/published split the stability corpus reached, re-derived
from recoverability instead of from politeness); codemod + bot-PR migration
with frozen per-epoch readers only in the indexer; and a minimal surface —
every new manifest key is a years-long obligation. @status:spec/done

@fact:lockfile-free **`vibe.lock` becomes free by construction:** valid only
for the exact (epoch, generator hash, recipe ids) that built it; any mismatch
→ silent regeneration; `--frozen` makes that a loud error in CI. Deterministic
under the same rules as the catalog. Consequence: volatile facts belong here —
the bottom rung — which is itself a design tool. @status:spec/done

@fact:configs-never-fail **Configs are preferences, not data:** loading cannot
fail by type (`fn load() -> Config`); unknown keys warn, invalid values
default-and-warn. Strict failure belongs where wrong reading corrupts
results; blocking work over a stale setting is the worst trade at ten releases
a day. @status:spec/done

@fact:uninventoried **Surfaces nobody had inventoried are formats too** — the
seven CLI `--json` reports, the MCP tool schemas (literally a contract with
foreign agents), skill/subskill files, the journal, the handshake — each gets
a registry entry, schema, epoch and corpus. An unnumbered format is a format
that will be broken without anyone noticing. @status:spec/done

## 5. Ordering: by irreversibility, not by value {#ordering}

@fact:build-order **What is built first is what cannot be done after
publication** — publication into git is irreversible and there will never be a
signal that the first reader arrived. Wave 0 (days): the format registry;
the epoch envelope in every artifact including `vibe.toml`; recipe identity
for `content_hash`; the must-understand and signature slots; yank and
tombstone. Wave 1 — what makes the freedom to break legal: determinism,
killing read-modify-write, the rebuild test. Wave 2 — the machinery against
the agent: the generator with policy, the symmetric tag on the union, the
quarantine loader, corpora + wire-diff + the break window. Wave 3 — the
external contract: narrow projection, generated clients, sunset calendar and
rehearsals, codemods and the bot. The convergence check: this wave-0 list
nearly coincides with the «four irreversibles» the stability-framed review
had independently produced — the same conclusions from opposite frames being
the strongest available signal they are right. @status:spec/done

@fact:do-not **The named do-nots:** do not describe the catalog in schemas
before read-modify-write is dead (that would freeze into schema a shape born
of a defect); do not introduce unknown-capture — it treats the symptom and
makes returning to RMW comfortable; do not build parallel epoch publication
before the second world is actually minted — the first implementation of the
epoch machinery may be degenerate (one live world, `min_client` in the
handshake), because a gate heavier than the system it guards gets walked
around, and a walked-around gate manufactures false confidence. @status:spec/done

## 6. The verdict's own honest wrongness budget {#wrongness}

@fact:wrong-recoverability **Most likely wrong:** the recoverability bet.
Upstream sources vanish — deleted repos, force-pushes, privatisation — and
without a content-addressed source archive the rebuild test starts to bind,
gets disabled, and the system silently degrades into Architecture C minus its
detector. (Mitigation noted at plan level: the `--from-clones` vendor-mirror
tree the indexer already supports is the natural seed of that archive.) @status:spec/done

@fact:wrong-rmw-cost **Second: the journal/projection rework is a real
rebuild**, and if it exceeds its budget the pragmatic fallback is temporary
capture — the very thing the verdict warns against — so the volume is measured
*before* wave 1 is committed, not after. @status:spec/done

@fact:wrong-raw-parsers **Third: «own the consumers' parsers» may not work** —
people will `curl | jq` because it is two characters, the real contract will
be the raw bytes, and there will never be a signal of it. Honest posture:
assume raw parsing exists, keep the narrow gate genuinely boring, and treat
any change to it at the highest severity. @status:spec/done

@fact:wrong-vows **Fourth: the one eternal file is still a vow**, reduced but
not eliminated by `successor`; and identity immutability
(`group:name:version` + content hash) was treated as untouchable — it is
existing project law, but a future identity-scheme break is an order of
magnitude costlier than any format break and this verdict does not design it.
Fifth: over-engineering — at thousands of packages the cheap path is «break
freely, one gate: `min_client`»; wave 0 is unconditional, waves 2–3 may wait. @status:spec/done
