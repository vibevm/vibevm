# Self-updating tools — lessons S1–S7 {#root}

**Scope of this document.** Seven lessons from building a tool that
installs, switches, and removes its own versions on a live machine.
Each is self-contained: the failure that taught it, the law in one
bolded line, the mechanics that satisfy the law, and a "symptoms you
need this" line so you can recognise the problem before you have paid
for it. Vocabulary is generic — *the launcher*, *the active pointer*,
*the instance directory* — because the laws outlive any one tool.

## S1 — activation truth is a live pointer, not the environment {#live-pointer}

**Context.** The first design made an environment variable the single
source of truth for the active version. Environment variables are
inherited at process start and frozen until the shell reloads — so
every switch or reinstall forced the user to open a new terminal. The
friction was structural, not a bug.

**The law.** *The active version is a live pointer file read on every
launch, plus the running binary's own path; the environment is
advisory, never the truth.*

**Mechanics.** A tiny launcher shim reads an active-pointer file each
time it runs and executes whatever instance that file names. Switching
rewrites the pointer, so the **next** invocation in the *same* shell
picks up the change with no reload — the filesystem is live where the
shell's environment is frozen. The running binary derives its own
identity from its own path (a `current_exe`-style lookup): it *is* the
binary, so it knows which version and which home it belongs to without
consulting any variable. Keep the environment variable set for external
tools that expect a `HOME`-style value, but demote it to advisory and
reconcile actual-vs-environment on demand; a managed process whose real
home disagrees with the stale variable can warn at startup.

**Symptoms you need this.** Users must open a new terminal after every
switch; scripts break on a stale home variable; "it works after I
restart my shell."

## S2 — the unit of install is a directory, switched by a pointer {#immutable-instances}

**Context.** The first idea for "reinstall over the running binary" was
a rename-aside trick — rename the running file, write the new one in
its place. It was empirically verified to work, and it handles exactly
one file. But a real distribution is many files — the binary plus
shared libraries and assets — and all of them are locked while the
process runs.

**The law.** *The unit of install and switch is a whole immutable
instance directory; activation is a pointer flip, so nothing in use is
ever overwritten.*

**Mechanics.** Each install writes a **new** instance directory and
leaves every prior one intact. Switching flips the active pointer (S1)
to the new directory. Because no in-use file is ever rewritten, there
are no file locks and no reload — the model is safe even for a shared
library the OS refuses to replace while it is mapped. The running
process keeps its own directory until it exits; reinstalling the
version you are currently running simply produces a fresh instance and
leaves the live one untouched. The rename-aside trick was dropped as
unnecessary once the unit became the directory.

**Symptoms you need this.** "File is locked by another process" on
reinstall; a self-update that cannot replace its own binary; shared
libraries that cannot be overwritten while loaded.

## S3 — cheap identity: count instances, do not hash gigabytes {#cheap-identity}

**Context.** A natural key for an instance is a content hash of the
built distribution — it deduplicates and it is self-describing. It does
not scale. A distribution may grow to gigabytes and ship as merged
binaries, and hashing 2 GB+ on every install is prohibitive.

**The law.** *Never content-hash a large payload to establish identity;
use a monotonic instance counter and cheap change detection.*

**Mechanics.** The instance key is a monotonic counter — always unique,
O(1), independent of payload size. To decide what to carry between
instances, hash only **small** files (below a threshold) and trust
`(size, mtime)` for **large** ones — stat, never read. Hardlink
unchanged files into the new instance; copy only the changed ones. If a
persistent build cache preserves mtimes across builds, "did anything
change" is answered without reading a byte of the big files. When
nothing changed, make no new instance at all. A prebuilt artifact is
keyed by the publisher's digest computed **once at publish**, never
re-hashed locally.

**Symptoms you need this.** Install time grows with payload size; a
multi-GB asset re-hashed on every run; dedup logic that reads files it
could have stat'd.

## S4 — hold sources by reference, never bulk-copy them {#sources-by-reference}

**Context.** Copying a source checkout into the tool's own storage is
untenable: a working tree's build directory is already tens of GB, and
it churns constantly. Copy it once and you have a stale, enormous
duplicate; copy it every install and the tool is unusable.

**The law.** *Hold sources by reference; never bulk-copy them into the
tool's own storage.*

**Mechanics.** A tool-owned source is a clone the tool updates
**incrementally** (fetch and checkout, stash first if dirty), never
re-clones — so a full rebuild is avoided. A user's own checkout is a
different origin: reference it by its canonical absolute path and build
it **in place**, never mutating its VCS state and never copying it.
Recording that path yields a *linked source* — a later install can
rebuild from the remembered location without being in the checkout,
with a clear error if it has moved. Only the built distribution (small
next to the source, and diff-copied per S3) is placed into an instance;
the source itself never is.

