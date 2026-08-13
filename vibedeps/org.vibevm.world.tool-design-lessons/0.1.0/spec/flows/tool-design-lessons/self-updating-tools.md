# Self-updating tools — lessons S1–S7 {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** Seven lessons from building a tool that
installs, switches, and removes its own versions on a live machine. @status:impl/done

@fact:each-lesson-is-self-contained Each is self-contained: the failure that taught it, the law in one
bolded line, the mechanics that satisfy the law, and a "symptoms you
need this" line so you can recognise the problem before you have paid
for it. @status:impl/done

@fact:vocabulary-is-generic Vocabulary is generic — *the launcher*, *the active pointer*,
*the instance directory* — because the laws outlive any one tool. @status:impl/done

## S1 — activation truth is a live pointer, not the environment {#live-pointer}

@fact:s1-context-env-was-the-single-source-of-truth **Context.** The first design made an environment variable the single
source of truth for the active version. @status:spec/done

@fact:s1-context-env-is-frozen-until-the-shell-reloads Environment variables are
inherited at process start and frozen until the shell reloads — so
every switch or reinstall forced the user to open a new terminal. @status:spec/done

@fact:s1-context-the-friction-was-structural The friction
was structural, not a bug. @status:spec/done

@fact:S1-LAW-ACTIVATION-TRUTH-IS-A-LIVE-POINTER **The law.** *The active version is a live pointer file read on every
launch, plus the running binary's own path; the environment is
advisory, never the truth.* @status:impl/done

@fact:S1-MECHANICS-A-LAUNCHER-SHIM-READS-THE-POINTER-EACH-RUN **Mechanics.** A tiny launcher shim reads an active-pointer file each
time it runs and executes whatever instance that file names. @status:impl/done

@fact:S1-MECHANICS-SWITCHING-REWRITES-THE-POINTER Switching
rewrites the pointer, so the **next** invocation in the *same* shell
picks up the change with no reload — the filesystem is live where the
shell's environment is frozen. @status:impl/done

@fact:S1-MECHANICS-THE-BINARY-DERIVES-ITS-IDENTITY-FROM-ITS-OWN-PATH The running binary derives its own
identity from its own path (a `current_exe`-style lookup): it *is* the
binary, so it knows which version and which home it belongs to without
consulting any variable. @status:impl/done

@fact:S1-MECHANICS-KEEP-THE-VARIABLE-BUT-DEMOTE-IT-TO-ADVISORY Keep the environment variable set for external
tools that expect a `HOME`-style value, but demote it to advisory and
reconcile actual-vs-environment on demand; a managed process whose real
home disagrees with the stale variable can warn at startup. @status:impl/done

@fact:s1-symptoms **Symptoms you need this.** Users must open a new terminal after every
switch; scripts break on a stale home variable; "it works after I
restart my shell." @status:spec/done

## S2 — the unit of install is a directory, switched by a pointer {#immutable-instances}

@fact:s2-context-the-rename-aside-trick **Context.** The first idea for "reinstall over the running binary" was
a rename-aside trick — rename the running file, write the new one in
its place. @status:spec/done

@fact:s2-context-it-worked-and-handles-exactly-one-file It was empirically verified to work, and it handles exactly
one file. @status:spec/done

@fact:s2-context-a-real-distribution-is-many-locked-files But a real distribution is many files — the binary plus
shared libraries and assets — and all of them are locked while the
process runs. @status:spec/done

@fact:S2-LAW-THE-UNIT-IS-A-WHOLE-IMMUTABLE-INSTANCE-DIRECTORY **The law.** *The unit of install and switch is a whole immutable
instance directory; activation is a pointer flip, so nothing in use is
ever overwritten.* @status:impl/done

@fact:S2-MECHANICS-EACH-INSTALL-WRITES-A-NEW-DIRECTORY **Mechanics.** Each install writes a **new** instance directory and
leaves every prior one intact. @status:impl/done

@fact:S2-MECHANICS-SWITCHING-FLIPS-THE-ACTIVE-POINTER Switching flips the active pointer (S1)
to the new directory. @status:impl/done

@fact:S2-MECHANICS-NO-LOCKS-AND-NO-RELOAD Because no in-use file is ever rewritten, there
are no file locks and no reload — the model is safe even for a shared
library the OS refuses to replace while it is mapped. @status:impl/done

