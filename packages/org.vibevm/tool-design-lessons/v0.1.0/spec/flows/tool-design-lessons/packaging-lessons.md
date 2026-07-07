# Packaging — lessons P1–P7 {#root}

**Scope of this document.** Seven lessons from turning a discipline into
a distributable package: what a package *is*, what ships inside it, what
establishes its identity, how it builds without polluting the consumer,
and when to generalise its machinery. Each lesson is self-contained —
the failure that taught it, the law in one bolded line, the mechanics,
and a "symptoms you need this" line. Vocabulary is generic — *the
package*, *the consumer*, *the slot* — because the laws port even where
the build system does not.

## P1 — a package is a project {#package-is-project}

**Context.** If a package has its own bespoke layout, an author must
learn a package-only directory convention, and the tooling that
validates an ordinary project cannot be pointed at a package. Two
shapes means two of everything — two linters, two mental models, two
ways to be wrong.

**The law.** *A package is a project — the distributable unit is
structurally identical to a working project, with no package-only
convention to learn.*

**Mechanics.** Prompt and spec content lives under a `spec/` subtree
laid out exactly as an ordinary project's; arbitrary code lives at the
root (a build manifest plus source), exactly as a project root holds
it; one manifest file names the package. Then authoring a package *is*
authoring a project — the same check command applies unchanged, the
same boot computation runs, the same layout is validated by the same
rules. Code is optional: a prompt-only package simply has no code at
its root. The distributable and the working project are one object,
one made installable.

**Symptoms you need this.** A package-only folder convention nobody
remembers; a project linter that refuses to run against a package;
documentation that exists only to explain how packages differ from
projects.

## P2 — ship the runtime, not a description of it {#ship-runtime}

**Context.** The discipline's tooling was hardcoded inside one
workspace. Installing the discipline package gave a consumer a
*description* of checkers they did not have — to actually run the
discipline they would have to re-implement the very tools already
written. The practice was documented but not distributable.

**The law.** *Ship the runtime, not a description of it — a practice's
tooling travels inside the package.*

**Mechanics.** If a package documents a checker, a linter, a generator,
or a build step, the package carries the **executable** form of it, not
only the prose. "Install X and you get a description of a tool you do
not have" is the exact failure to design out. A consumer who installs
the package holds the working tool the same moment they hold the words
that describe it — the strong-author artifacts (the guide, the cards)
and the runtime (the checkers) ship together or the practice is not
really distributable.

**Symptoms you need this.** "Install this, then go build the tool it
describes"; a practice only its author can actually run; a gap between
what the doc promises and what the install delivers.

## P3 — identity is the source, excluded by denylist {#identity-is-source}

**Context.** Hashing or copying a package's whole directory pulls in
build output — non-deterministic (timestamps, host paths, incremental
state) and potentially gigabytes. That makes identity unstable and
materialisation ruinous: the same source yields a different hash on
every build, and the copy drags tens of GB of artifacts.

**The law.** *Identity is the source, never build artifacts — exclude
them by a denylist of never-source directories, not a per-file
allow-list.*

**Mechanics.** A short denylist names what was **never** source — VCS
internals, caches, build output (for example `.git/`, `target/`,
`node_modules/`) plus an optional package-level ignore file. The
identity hash, the snapshot copy, and the materialised slot all operate
over the source *minus* that denylist. It must be a **denylist, not an
allow-list**: a per-file allow-list resurrects a write manifest that
someone has to maintain and breaks the verbatim guarantee — a human
reading the package directory should see *exactly* what materialises,
with no path rewriting and no hidden selection. The denylist only
formalises "what was never source"; it introduces no choice.

**Symptoms you need this.** Identity changes when nothing but the build
changed; a per-file ship list nobody keeps current; a materialise step
that copies gigabytes of build output.

## P4 — build output goes to a gitignored location {#build-output-elsewhere}

**Context.** Once a package can carry code, building it consumer-side is
tempting to do in place. But writing build output into the committed
tree pollutes it, and folding that output into identity (P3) makes the
hash flip on every build.

**The law.** *Build output goes to a gitignored location, never the
committed tree and never the identity hash.*