**Symptoms you need this.** The install root balloons to tens of GB; a
full re-clone on every install; a build that mutates the user's git
state.

## S5 — durable environment edits: idempotent, additive, consented, testable {#durable-env-edits}

**Context.** A tool that puts itself on the search path or sets a home
variable edits durable machine state that outlives the process. Get it
wrong and you corrupt a user's shell profile, duplicate an entry on
every run, or silently mutate a developer's machine from a test.

**The law.** *Edit durable environment state idempotently and
additively, with consent and honesty — and behind an injectable seam so
tests never touch the real machine.*

**Mechanics.** Five rules, none optional. **Idempotent** — a marker
guards the edit, so re-running adds no duplicate line or entry.
**Never clobber** — add only your own entry and preserve the rest of
the search path; you are a guest in a file the user owns. **OS/shell-aware**
— the user environment registry on one platform, a marked block in the
detected shell's rc file (bash/zsh/fish/profile) on another. **Consent
and honesty** — a mutating edit needs a confirm or an explicit yes flag,
prints the diff it will apply, and states plainly that the change
reaches only **new** shells. **Injectable seam** — the durable writer is
an interface the tests stub with a temporary file, so the suite
exercises the rc-file path without mutating the developer's box. (The
mechanics of writing safely *inside* a shared, human-owned file are a
lesson of their own — see `flow:managed-blocks`.)

**Symptoms you need this.** Duplicated search-path entries; a clobbered
profile; tests that pass only on the author's machine; users surprised
their current shell did not pick up the change.

## S6 — required tools live in one runnable table {#runnable-knowledge}

**Context.** A from-source build needs a specific host stack — a
compiler, a linker, a version-control client, a language toolchain at a
minimum version. If that list lives in prose, it drifts from what the
code actually checks, and bumping the stack means editing several
disconnected places.

**The law.** *Keep the required tools in one table the doctor command
reads and a test asserts — knowledge is runnable, so updates are
mechanical.*

**Mechanics.** One table, each row `(name, minimum version, check
command, help URL)`. The doctor command iterates it and reports what is
missing, with remediation, rather than failing deep in a build. A test
asserts the table is well-formed, so it cannot rot unnoticed. Related
knowledge follows the same rule: the default build profile is a single
constant, and the language pin is **read** from the toolchain file, not
hard-coded. When the stack moves you edit the table, and the doctor and
the test move with it for free. A secret (a publish token, say) is
never in this set — required *to build* and required *to publish* are
different lists.

**Symptoms you need this.** "Works on my machine" build failures; a
setup document that lists a tool the checker forgot; a stack bump that
needs edits in four files.

## S7 — removal and garbage collection that protect {#safe-removal}

**Context.** A remove or garbage-collect command that is too eager will
delete the version you are running out from under you, or wipe a shared
cache that other tools on the machine depend on. Destructive by default
is how a version manager loses a user's trust in one command.

**The law.** *Removal protects the active and the running instance and
never touches shared caches; a wholesale wipe needs an explicit flag and
a reconfirm.*

**Mechanics.** Removing the **active** version requires a force flag;
the **running** instance's files are never deleted out from under it
(best-effort — skipped if locked, collected on a later run; on some
systems the unlink succeeds and the inode lives until the process
exits). Garbage collection operates **only** inside the tool's own
install root — never the shared package caches other tools rely on. A
"remove everything" needs both an explicit flag **and** a
re-confirmation, never a bare invocation. A user's own source tree is
never removed — the tool only forgets its provenance record; tool-owned
clones are the tool's to drop. Hardlinked files (S3) are refcount-safe:
dropping one instance never corrupts another that shares its inodes.

**Symptoms you need this.** A garbage-collect that deletes the binary
you are running; a wipe that nukes a cache shared with other tools; an
uninstall that removes a user's checkout.

## Summary {#summary}

- S1 — the active version is a live pointer file plus the running
  binary's own path; the environment is advisory.
- S2 — install and switch a whole immutable directory; flip a pointer,
  overwrite nothing in use.
- S3 — count instances with a monotonic key; hash small files, stat big
  ones, hardlink the unchanged.
- S4 — reference sources, never bulk-copy them; build in place or update
  a clone incrementally.
- S5 — durable edits are idempotent, additive, consented, OS-aware, and
  behind a test seam.
- S6 — one required-tools table the doctor reads and a test asserts.
- S7 — protect the active and running instance; a full wipe is
  flag-plus-reconfirm, never default.