@fact:S2-MECHANICS-THE-RUNNING-PROCESS-KEEPS-ITS-OWN-DIRECTORY The running
process keeps its own directory until it exits; reinstalling the
version you are currently running simply produces a fresh instance and
leaves the live one untouched. @status:impl/done

@fact:s2-mechanics-the-rename-aside-trick-was-dropped The rename-aside trick was dropped as
unnecessary once the unit became the directory. @status:spec/done

@fact:s2-symptoms **Symptoms you need this.** "File is locked by another process" on
reinstall; a self-update that cannot replace its own binary; shared
libraries that cannot be overwritten while loaded. @status:spec/done

## S3 — cheap identity: count instances, do not hash gigabytes {#cheap-identity}

@fact:s3-context-a-content-hash-is-a-natural-key **Context.** A natural key for an instance is a content hash of the
built distribution — it deduplicates and it is self-describing. @status:spec/done

@fact:s3-context-it-does-not-scale It does
not scale. @status:spec/done

@fact:s3-context-hashing-gigabytes-is-prohibitive A distribution may grow to gigabytes and ship as merged
binaries, and hashing 2 GB+ on every install is prohibitive. @status:spec/done

@fact:S3-LAW-NEVER-CONTENT-HASH-A-LARGE-PAYLOAD **The law.** *Never content-hash a large payload to establish identity;
use a monotonic instance counter and cheap change detection.* @status:impl/done

@fact:S3-MECHANICS-THE-INSTANCE-KEY-IS-A-MONOTONIC-COUNTER **Mechanics.** The instance key is a monotonic counter — always unique,
O(1), independent of payload size. @status:impl/done

@fact:S3-MECHANICS-HASH-SMALL-FILES-AND-STAT-LARGE-ONES To decide what to carry between
instances, hash only **small** files (below a threshold) and trust
`(size, mtime)` for **large** ones — stat, never read. @status:impl/done

@fact:S3-MECHANICS-HARDLINK-UNCHANGED-FILES Hardlink
unchanged files into the new instance; copy only the changed ones. @status:impl/done

@fact:S3-MECHANICS-A-BUILD-CACHE-ANSWERS-DID-ANYTHING-CHANGE If a
persistent build cache preserves mtimes across builds, "did anything
change" is answered without reading a byte of the big files. @status:impl/done

@fact:S3-MECHANICS-WHEN-NOTHING-CHANGED-MAKE-NO-NEW-INSTANCE When nothing changed, make no new instance at all. @status:impl/done

@fact:S3-MECHANICS-A-PREBUILT-ARTIFACT-IS-KEYED-AT-PUBLISH A prebuilt artifact is
keyed by the publisher's digest computed **once at publish**, never
re-hashed locally. @status:impl/done

@fact:s3-symptoms **Symptoms you need this.** Install time grows with payload size; a
multi-GB asset re-hashed on every run; dedup logic that reads files it
could have stat'd. @status:spec/done

## S4 — hold sources by reference, never bulk-copy them {#sources-by-reference}

@fact:s4-context-copying-a-checkout-is-untenable **Context.** Copying a source checkout into the tool's own storage is
untenable: a working tree's build directory is already tens of GB, and
it churns constantly. @status:spec/done

@fact:s4-context-copy-once-or-copy-always-both-fail Copy it once and you have a stale, enormous
duplicate; copy it every install and the tool is unusable. @status:spec/done

@fact:S4-LAW-HOLD-SOURCES-BY-REFERENCE **The law.** *Hold sources by reference; never bulk-copy them into the
tool's own storage.* @status:impl/done

@fact:S4-MECHANICS-A-TOOL-OWNED-SOURCE-IS-UPDATED-INCREMENTALLY **Mechanics.** A tool-owned source is a clone the tool updates
**incrementally** (fetch and checkout, stash first if dirty), never
re-clones — so a full rebuild is avoided. @status:impl/done

@fact:S4-MECHANICS-A-USERS-CHECKOUT-IS-BUILT-IN-PLACE A user's own checkout is a
different origin: reference it by its canonical absolute path and build
it **in place**, never mutating its VCS state and never copying it. @status:impl/done

@fact:S4-MECHANICS-RECORDING-THE-PATH-YIELDS-A-LINKED-SOURCE Recording that path yields a *linked source* — a later install can
rebuild from the remembered location without being in the checkout,
with a clear error if it has moved. @status:impl/done