**Mechanics.** A build hook directs its output to a gitignored path
**outside** the committed slot; the hook is handed the project root so
it can address that location explicitly. The slot's reset semantics are
then trivially safe — there is nothing in the slot to reset, because
the build never wrote there. A language-native consumer may instead
reference the shipped source through its own toolchain and skip the
hook entirely. Either path, the committed tree and the identity hash
see only source; build output lives where version control already
ignores it.

**Symptoms you need this.** A build that dirties the committed tree;
identity that flips after a build; a re-materialise that clobbers build
state the slot should never have held.

## P5 — vendor and commit the bootstrap toolchain {#vendor-bootstrap}

**Context.** A tool that consumes its own discipline toolchain has a
chicken-and-egg risk: a fresh clone cannot build because the tool that
builds it is not there yet. The bootstrap has to come from somewhere
that exists before the first build.

**The law.** *Vendor and commit the bootstrap toolchain beside the code
that needs it — there is no chicken-and-egg.*

**Mechanics.** The consumed toolchain lives in a **committed** slot, so
a fresh clone builds from a clean checkout with **no prior install
step** — the dependency target already exists in the tree. The
development loop stays ergonomic: editing the in-repo package source
re-materialises the slot on the next install, so the consumed copy
tracks the edited source without a manual wipe. The toolchain a build
needs is vendored beside the code that needs it, versioned and offline-
buildable — not fetched from a network the first build cannot yet
reach.

**Symptoms you need this.** "Run the installer before you can build the
installer"; a clone that will not build offline; a bootstrap step that
assumes the tool is already on the search path.

## P6 — spike before the irreversible move {#spike-first}

**Context.** Relocating code across a workspace boundary, or adopting a
cross-workspace topology, has sharp platform-specific edges — on one OS,
path canonicalisation adds a prefix and the build tool mishandles it.
Discover that *after* the move and you have paid for a migration you
must now unwind under pressure.

**The law.** *Spike the risky topology empirically on the target
platform before the irreversible move; keep an evidence-chosen
fallback.*

**Mechanics.** Before physically relocating anything, validate the
risky arrangement on the **actual target host** — the one with the
sharp edges, not the friendliest one to hand. Keep a rejected-but-
retained alternative ready, and choose between the primary design and
the fallback on spike **evidence**, not by default or by taste. The
irreversible move happens only after the topology is proven to work
where it actually has to.

**Symptoms you need this.** A migration that works on the author's OS
and breaks on the user's; a topology adopted on faith and unwound in a
panic; "it should work" standing in for "I ran it on the target."

## P7 — build the general mechanism on real demand {#build-on-demand}

**Context.** A language-neutral core is a genuine reusable artifact, and
that makes it tempting to extract up front. But extracting it before a
second consumer exists is speculative: you design the abstraction
against a single use, and the second use reshapes it anyway — so you
pay for generality twice.

**The law.** *Extract the general mechanism when the second consumer
arrives, not before.*

**Mechanics.** Ship the whole thing in the one place that needs it now,
and **document** the future extraction as a clean follow-up with an
explicit trigger ("taken when the first non-X pilot needs it"). The end
state may well be symmetric — a neutral core with per-language frontends
— but the ordering is driven by real second-consumer demand, not built
on speculation. When the second consumer is real, you extract against
**two** real uses instead of one imagined one, and the abstraction fits
on the first try.

**Symptoms you need this.** An abstraction with exactly one caller; a
"reusable core" reshaped the moment its second user appears; generality
paid for long before it earns out.

## Summary {#summary}

- P1 — a package is a project; the same layout, the same checks, no
  package-only convention.
- P2 — ship the runtime, not prose describing a tool the consumer never
  receives.
- P3 — identity is the source, excluded by a denylist of never-source
  dirs, never a per-file allow-list.
- P4 — build output goes to a gitignored location, outside the committed
  tree and the hash.
- P5 — vendor and commit the bootstrap toolchain beside the code that
  needs it.
- P6 — spike the risky topology on the target platform first; keep an
  evidence-chosen fallback.
- P7 — extract the general mechanism when the second consumer arrives,
  not on speculation.