@fact:S4-MECHANICS-ONLY-THE-BUILT-DISTRIBUTION-ENTERS-AN-INSTANCE Only the built distribution (small
next to the source, and diff-copied per S3) is placed into an instance;
the source itself never is. @status:impl/done

@fact:s4-symptoms **Symptoms you need this.** The install root balloons to tens of GB; a
full re-clone on every install; a build that mutates the user's git
state. @status:spec/done

## S5 — durable environment edits: idempotent, additive, consented, testable {#durable-env-edits}

@fact:s5-context-durable-machine-state-outlives-the-process **Context.** A tool that puts itself on the search path or sets a home
variable edits durable machine state that outlives the process. @status:spec/done

@fact:s5-context-get-it-wrong-and-you-corrupt-a-users-machine Get it
wrong and you corrupt a user's shell profile, duplicate an entry on
every run, or silently mutate a developer's machine from a test. @status:spec/done

@fact:S5-LAW-EDIT-DURABLE-STATE-IDEMPOTENTLY-AND-ADDITIVELY **The law.** *Edit durable environment state idempotently and
additively, with consent and honesty — and behind an injectable seam so
tests never touch the real machine.* @status:impl/done

@fact:S5-MECHANICS-FIVE-RULES-NONE-OPTIONAL **Mechanics.** Five rules, none optional. @status:impl/done

@fact:S5-MECHANICS-IDEMPOTENT **Idempotent** — a marker
guards the edit, so re-running adds no duplicate line or entry. @status:impl/done

@fact:S5-MECHANICS-NEVER-CLOBBER **Never clobber** — add only your own entry and preserve the rest of
the search path; you are a guest in a file the user owns. @status:impl/done

@fact:S5-MECHANICS-OS-AND-SHELL-AWARE **OS/shell-aware**
— the user environment registry on one platform, a marked block in the
detected shell's rc file (bash/zsh/fish/profile) on another. @status:impl/done

@fact:S5-MECHANICS-CONSENT-AND-HONESTY **Consent
and honesty** — a mutating edit needs a confirm or an explicit yes flag,
prints the diff it will apply, and states plainly that the change
reaches only **new** shells. @status:impl/done

@fact:S5-MECHANICS-INJECTABLE-SEAM **Injectable seam** — the durable writer is
an interface the tests stub with a temporary file, so the suite
exercises the rc-file path without mutating the developer's box. @status:impl/done

@fact:s5-mechanics-managed-blocks-pointer (The
mechanics of writing safely *inside* a shared, human-owned file are a
lesson of their own — see `flow:managed-blocks`.) @status:impl/done

@fact:s5-symptoms **Symptoms you need this.** Duplicated search-path entries; a clobbered
profile; tests that pass only on the author's machine; users surprised
their current shell did not pick up the change. @status:spec/done

## S6 — required tools live in one runnable table {#runnable-knowledge}

@fact:s6-context-a-from-source-build-needs-a-host-stack **Context.** A from-source build needs a specific host stack — a
compiler, a linker, a version-control client, a language toolchain at a
minimum version. @status:spec/done

@fact:s6-context-a-prose-list-drifts If that list lives in prose, it drifts from what the
code actually checks, and bumping the stack means editing several
disconnected places. @status:spec/done

@fact:S6-LAW-REQUIRED-TOOLS-LIVE-IN-ONE-RUNNABLE-TABLE **The law.** *Keep the required tools in one table the doctor command
reads and a test asserts — knowledge is runnable, so updates are
mechanical.* @status:impl/done

@fact:S6-MECHANICS-ONE-TABLE-WITH-FOUR-COLUMNS **Mechanics.** One table, each row `(name, minimum version, check
command, help URL)`. @status:impl/done

@fact:S6-MECHANICS-THE-DOCTOR-ITERATES-THE-TABLE The doctor command iterates it and reports what is
missing, with remediation, rather than failing deep in a build. @status:impl/done

@fact:S6-MECHANICS-A-TEST-ASSERTS-THE-TABLE-IS-WELL-FORMED A test
asserts the table is well-formed, so it cannot rot unnoticed. @status:impl/done

@fact:S6-MECHANICS-RELATED-KNOWLEDGE-FOLLOWS-THE-SAME-RULE Related
knowledge follows the same rule: the default build profile is a single
constant, and the language pin lives **once, in the workspace manifest**
(`rust-version`) and is **read** from it at build time — never repeated
in the tool table by hand. (The toolchain file keeps its own job — the
channel; the manifest's pin is what the tool checks against, and the
enforcing compiler reads the same key.) @status:impl/done

@fact:S6-MECHANICS-THE-DOCTOR-AND-THE-TEST-MOVE-FOR-FREE When the stack moves you edit the table, and the doctor and
the test move with it for free. @status:impl/done

@fact:S6-MECHANICS-A-SECRET-IS-NEVER-IN-THIS-SET A secret (a publish token, say) is
never in this set — required *to build* and required *to publish* are
different lists. @status:impl/done

@fact:s6-symptoms **Symptoms you need this.** "Works on my machine" build failures; a
setup document that lists a tool the checker forgot; a stack bump that
needs edits in four files. @status:spec/done

## S7 — removal and garbage collection that protect {#safe-removal}

@fact:s7-context-an-eager-remove-deletes-what-you-are-running **Context.** A remove or garbage-collect command that is too eager will
delete the version you are running out from under you, or wipe a shared
cache that other tools on the machine depend on. @status:spec/done

@fact:s7-context-destructive-by-default-loses-trust Destructive by default is how a version manager loses a user's trust
in one command. @status:spec/done

@fact:S7-LAW-REMOVAL-PROTECTS-THE-ACTIVE-AND-RUNNING-INSTANCE **The law.** *Removal protects the active and the running instance and
never touches shared caches; a wholesale wipe needs an explicit flag and
a reconfirm.* @status:impl/done

@fact:S7-MECHANICS-REMOVING-THE-ACTIVE-VERSION-REQUIRES-A-FORCE-FLAG **Mechanics.** Removing the **active** version requires a force flag;
the **running** instance's files are never deleted out from under it
(best-effort — skipped if locked, collected on a later run; on some
systems the unlink succeeds and the inode lives until the process
exits). @status:impl/done

@fact:S7-MECHANICS-GC-ONLY-INSIDE-THE-TOOLS-OWN-INSTALL-ROOT Garbage collection operates **only** inside the tool's own
install root — never the shared package caches other tools rely on. @status:impl/done

@fact:S7-MECHANICS-REMOVE-EVERYTHING-NEEDS-A-FLAG-AND-A-RECONFIRM A
"remove everything" needs both an explicit flag **and** a
re-confirmation, never a bare invocation. @status:impl/done

@fact:S7-MECHANICS-A-USERS-OWN-SOURCE-TREE-IS-NEVER-REMOVED A user's own source tree is
never removed — the tool only forgets its provenance record; tool-owned
clones are the tool's to drop. @status:impl/done

@fact:S7-MECHANICS-HARDLINKED-FILES-ARE-REFCOUNT-SAFE Hardlinked files (S3) are refcount-safe:
dropping one instance never corrupts another that shares its inodes. @status:impl/done

@fact:s7-symptoms **Symptoms you need this.** A garbage-collect that deletes the binary
you are running; a wipe that nukes a cache shared with other tools; an
uninstall that removes a user's checkout. @status:spec/done

## Summary {#summary}

- @fact:SUM-S1-LIVE-POINTER S1 — the active version is a live pointer file plus the running
  binary's own path; the environment is advisory. @status:impl/done
- @fact:SUM-S2-IMMUTABLE-INSTANCES S2 — install and switch a whole immutable directory; flip a pointer,
  overwrite nothing in use. @status:impl/done
- @fact:SUM-S3-CHEAP-IDENTITY S3 — count instances with a monotonic key; hash small files, stat big
  ones, hardlink the unchanged. @status:impl/done
- @fact:SUM-S4-SOURCES-BY-REFERENCE S4 — reference sources, never bulk-copy them; build in place or update
  a clone incrementally. @status:impl/done
- @fact:SUM-S5-DURABLE-ENV-EDITS S5 — durable edits are idempotent, additive, consented, OS-aware, and
  behind a test seam. @status:impl/done
- @fact:SUM-S6-RUNNABLE-KNOWLEDGE S6 — one required-tools table the doctor reads and a test asserts. @status:impl/done
- @fact:SUM-S7-SAFE-REMOVAL S7 — protect the active and running instance; a full wipe is
  flag-plus-reconfirm, never default. @status:impl/done
