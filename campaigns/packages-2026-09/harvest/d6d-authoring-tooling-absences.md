# D6d — four small `world` packages: seven claimed absences, re-verified before demotion

_Worked 2026-07-29. Subjects: `packages/org.vibevm.world/secrets-hygiene/v0.1.0/`,
`packages/org.vibevm.world/addressable-specs/v0.1.0/`,
`packages/org.vibevm.world/qualified-naming/v0.1.0/`,
`packages/org.vibevm.world/tool-design-lessons/v0.1.0/`. Seven obligations,
all `build-or-demote`, 8 drift verdicts. Every one asserts that some mechanism,
checker, artefact or record **does not exist**._

_This batch is worked under [§3.7
`#compliance-blindness`](../PHASE-D-BATCH-PLAN.md#compliance-blindness) and
[§6.1 `##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons):
a demotion is the **last** step, not the first, and a `not-found` is a fact
about the search perimeter until the perimeter has been checked. Measured over
wave 5's 76 `build-or-demote` verdicts, eighteen claimed absences were false and
seventeen were disproved by HOST artefacts. **Every entry below names the
perimeter it searched.** No code was written; no `git` command that writes was
run; no credential file was read, printed or echoed._

Obligations: F-327 · F-328 · F-287 · F-288 · F-244 · F-342 · F-343.

**The standing perimeter** (referred to below as *the standing perimeter*), run
from the repository root:

```
packages/**  vibedeps/**  crates/**  xtask/**  tools/**  spec/**
discipline/**  terraform/**  research/**  campaigns/**  legacy-spec/**
fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately**: it is a third-party study
corpus, not our shipped surface, and a hit there is not an implementation of
ours.

**Why that perimeter and not the package.** A mechanism in this family has four
layers — its SPEC in the package, its ENGINE in that package's library crates,
its DRIVER in a CLI, and its DEPLOYMENT in the consuming project. A fact can be
true at any one and invisible at the other three. These four packages specify
*prompt-only flows* — disciplines a consumer adopts — so the artefacts that
prove adoption live in the host, and a package-scoped search reads every
successful adoption back as an absence.

**Search for the THING, not for the string the verdict used.** A mechanism can
ship under another name, in another language, or as a shell script — and a
clause can be true *by a platform's own semantics* while being unenforced by our
code, which is two different findings the record must distinguish.

---

## F-327 / F-328 — one clause, judged as one set: the absence is real, the sentence is a norm, and the consumer is measurably not keeping it

*(Judged together under [§3.7's consistency corollary](../PHASE-D-BATCH-PLAN.md#compliance-blindness).
F-328's own verdict says it is «the same clause ruled at this package's boot
snippet and here stated at full strength», so re-verifying the row and not the
set would repeat exactly the error the corollary was bought for. The set turned
out to be **four** anchors, not two.)*

**Outcome:** ROUTE-B CANDIDATE (both obligations, both anchors) — **no edit made**
**Anchors:** 0 of 2 moved.
`57-flow-secrets-hygiene.md#LAW-NEVER-PERSISTED` — not edited (defined at that
file's line 35, a real definition, not a citation).
`SECRETS-HYGIENE-PROTOCOL.md#EXACTLY-ONE-SANCTIONED-AT-REST-LOCATION` — not
edited (defined at that file's line 71).
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above, **widened** past the
verdict's own `crates/ --include='*.rs'` to every file type that could carry a
permission act — `*.rs *.toml *.json *.sh *.ps1 *.md *.go *.ts *.py *.yml` —
across `packages/ vibedeps/ crates/ xtask/ tools/ spec/ discipline/ terraform/
research/ campaigns/ legacy-spec/ fixtures/ schemas/ docs/ manual-tests/` and
the repository root, for `set_permissions` · `PermissionsExt` · `from_mode` ·
`0o600` · `0o700` · `chmod` · `icacls` · `Set-Acl` · `SetNamedSecurityInfo` ·
`DACL` · `windows_acl` · `umask` · `attrib +`. **Widened a second time off the
string and onto the thing:** who *writes* a token file at all, what the two
operator-facing host guides say, and — because this host is Windows, where a
POSIX mode is not the mechanism — the actual ACL on the sanctioned directory.
`refs/**` searched and reported separately below.
**No credential file was read, opened, listed or printed.** Every ACL query
below is directory metadata; the standing prohibition on reading anything under
the settings dir was kept, and where it bounds a claim the bound is stated.

**What the search found — the absence is real, and more precisely so than the
verdict states:**

```console
$ grep -rn -E "set_permissions|PermissionsExt|from_mode|0o600|0o700|icacls|Set-Acl|SetNamedSecurityInfo|DACL|windows_acl|umask|attrib \+" \
    crates xtask tools spec discipline terraform schemas fixtures docs manual-tests \
    packages/org.vibevm.world packages/org.vibevm.ai-native \
    --include='*.rs' --include='*.toml' --include='*.json' --include='*.sh' --include='*.ps1' \
    --include='*.md' --include='*.go' --include='*.ts' --include='*.py' --include='*.yml'
crates/vibe-cli/src/commands/vvm/env.rs:143:    use std::os::unix::fs::PermissionsExt;
crates/vibe-cli/src/commands/vvm/env.rs:146:    fs::set_permissions(p, perm).with_context(|| format!("chmod +x `{}`", p.display()))?;
```

Two lines, one call, and it is the launcher `chmod +x` the verdict already
named. **The widened perimeter confirms the verdict rather than overturning it:**
nothing in this repository sets or checks the mode or the ACL of a credential
file, in any language, in any script, at any layer.

One implementation of the pattern does ship, in a package's own crate, and it is
worth naming because it is the shape a builder would copy:

```console
$ sed -n '52,67p' packages/org.vibevm.fractality/fractality/v0.1.0/crates/fractality-mc-client/src/lock.rs
    /// Writes the lockfile. On Unix the file is chmod 0600; on Windows the
    /// user-profile ACL is the boundary for v0.1 (hardening: DEF-10).
    pub fn write(&self, home: &Utf8Path) -> Result<(), String> {
        …
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path.as_std_path(), std::fs::Permissions::from_mode(0o600));
        }
```

That is a *bearer-token* lockfile, not a publish token, and it states in its own
doc comment the exact distinction this obligation turns on: chmod on Unix, the
profile ACL on Windows, and the Windows half deferred as `DEF-10`.

**And the fact the verdict did not reach: nothing writes the token file.**
`vibe-publish` only ever *reads* it — `read_token_file` at
`crates/vibe-publish/src/token.rs:201-217` is `fs::read_to_string`, and
`token_file_candidates` (`token.rs:228-233`) is the whole disk surface. There is
no writer anywhere:

```console
$ grep -rn -E "write.*publish\.token|publish\.token.*write|fs::write.*token" \
    crates xtask tools packages/org.vibevm.world packages/org.vibevm.ai-native --include='*.rs'
crates/vibe-publish/src/lib.rs:142:  Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibe/git.publish.token`. \
crates/vibe-publish/src/lib.rs:144:  fix: export `VIBEVM_PUBLISH_TOKEN` or write `~/.vibe/<host-prefix>.publish.token`)"
```

Both hits are **error text telling the operator to write it**. The file is
created by hand, outside the tool, so there is no code path at which vibevm
*could* set its mode — which makes «no `set_permissions` on a token file» a
weaker finding than it looks. The protection was never designed as a tool act.

**It was designed as an operator obligation, and the consumer wrote it down —
in both platform idioms:**

```console
$ grep -rn -iE "chmod|permission|acl" RUNTIME-GUIDE.md DEV-GUIDE.md
RUNTIME-GUIDE.md:53:**Token files are surface secrets** — set chmod 600 (POSIX) or restrict ACLs to
  your user (Windows), never commit, never paste into chat / logs / screenshots /
  video. `vibe` redacts the value at every level (CLI output, JSON event stream,
  error messages); the operator must extend the same discipline. See
  [PROP-000 §20](spec/common/PROP-000.md#token-secrecy).
DEV-GUIDE.md:47:- chmod 600 / Windows ACL-restricted to your user.
```

`RUNTIME-GUIDE.md:53` states the rule in the right idiom for each platform and
says outright that **«the operator must extend the same discipline»**. The
consumer therefore *accepted* this rule; it did not reject it, and it did not
route it to code. `spec/common/PROP-000.md:303`'s «chmod-protected» is the same
rule stated in the POSIX idiom only — a host-side wording defect on a
Windows-primary box, not a package defect.

**The decisive measurement — the consumer is not keeping the rule today.** This
host is Windows, where the mechanism is the directory ACL, so I measured the
ACL on the sanctioned directory itself. Directory metadata only; no file under
it was opened or listed:

```console
PS> $s = Join-Path $env:USERPROFILE ".vibe"
PS> (Get-Acl $s).Access | ForEach-Object { "{0} | {1} | {2} | inherited={3} | applies={4}" -f
      $_.IdentityReference,$_.AccessControlType,$_.FileSystemRights,$_.IsInherited,$_.InheritanceFlags }
overwatch\CodexSandboxUsers   | Allow  | ReadAndExecute, Synchronize | inherited=False | applies=ContainerInherit, ObjectInherit
NT AUTHORITY\SYSTEM           | Allow  | FullControl                 | inherited=True  | applies=ContainerInherit, ObjectInherit
BUILTIN\Administrators        | Allow  | FullControl                 | inherited=True  | applies=ContainerInherit, ObjectInherit
overwatch\olegc               | Allow  | FullControl                 | inherited=True  | applies=ContainerInherit, ObjectInherit

PS> (Get-Acl $env:USERPROFILE).Access | Where-Object { $_.IdentityReference -like "*CodexSandbox*" }
(no output — the group appears on `.vibe` but NOT on the profile directory)

PS> Get-LocalGroupMember -Group "CodexSandboxUsers"
overwatch\CodexSandboxOffline  (User)
overwatch\CodexSandboxOnline   (User)
```

Read that in order. The profile directory `C:\Users\olegc` *is* per-user by the
platform's own semantics — owner, SYSTEM, Administrators, nothing else. But the
sanctioned settings directory `C:\Users\olegc\.vibe` carries an **explicit,
non-inherited** ACE that the profile does not have, granting
`overwatch\CodexSandboxUsers` **ReadAndExecute**, with `ContainerInherit,
ObjectInherit` — i.e. propagating to the objects inside it. That group holds two
real accounts, `CodexSandboxOffline` and `CodexSandboxOnline`, neither of which
is the owner.

So «readable only by the owner» is **false of the live sanctioned location**, and
false by an act someone took deliberately on that directory. *Bound on this
claim, stated because the perimeter rule requires it:* I did not enumerate or
stat any file inside the settings dir — the standing prohibition forbids it — so
this is measured at directory grain, and a per-file `Deny` ACE overriding the
inherited grant would not have been visible to me. The inheritance flags make the
grant the default for children; whether every child actually carries it is the
one thing this measurement does not settle, and it is the first thing the host
task should check.

**Which layer has it, if any:** **nowhere** for enforcement — not spec, not
engine, not driver, not deployment: no code sets it, no checker verifies it, and
no code writes the file at which it could be set. **Consumer documentation**
carries the obligation correctly in both idioms (`RUNTIME-GUIDE.md:53`,
`DEV-GUIDE.md:47`). **The consumer's actual deployment violates it** (the `.vibe`
ACL above).

**Why this is route (b) and not a demotion — the sentence is a norm, and it is
the *right* norm.** §3.3 demotes a package that describes something as BUILT.
Neither sentence does. `##LAW-NEVER-PERSISTED` is one of «The four laws» and
reads *«**Never persisted** outside the one sanctioned at-rest location: a
per-user, permission-protected file…»* — a prescription on where a secret may
live, qualified by a property that location must have.
`##EXACTLY-ONE-SANCTIONED-AT-REST-LOCATION` states that property at full
strength. Neither claims vibevm chmods anything, and neither promises a checker.
The absence the verdict measured is an absence in the **consumer**, which §3.6
routes to the host and never to the package.

And here the default runs harder than usual in the package's favour, because the
measurement above is a **live counter-example proving the rule was needed**.
Appending «*Specified, not built*» to a credentials law on the day the sanctioned
directory is found readable by two non-owner accounts would be the *профанация*
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) exists to prevent, in the one place
where getting it wrong costs more than a wrong document.

**The consistency corollary fired, and it fired the other way.** The set is not
two anchors but **four**, and the same clause on the same evidence already
carries **two `confirmed` verdicts** alongside these two `drift` ones:

```console
$ python - <<  # over campaigns/packages-2026-09/tasks/evidence/batch-W5d-2.json and -3.json
57-flow-secrets-hygiene.md#LAW-NEVER-PERSISTED                             drift      (F-327)
57-flow-secrets-hygiene.md#NEVER-PERSIST-OUTSIDE-THE-SANCTIONED-LOCATION   confirmed
SECRETS-HYGIENE-PROTOCOL.md#EXACTLY-ONE-SANCTIONED-AT-REST-LOCATION        drift      (F-328)
SECRETS-HYGIENE-PROTOCOL.md#ROW-LAW-NEVER-PERSISTED                        confirmed
```

`##ROW-LAW-NEVER-PERSISTED` (`SECRETS-HYGIENE-PROTOCOL.md:42`) states *«Only one
sanctioned at-rest location: a per-user, permission-protected file (or an env var
for CI)»* — verbatim the clause under dispute — and was ruled **confirmed** on a
reason that names the problem and rules for the package anyway: *«The
"permission-protected" qualifier is the unsupported part — no `set_permissions`
call touches a token file anywhere in `crates/`»*. It already sits at
`@spec/done`. `##NEVER-PERSIST-OUTSIDE-THE-SANCTIONED-LOCATION`
(`57-flow-secrets-hygiene.md:71`) was likewise confirmed. **One clause, one body
of evidence, four anchors, two opposite verdicts** — whatever the boss rules, it
must be the same ruling for all four, and two of the four are already recorded
the way this entry recommends.

**`refs/**` — reported separately, and it is not ours.** Ten hits, every one
third-party study material: `refs/src/agent-scripts/skills/npm/scripts/npm-auth-login.mjs:87`
(`mode: 0o600`), `refs/src/agent-scripts/skills/release-mac-app/scripts/lib/mac_release.sh:213`
and `refs/src/agent-scripts/skills/ssh-doctor/SKILL.md:114,127` (`chmod 600`),
and six in `refs/src/cargo/`. None is an implementation of ours; they are
evidence that the pattern is ordinary, not that we run it.

**Verdict recommendation, per anchor:**
`##LAW-NEVER-PERSISTED` → **route (b)**, not drift — the absence is real and it
is the consumer's; the sentence is a law, not a record claim, and the consumer's
own `RUNTIME-GUIDE.md:53` accepts it.
`##EXACTLY-ONE-SANCTIONED-AT-REST-LOCATION` → **route (b)**, identically, and
**judged with its two already-confirmed siblings** rather than alone.

**Host obligation this opens, and it is the sharpest thing in this batch.** Two
non-owner accounts hold ReadAndExecute on the directory that is, by three of our
own documents, the single sanctioned at-rest location for every credential this
tooling handles. That is a live exposure, not a documentation defect, and it
belongs in front of the owner ahead of any verdict bookkeeping. The rule the
package states is the rule that would have prevented it.

**New obligations noticed:** (1) `spec/common/PROP-000.md:303`'s
`##TS-NEVER-PERSISTED` says «per-user, chmod-protected» — a POSIX-only idiom for
a property that on this platform is an ACL, while the consumer's own
`RUNTIME-GUIDE.md:53` already states it correctly in both. A host-side
`reality-mismatch`, outside my edit scope, recorded. (2) Nothing anywhere
*verifies* the sanctioned location's protection — no `vibe doctor` check, no
`vibe-check` rule (`grep -rn "token" crates/vibe-check/src crates/vibe-cli/src/commands/vvm/doctor.rs`
returns only unrelated tokenisers). Given that a live violation went unnoticed
until this pass, a checker is the Phase-E build this obligation actually implies
— and unlike the mode-setting the verdict asked for, a *checker* is something
vibevm can do without owning the file.

---

## F-287 — the graph carries both directions and answers the spec-side question; what does not ship is the *authoring side* of the second marker

**Outcome:** CORRECTED (right about the fact, wrong about the mechanism — §3.3 demotion would have been false)
**Anchors:** 1 of 1 touched.
`ADDRESSABLE-SPECS-PROTOCOL.md#CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT`
— corrected, marker deliberately **kept** at `@impl/done`. Verified to be a real
definition at that file's line 221, not a citation.
**Files touched:**
`packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md`
**Perimeter searched:** the standing perimeter above, for `Implements: spec://`
· `^Test:` · `^Tests:` · `^Verified-by:` · `^Verifies:` · `#[spec(` ·
`#[verifies(` · `#[specmark::spec(` · `#[specmark::verifies(` · `specmark::scope!` ·
`Verb::Verifies` · `explain` — plus three things a string search does not reach:
`specmap.json` read key by key, the `verifies` **producer** in the engine crate,
and the **renderer** that answers the spec-side question. The absence claim here
is about a *direction*, so the perimeter had to include the layer that consumes
the markers, not only the layer that writes them.

**What the search found — first, both halves of the verdict's premise check out:**

```console
$ grep -rhoE "#\[(specmark::)?(verifies|spec)\(|specmark::scope!\(" crates xtask --include='*.rs' | sort | uniq -c | sort -rn
    403 specmark::scope!(
    224 #[spec(
    222 #[verifies(
     74 #[specmark::spec(
      6 #[specmark::verifies(

$ grep -rn "^ *Test: |^ *Tests: |^ *Verified-by:|^ *Verifies:" spec discipline terraform docs legacy-spec packages --include='*.md'
packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL.md:230:Test: payments_core::tests::timeout_marks_old_messages
```

The `Test:` line the fact illustrates occurs **exactly once in the whole
perimeter, and it is this document's own example**. No host spec document
authors a verification record. That half of the verdict is solid.

On the first half the verdict is very slightly off, and the correction matters
for how the fact should read. `// Implements: spec://` does not occur "exactly
ONCE" — it occurs several times, and **not once in a source file of any
language**: every hit is prose *about* the practice (this document's own fence
at line 225, `flow:redbook`'s chapter 2 which is this flow's source, and
`flow:sync-from-code`'s `review-workflow.md:68`), plus vendored copies of those
under the fractality specspace's `vibedeps/` and `.vibe/cache/`. The comment
form is a teaching notation, never a shipped marker.

**And then the thing the verdict's perimeter did not reach: the second edge is
authored at scale, and the spec-side question is answered.** `#[verifies(...)]`
is used **228 times** (222 bare + 6 path-qualified) — within four of `#[spec]`'s
298. Those become edges through a named producer:

```console
$ grep -rn "verifies" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/rscan.rs
rscan.rs:1:   //! Rust side of the scanner: `#[spec]` / `#[verifies]` attributes and
rscan.rs:24:  specmark_grammar::Verb::Verifies => EdgeVerb::Verifies,
rscan.rs:105: "verifies" => match &attr.meta {
rscan.rs:107:     Ok(args) => out.push((args.into_verifies_edge(), line)),
```

and the renderer answers the question **from the spec unit**, not from the code
item — `explain_unit` takes a `spec://` URI and lists every edge into it:

```console
$ sed -n '89,116p' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/explain.rs
/// Render the subgraph around a `spec://` URI: the unit, every edge
/// into it, suspects.
fn explain_unit(map: &Specmap, uri: &str) -> Result<String> {
    …
    out.push_str("  edges in:\n");
    for e in edges { out.push_str(&format!("    {} ← `{}` ({}:{})…", verb_str(e), e.fromSymbol, e.file, e.line)); }
```

with its own tests asserting exactly this output (`explain.rs:414`
`assert!(text.contains("verifies ←"))`, and `:396` on the sibling-coverage line
`"also: verifies ← `vibe_resolver::conditional::tests::parses_simple`"`), and a
driver at `xtask/src/main.rs:343` (`run_trace_explain`).

The data is live in the host's committed map:

```console
$ python -c "…specmap.json…"   # edges into one unit, and the totals
spec unit spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-030#knob
  verifies ← vibe_cli::commands::install::flag_tests::offline_without_a_local_registry_bails_before_the_network (crates/vibe-cli/src/commands/install/flag_tests.rs:98)
  verifies ← vibe_cli::commands::install::flag_tests::short_circuit_conflicts_with_embedded_last          (…flag_tests.rs:67)
  verifies ← vibe_resolver::embedded_provider::tests::short_circuit_falls_through_when_embedded_absent    (crates/vibe-resolver/src/embedded_provider.rs:455)
  verifies ← vibe_resolver::embedded_provider::tests::short_circuit_stops_at_the_first_serving_provider   (…embedded_provider.rs:428)

distinct spec units with at least one verifies edge: 83
total verifies edges: 223        (verbs overall: implements 677 · verifies 223 · deviates 12)
```

So *"which test verifies this unit"* — the exact question the `Test:` line exists
to answer — **is answered, for 83 units, from the spec side, by shipped code with
a test on it.** What moved is the authoring side of the record, not its
existence.

**Which layer has it, if any:** **engine crate** for the producer and the
spec-side renderer (`core-ai-native-specmap/src/rscan.rs:105-107`,
`explain.rs:91-118`); **host driver** at `xtask/src/main.rs:343`; **host
deployment** for the data (`specmap.json`, 912 edges). **Nowhere** for the
`Test:` line as an authored artefact, and **nowhere** for the `// Implements:`
comment as a shipped marker — both are teaching notations only.

**The consistency corollary, checked before deciding — and it argues the same
way.** This anchor is one of five in the `{#graph}` section, and the other four
were all judged **confirmed** in the same Phase-C batch
(`campaigns/packages-2026-09/tasks/evidence/batch-W3a-1.json`):

```
ADDRESSABLE-SPECS-GIVE-A-DEPENDENCY-GRAPH-FOR-FREE  confirmed   "the host has built the graph and it is on disk"
CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT  drift   (F-287)
THESE-ARE-BIDIRECTIONAL-EDGES                      confirmed   "bidirectionality is enforced in code here, in both directions"
NO-TOOLING-IS-REQUIRED-TO-BENEFIT                  confirmed   "…the `Test:` line half has no host instance … filed on CODE-M…"
the-graph-pays-off-from-the-first-marker           confirmed   "markers first, tooling after"
```

The section's own lead is confirmed, bidirectionality is confirmed, and
`##NO-TOOLING-IS-REQUIRED-TO-BENEFIT`'s verdict says in its own words that the
`Test:`-line concern was **deliberately filed onto this anchor**. So F-287 is
the section's designated accumulator for one sub-claim, not an independent
finding — and demoting it would have put «specified, not built» on the one
anchor of five that carries the *how* of a graph the other four confirm is
built.

**What changed and why.** A correction, not a demotion, and the marker stays
`@impl/done` on purpose. The sentence keeps every original word — *"Code marks
what it implements; the spec records what verifies it:"* — and both example
fences are untouched. Appended is one italic parenthetical saying that those two
forms are the plain-text ones which need no tooling; that where a project
mechanizes the graph both records are commonly authored on the **code** side
instead, with the verification edge tagging the **test** rather than a `Test:`
line in the document; and that the spec-side answer is then rendered from the
graph rather than maintained by hand — *either form yields the same
bidirectional edge, only the authoring side moves.*

**A wording decision worth the boss's eye.** I kept the clause **tool-neutral** —
it names the *shape* (a tag on the test, a rendered spec-side answer) and not
`specmap` / `#[verifies]` / `xtask trace explain`. Two reasons, and the boss may
overrule either. First, this is a `world` flow meant to be re-derived by any
project, and its own `##COPY-THE-PROMPT-TASK-NOT-THE-IMPLEMENTATION` (line 246)
forbids transplanting it as dogma; naming one consumer's toolchain in it would
make the document less portable, not more true. Second, the confirmed sibling
`##NO-TOOLING-IS-REQUIRED-TO-BENEFIT` rests on the plain-text `Test:` line still
being a valid option, so the clause had to *add* the mechanized form without
retiring the low-tech one. If the gate wants the concrete host artefacts named,
the sentence to extend is this one and the artefacts are in the paragraph above.

**Verdict recommendation, per anchor:**
`##CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT` → **corrected, now
confirmed** — the bidirectional record exists in both directions and answers the
spec-side question for 83 units through `explain_unit`; only the authoring side
of the second marker differs from the illustration, and the text now says so.

**New obligations noticed:** (1) `##NO-TOOLING-IS-REQUIRED-TO-BENEFIT` (line 236,
`@impl/done`, confirmed, **not mine**) still asserts that *"the `Test:` line
answers 'which test verifies it'"* as a live half of a two-half claim whose other
half was verified by grep. With the `Test:` line now explicitly framed as one of
two forms, that anchor reads slightly ahead of its own evidence — worth a
re-read in the same pass, not a separate finding. (2) The 12 `deviates` edges in
`specmap.json` have no counterpart anywhere in this flow's vocabulary, which
describes only implements and verifies. Outside every anchor in my batch,
recorded.

---

## F-288 — a prescription the consumer does not keep, and it proves it *can* keep it: the same trigger is checked and fires, for code

**Outcome:** ROUTE-B CANDIDATE — **no edit made**
**Anchors:** 0 of 1 moved. `authoring-rules.md#SPLIT-WHEN-OVER-BUDGET` — not
edited. Verified to be a real definition at that file's line 149 (a list item
under `##split-triggers-lead`), not a citation.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above, for `budget` ·
`max_file_lines` · `token_budget` · `5000` · `3000` · `split` · `extract` ·
`promot` — **plus** a direct measurement rather than a grep, because an absence
of splits cannot be grepped for: every `spec/**/*.md` sized and ranked, and
every changelog line in `spec/**` read for a split record. Widened past the
sentence's own subject once, to ask whether this repository keeps the trigger
*anywhere* — which is what decides whether the rule is sound or merely unkept.

**What the search found — first, the verdict's number reproduces exactly:**

```console
$ python  # every spec/**/*.md sized (spec/boot excluded), words x 1.33 -> tokens
spec/ documents measured (spec/boot excluded): 59
over the 5000-token hard limit: 11
  ~ 45611 tok   34294 words  spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
  ~ 17321 tok   13024 words  spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md
  ~ 14362 tok   10799 words  spec/modules/vibe-registry/PROP-002-decentralized-registry.md
  ~ 14000 tok   10527 words  spec/modules/vibe-resolver/PROP-003-dep-evolution.md
  ~ 11209 tok    8428 words  spec/modules/vibe-index/PROP-005-package-index.md
  ~  6954 tok    5229 words  spec/modules/vibe-cli/PROP-037-tree-tui.md
  ~  6947 tok    5224 words  spec/common/PROP-019-version-manager.md
  ~  6784 tok    5101 words  spec/modules/vibe-workspace/PROP-035-spec-compiler.md
  ~  6524 tok    4906 words  spec/modules/vibe-progress/PROP-043-progress-markup.md
  ~  6047 tok    4547 words  spec/modules/vibe-workspace/PROP-007-workspace.md
  ~  5403 tok    4063 words  spec/common/PROP-000.md
```

Eleven over the limit; drop the two campaign plans in `spec/terraforms/`, which
are not «one module spec document», and the count is **exactly the nine** the
verdict names. The three worst — PROP-002 at ~14.4k, PROP-003 at ~14.0k,
PROP-005 at ~11.2k — are each **2–3× the hard limit and each still one file**.

Zero splits are recorded on this trigger:

```console
$ grep -rn -iE "^- \[20[0-9-]+\].*(split|extract|promot|moved out|budget)" spec --include='*.md'
(no output)
```

and nothing checks a spec document's size anywhere:

```console
$ grep -rn -iE "spec.*(token|size).*(budget|limit)|budget.*spec" crates/progress-core/src crates/vibe-check/src xtask/src progress.toml
(no output)
```

**The absence is therefore real, on the widest perimeter, and it is an absence in
the consumer.**

**The near-misses, named so they are not mistaken for the thing.** Two hits look
like this trigger and are not: `crates/progress-core/src/weave.rs:40-42` shards a
**generated weave** to a token budget (`estimate_tokens(&body) + … > budget`),
which is output sharding, not a document split; and `conform.toml:35`'s
`max_file_lines = 600` is a per-file **line** budget on **code**. Neither
measures a spec document.

**But the second of those is the finding that decides the route.** That code
budget has a checker — `FileLength { max_lines: 600 }`
(`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/budget.rs:137-141`),
configured at `conform.toml:35` — and it **fires, and files get split because of
it**, with the split recorded in the file that resulted:

```console
$ sed -n '1,8p' crates/progress-core/src/cache/tests.rs
//! The cache's own tests — what a record keeps, and what it refuses to keep.
//!
//! File-backed submodule of [`super`] so both cells stay inside the
//! AI-Native file budget, the same split `baseline/project` already
//! carries. Nothing moved but the file: every assertion here is the one
//! that stood beside the code it tests, …
```

So this repository keeps *exactly this trigger* — over budget, therefore split —
mechanically, with a checker, on its code. It does not keep it on its specs,
where this flow states it and where no checker exists. That is not a rule the
consumer rejected as unworkable; it is a rule the consumer demonstrably works by
elsewhere and has not extended to the surface this flow governs.

**Which layer has it, if any:** **nowhere** on the spec side — no checker, no
split, nine documents over. **Engine crate + host config** for the same trigger
on the code side (`core-ai-native-conform`'s `FileLength`, `conform.toml:35`),
with two host source files carrying the resulting splits.

**Why this is route (b) and not a demotion, decided by reading the sentence.**
The task's own test is whether the package states this as a *described practice*
or a *prescription*. It is unambiguously a prescription, and the grammar settles
it: the lead is `##split-triggers-lead` — **«Split when any of these fires:»**,
imperative — and `##SPLIT-WHEN-OVER-BUDGET` is one of four triggers hanging off
that imperative. It does not say «projects tend to split when over budget»; it
says split. §3.6 is then explicit that a package does not yield to a consumer
that simply does not comply, and softening a rule because it has been ignored
nine times is the *профанация* the mandate exists to prevent.

**The consistency check argues the same way, and unusually hard.** Every sibling
in the same two sections was judged **confirmed** in the same Phase-C batch
(`campaigns/packages-2026-09/tasks/evidence/batch-W3a-3.json`):

```
split-triggers-lead                      confirmed   "a lead-in to the four-item list … four bullets"
SPLIT-WHEN-OVER-BUDGET                   drift       (F-288)
SPLIT-WHEN-A-UNIT-NEEDS-AND-ALSO         confirmed   PROP-029's changelog line 50, a dated host instance
SPLIT-WHEN-TWO-AUDIENCES-EMERGE          confirmed   spec/modules/vibe-progress/ split three ways by genre
SPLIT-WHEN-ONE-SECTION-IS-CITED-FAR-MORE confirmed   PROP-029 #CHANGELOG-EXTRACTED, the one split on record
A-SPEC-PAST-ITS-BUDGET-IS-TWO-SPECS      confirmed   "the three worst overruns have exactly the shape the fact predicts"
THE-NUMBERS-ARE-BUDGETS-NOT-PHYSICS      confirmed
```

Read the last two together with F-288 and the case closes.
`##A-SPEC-PAST-ITS-BUDGET-IS-TWO-SPECS` — *«A spec that keeps growing past its
budget is not a big spec, it is two specs sharing a file»* — was **confirmed on
the host's own over-budget documents**, its verdict finding that each of the
three worst *«is a single `## 2. Decisions {#decisions}` unit — 611 lines in
PROP-002, 665 in PROP-003, 526 in PROP-005 — holding many decisions under one
anchor, in a file 2-3× over budget. Plural heading, plural content, one
address.»* The diagnosis this flow makes is therefore **confirmed true of the
consumer**, and F-288 is the same evidence read once more to observe that the
consumer has not acted on a diagnosis it agrees with. A rule whose neighbouring
anchor is confirmed *by the very documents that violate it* is the last rule in
this batch that should be softened.

**Verdict recommendation, per anchor:**
`##SPLIT-WHEN-OVER-BUDGET` → **route (b)**, not drift — the trigger is a
prescription under an imperative lead, the absence is nine unsplit documents in
the consumer, and the consumer already runs the identical trigger with a checker
on its code. The obligation is the host's.

**New obligations noticed:** (1) The host obligation this opens has an obvious
first move and it is worth writing down with the route: the spec-side budget has
no checker while the code-side one does, so «nine documents over» is invisible
until someone measures by hand — as this entry just did. A spec-document size
check is a small, mechanical Phase-E item of exactly the shape
`core-ai-native-conform` already ships for code. (2) The two `spec/terraforms/`
campaign plans are ~45.6k and ~17.3k tokens — 9× and 3.5× the hard limit, far
past every module spec — and were excluded above only because they are not
«module spec documents». Whether a campaign plan is inside or outside this budget
is undecided by the flow, and the larger of the two is this campaign's own
governing document. Recorded, not acted on; it is not this anchor's claim.

---

## F-244 — half a sentence is unbuilt in the consumer's resolver; the batch cut split the claim across two routes, and F-244 holds neither end of it

**Outcome:** ROUTE-B CANDIDATE, with a PARTIAL character (1 of 2 halves fails) —
**no edit made**, and a second, independent reason not to edit
**Anchors:** 0 of 2 moved.
`ref-grammar.md#THE-KIND-TAG-VALIDATES-IT-NEVER-DISAMBIGUATES` — not edited
(real definition at that file's line 65).
`ref-grammar.md#SUM-THE-KIND-TAG-VALIDATES-THE-RESOLVED-TYPE` — not edited
(real definition at line 183).
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above, and **deliberately not by
the verdict's string.** `KindMismatch` is the name of a thing that was never
built, so searching for it can only ever confirm itself. I searched for the
**thing** instead, three ways: (i) any post-resolution kind comparison or
mismatch error *under any name* — `kind.{0,30}mismatch` · `mismatch.{0,30}kind` ·
`expected kind` · `declared kind` · `kind does not match` · `.kind !=` — over
`crates/ xtask/ tools/`; (ii) **every consumer of the parsed field**, i.e. every
read of `pkgref.kind` anywhere in the tree, since a validation must read it to
perform it; (iii) every test whose name suggests a kind rejection. Then the
`KindMismatch` string itself across the whole perimeter for completeness.

**What the search found — the VALIDATES half is absent, and on a stronger
footing than the verdict had:**

```console
$ grep -rn "KindMismatch" crates xtask tools spec packages vibedeps docs schemas fixtures *.md
crates/vibe-core/src/package_ref.rs:428:   /// one; it is validated against the resolved manifest (a `KindMismatch`)
spec/design/workspace-and-qualified-naming.md:81:  … a present prefix is checked (`KindMismatch` on mismatch) …
spec/modules/vibe-registry/PROP-008-qualified-naming.md:97:  … the resolver asserts `resolved.kind == prefix`; mismatch is a `KindMismatch` error.
```

Three hits, none of them code — as the sibling verdict found. The searches for
the *thing* return nothing either: no comparison of a ref's kind against a
resolved manifest's kind exists under any spelling. And the third sweep closes
it, because it enumerates every place the parsed field is used at all:

```console
$ grep -rn -E "\bpkgref\.kind\b|\.kind\.is_some|if let Some\(kind\)" crates xtask tools --include='*.rs'
crates/vibe-cli/src/commands/registry/redirect/{create,sync,update}.rs   .repo_name(pkgref.kind, group, &pkgref.name)   # builds a repo name
crates/vibe-cli/src/commands/short_name.rs:135                          kind: pkgref.kind,                             # carried forward
crates/vibe-cli/src/commands/update.rs:110                              pkgref.kind,                                   # carried forward
crates/vibe-core/src/package_ref.rs:517                                 if let Some(kind) = self.kind                  # Display
crates/vibe-core/src/package_ref/tests.rs:109,120,129,140,212           assert_eq!(r.kind, …)                          # parse round-trip
```

Every single reader of the prefix either **renders** it, **carries** it forward,
or **builds a repository name** from it. Not one **checks** it. The prefix is
parsed, propagated and printed, and never once compared to anything. That is a
cleaner statement of the absence than «`KindMismatch` exists in no `.rs` file»,
and it forecloses the obvious rebuttal that the check ships under another name.

**And the NEVER-DISAMBIGUATES half is airtight, confirmed by reading the types:**

```console
$ sed -n '495,506p' crates/vibe-core/src/package_ref.rs
    /// The version-stripped identity string. … The
    /// `kind` prefix is never part of it — `kind` is metadata, not identity.
    pub fn qualified_name(&self) -> String {
        match &self.group { Some(group) => format!("{group}/{}", self.name), None => … }
    }

$ sed -n '28,37p' crates/vibe-cli/src/commands/short_name.rs
enum ShortNameOutcome {
    Resolved(Group),
    NotFound,
    /// More than one group publishes the name — a collision
    /// (PROP-008 §2.7). … the variant carries at least two.
    Ambiguous(Vec<Group>),
}
```

The candidate set is `Vec<Group>` and the identity string drops `kind`, so the
tag cannot enter disambiguation **even in principle**, and the error the user
sees offers only group-qualified alternatives (`short_name.rs:153-154`,
`render_collision(&pkgref.name, &groups)`).

**The two near-misses, checked and excluded.** Two sites do filter on kind —
`crates/vibe-index/src/index/search.rs:83` and
`crates/vibe-registry/src/search/full_scan.rs:164`, both `if let Some(k) =
kind_filter && …kind != k { continue; }`. I read both: they are the
`--kind` filter of the **search** command, narrowing a result list, and neither
sits on the pkgref-resolution path. They are not the missing validation, and
they are not kind disambiguating a reference. Worth naming because they are the
only two places in the repository where a package kind is compared to anything.

**Which layer has it, if any:** **nowhere** for the validation — no engine, no
driver, no deployment, and the field it would read is parsed and never checked.
**Host crates** for the never-disambiguates half, settled structurally rather
than by a check (`package_ref.rs:499-506`, `short_name.rs:28-37`). **Host spec**
for the claim restated and equally unbuilt (`PROP-008 ##KIND-VALIDATION`,
`@impl/done`; `spec/design/workspace-and-qualified-naming.md:81`).

**Why this is route (b): the campaign's own wave-2–4 rule decides it.** The §7
LOG entry for waves 2–4 states the test in one line — *«a package moves only
where its own sentence is false about something inside its own tree — its own
bullets, its own summary, its own example, or a shipped sibling in the same
namespace»*. `qualified-naming` is a prompt-only `world` flow specifying a
**reference grammar**; it ships no resolver, and its own tree contains nothing
this sentence is false about. Its own worked example agrees with it
(`ref-grammar.md:86-87`, *"`plugin` is checked against the manifest after
resolution"*), and its own summary agrees with it. What is false is the **host's
implementation of the grammar**, which parses the prefix and skips the check —
and which asserts the same unbuilt behaviour in its own `PROP-008
@fact:KIND-VALIDATION` at `@status:impl/done`. The obligation is the host's, on both
documents.

**The second reason not to edit, and it is independent of the first: the batch
cut split one claim across two routes, and F-244 holds neither end of it.** The
claim has six anchors in this one file and Phase C judged them as a set:

```
ROW-FORM-KIND-AND-NAME              ("kind is validated after resolution")   drift   -> F-178
THE-KIND-TAG-VALIDATES-IT-NEVER-DISAMBIGUATES                                drift   -> F-244
THE-RESOLVER-CHECKS-THE-TYPE-AND-ERRORS-ON-A-MISMATCH  (the body, the ROOT)  drift   -> F-178
SUM-THE-KIND-TAG-VALIDATES-THE-RESOLVED-TYPE                                 drift   -> F-244
ROW-FORM-KIND-AND-GROUP-QUALIFIED                                        confirmed
A-REAL-AMBIGUITY-IS-ALWAYS-A-GROUP-COLLISION   (the other half)           confirmed
```

F-244's own verdict names the root explicitly — *«Root at
`##THE-RESOLVER-CHECKS-THE-TYPE-AND-ERRORS-ON-A-MISMATCH`»* — and that root is
**not in F-244**. It sits in **F-178, typed `reality-mismatch`, routed
`sync-from-code`**, which §1.2 sends to the **owner, on every spec diff**. So
F-244 is the headline (line 65) and the summary (line 183) of a claim whose body
(line 67) belongs to an owner-approval obligation.

Demoting F-244 alone would leave the document reading *«The kind tag validates,
it never disambiguates. **Specified, not built.** @status:spec/done»* at line 65 and,
two lines below at line 67, *«the resolver checks that the resolved package's
type matches, and errors on a mismatch. @status:impl/done»* — a self-contradiction
authored by the batch boundary rather than by anything true. §6.1's
`##ROUTE-BEFORE-FALSIFIER` was bought for the batch cut ignoring `closure_route`;
this is the same lesson arriving from the other side, where honouring the route
cut one claim in half. **Whatever the boss rules, these four anchors move
together or not at all, and two of them need the owner.**

**Verdict recommendation, per anchor:**
`##THE-KIND-TAG-VALIDATES-IT-NEVER-DISAMBIGUATES` → **route (b)**, not drift —
one half of a two-half sentence is unimplemented **in the consumer's resolver**,
which this package does not ship; and the anchor cannot move without its body,
which is F-178's and the owner's.
`##SUM-THE-KIND-TAG-VALIDATES-THE-RESOLVED-TYPE` → **route (b)**, identically —
it is the summary restatement of the same claim and inherits its body's route by
the same precedent its own verdict invokes.

**New obligations noticed:** (1) The host obligation is **two documents, not
one**: `spec/modules/vibe-registry/PROP-008-qualified-naming.md:97`
(`##KIND-VALIDATION`, `@impl/done`) specifies `resolved.kind == prefix` and a
`KindMismatch` error that no code contains, and
`spec/design/workspace-and-qualified-naming.md:81` records the owner's decision
that a present prefix *is* checked. Both are host-side and outside my edit
scope; both are falsified by the same three searches above. (2) The build this
implies is genuinely small — the prefix is already parsed onto
`PackageRef.kind`, survives `qualify()`, and reaches the resolved reference, so
the missing work is one comparison and one error variant at the point of
resolution. §3.3's own *«Revisit when: an obligation's mechanism is a two-line
fix»* proviso may apply here more than anywhere else in this batch, and it is
the boss's call rather than mine.

---

## F-342 — the absence is real and deeper than the verdict found; the consumer has already recorded it, at the right marker

**Outcome:** ROUTE-B CANDIDATE — **no edit made**
**Anchors:** 0 of 1 moved.
`packaging-lessons.md#P4-MECHANICS-THE-HOOK-DIRECTS-OUTPUT-OUTSIDE-THE-COMMITTED-SLOT`
— not edited. Verified to be a real definition at that file's line 132.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above, all file types, for
`VIBE_PROJECT_ROOT` · `PROJECT_ROOT` — **and then off the string**, because a
project root can be handed to a child process three ways and only one of them is
an environment variable: I read the hook layer's **data model** (`HookContext`),
its **environment builder** (`build_env`), and the **working directory** the
spawned process actually gets (`HookRunner` / `SystemHookRunner`). A variable
named something else, or a cwd set to the root, would each satisfy the sentence.

**What the search found — the absence is real, and one layer deeper than the
verdict reached:**

```console
$ grep -rn -E "VIBE_PROJECT_ROOT|PROJECT_ROOT" crates xtask tools spec packages vibedeps discipline terraform docs schemas fixtures manual-tests legacy-spec *.md *.toml *.json
spec/common/PROP-024-code-bearing-packages.md:158:   that location, the hook runner gains a `VIBE_PROJECT_ROOT` environment
spec/common/PROP-024-code-bearing-packages.md:270:   **`VIBE_PROJECT_ROOT`** …
spec/modules/vibe-workspace/PROP-020-install-hooks.md:108:  `VIBE_PROJECT_ROOT`, the workspace absolute root, so a build hook can target a
specmap.json:22601:  "heading": "…**`VIBE_PROJECT_ROOT`**"   (the indexed copy of PROP-024:270)
```

Three spec lines and one derived index entry; no code, in any language. So far
this reproduces the verdict. The two extra sweeps make it stronger:

```console
$ grep -n -A9 "pub struct HookContext" crates/vibe-workspace/src/hooks.rs
pub struct HookContext<'a> {
    pub group: &'a Group,
    pub name: &'a str,
    pub version: &'a str,
    pub kind: &'a str,
    /// The materialised slot — the hook's working directory.
    pub slot: &'a Path,
}
```

**The hook layer's context has no project-root field at all.** The absence is
therefore not «one variable missing from a six-element vector» — the value does
not exist anywhere in the hook layer's data model, so `build_env`
(`crates/vibe-workspace/src/hooks.rs:357-372`) could not emit it even if someone
added the key. And the third route is closed too: the hook's working directory
is documented on that same field as **the slot**, and the runner honours it
(`hooks.rs:252`, `cmd.arg(&inv.script).current_dir(cwd)`). A hook therefore
starts *inside* the committed slot with no handle on anything above it. The only
thing left to a hook author is to walk up from `VIBE_PACKAGE_DIR` by counting
path segments — which is not «handed the project root», and is exactly the
fragility the variable exists to prevent.

**Which layer has it, if any:** **nowhere** — not the data model, not the
environment, not the cwd. **Host spec** for the specification, twice.

**But the consumer has already written the gap down, correctly marked — and that
decides the route.** Both host statements are honest about not having shipped it:

```console
$ sed -n '104,109p' spec/modules/vibe-workspace/PROP-020-install-hooks.md
- ##HOOK-ENV The runner passes a documented environment: `VIBE_PACKAGE_GROUP`,
  `VIBE_PACKAGE_NAME`, `VIBE_PACKAGE_VERSION`, `VIBE_PACKAGE_KIND`,
  `VIBE_PACKAGE_DIR` (the slot, also CWD), `VIBE_HOOK_PHASE`.
  ([PROP-024 §2.3](…) adds `VIBE_PROJECT_ROOT`, the workspace absolute root, so a
  build hook can target a gitignored build dir *outside* the slot; **it lands with
  that work**.) @impl/done

$ sed -n '154,161p' spec/common/PROP-024-code-bearing-packages.md
- ##HOOK-BUILD A code-bearing tool package builds via a **`post-install` hook** …
  To let a hook address that location, the hook runner **gains** a `VIBE_PROJECT_ROOT`
  environment variable … — a small [PROP-020 §2.2](…) addition. @spec/done
```

`##HOOK-ENV` is `@impl/done` and lists **exactly the six variables that ship** —
which is true — and puts the seventh in a parenthetical whose own words are
*«it lands with that work»*. `##HOOK-BUILD`, which specifies the addition, is
`@spec/done` — the honest marker for specified-not-built. The consumer is not
silently failing this mechanic; it has scheduled it, named the work it lands
with, and marked both statements correctly.

**Why this is route (b) and not a demotion.** Two reasons, and the second is the
document's own.

First, the campaign's wave-2–4 rule: *a package moves only where its own sentence
is false about something inside its own tree*. `tool-design-lessons` is a
prompt-only `world` flow; it ships no hook runner, no `build_env`, no
`HookContext`. Its sentence is false about **the host's** hook layer, and the
host is the side that owes the work.

Second — and this forecloses the obvious objection that a *lessons* document is
retrospective and therefore describing what its author actually built — the
document says in its own opening that it is not:

```console
$ sed -n '14,16p' packages/org.vibevm.world/tool-design-lessons/v0.1.0/spec/flows/tool-design-lessons/packaging-lessons.md
##vocabulary-is-generic Vocabulary is generic — *the
package*, *the consumer*, *the slot* — because the laws port even where
the build system does not.
```

Every lesson is structured **Context → Law → Mechanics → Symptoms**, and
«Mechanics» is *how the law is kept*, in generic vocabulary, deliberately
portable off the build system that taught it. `##P4-LAW-...` states the law;
this anchor states the design that satisfies it. That is a prescription, and
§3.6 does not let a prescription yield to a consumer that has scheduled rather
than shipped it.

**Verdict recommendation, per anchor:**
`##P4-MECHANICS-THE-HOOK-DIRECTS-OUTPUT-OUTSIDE-THE-COMMITTED-SLOT` →
**route (b)**, not drift — the absence is real and total (no field, no variable,
no cwd), and it is the consumer's; the consumer has already recorded it at
`@spec/done` with the work it lands with named. The obligation is the host's and
is already half-written.

**New obligations noticed:** (1) The host obligation is unusually well specified
already — `PROP-024 ##HOOK-BUILD` names the variable, its value (the workspace
absolute root), and the section it amends. What is missing before Phase E can
build it is a `project_root` field on `HookContext`, since `build_env` cannot
emit a value the context does not carry; that is the actual first line of the
change and it is not in either PROP. (2) `##P4-MECHANICS-A-LANGUAGE-NATIVE-CONSUMER-MAY-SKIP-THE-HOOK`
(line 140, `@impl/done`, **not mine**) says a language-native consumer may skip
the hook entirely and reference the shipped source through its own toolchain —
which is precisely what vibevm does as a Rust consumer
(`PROP-024 ##NATIVE-CONSUMER-SKIP`). That is the *reason* the hook path has gone
unexercised, and it is worth the boss noting when scoring how urgent the build
is: the mechanic this anchor describes has no consumer today.

---

## F-343 — the consent gate is not missing, it is *unapplied on one path*; only the diff is genuinely absent

**Outcome:** ROUTE-B CANDIDATE — **no edit made**. One of the verdict's two
claimed absences did not survive.
**Anchors:** 0 of 1 moved.
`self-updating-tools.md#S5-MECHANICS-CONSENT-AND-HONESTY` — not edited. Verified
to be a real definition at that file's line 188.
**Files touched:** `none`
**Perimeter searched:** the standing perimeter above, narrowed to the surface
that could carry the mechanic and then widened *inside* it: every consent gate in
the `vvm` command family (`confirm(` · `args.yes` · `--yes` · `-y` · `dry_run` ·
`dry-run` · `assume_yes` · `no_confirm`), **every call site of the two durable
writers** `set_vibevm_home` / `ensure_on_path` — since the clause is about
gating a mutating edit, and the gate must sit above the writer — plus `env.rs`
swept for a diff/preview by any name (`diff` · `preview` · `would write` ·
`would apply` · `plan` · `before/after`), and the `use` command's own argument
struct read in full, because a flag that supplies consent would be declared there
or nowhere.

**What the search found — clause by clause, and the verdict is wrong on one of
the two it calls absent.**

**Clause 1, CONSENT — the mechanism ships, and is applied on four other mutating
paths.** This is the finding that changes the entry:

```console
$ sed -n '439,455p' crates/vibe-cli/src/commands/vvm/mod.rs
/// Confirm a mutating action: `--yes`/unattended skip the prompt; a non-TTY
/// without `--yes` is an error rather than a silent apply.
fn confirm(ctx: &output::Context, yes: bool, prompt: &str) -> Result<bool, VvmError> {
    if yes || ctx.is_unattended() { return Ok(true); }
    if !std::io::stdin().is_terminal() {
        return Err(VvmError::NoTty {
            detail: "no TTY for confirmation; pass `--yes` to proceed unattended".to_string(),
        });
    }
    Ok(Confirm::new().with_prompt(prompt).default(true).interact().unwrap_or(false))
}

$ grep -rn -E "confirm\(|dry_run" crates/vibe-cli/src/commands/vvm/
doctor.rs:103           if args.fix && confirm(ctx, args.yes, "Write shims and put the shim dir on PATH?")?
relocate/mod.rs:304     if args.dry_run {                       # prints the plan, changes nothing
relocate/mod.rs:321     // The removal is irreversible — confirm unless `--yes`/unattended.
relocate/mod.rs:325     && !confirm( … args.yes, … )
remove.rs:102           if !confirm( … args.yes, … )
remove.rs:208           if !confirm( … args.yes, … )
```

The gate the clause asks for — *«a confirm or an explicit yes flag»* — **exists,
with the exact semantics the lesson prescribes**, including the part most tools
get wrong: a non-TTY run without `--yes` is an **error**, never a silent apply.
It is applied on four mutating paths, one of which also carries a real
`--dry-run` that prints the plan first. «CONSENT does not [exist]» is therefore
**a false absence**: what is missing is not the mechanism but its application to
one command.

**And that one command is the ordinary one, which is the real defect.** The two
durable writers have exactly two production call sites in the whole tree:

```console
$ grep -rn -E "set_vibevm_home|ensure_on_path" crates --include='*.rs'   # call sites only
crates/vibe-cli/src/commands/vvm/doctor.rs:106   make_persister(env, shell)?.ensure_on_path(&shim_dir)?;   # gated at :103
crates/vibe-cli/src/commands/vvm/mod.rs:311      persister.set_vibevm_home(&home)?;                        # UNGATED
crates/vibe-cli/src/commands/vvm/mod.rs:312      persister.ensure_on_path(&store.shim_dir())?;             # UNGATED
```

`run_use_cmd` (`mod.rs:291-330`) reaches both writers with no gate, and no flag
exists to ask for one:

```console
$ sed -n '120,132p' crates/vibe-cli/src/cli/vvm.rs
pub struct VvmUseArgs {
    pub selector: String,
    #[command(flatten)] pub kind: ForcedKind,
    /// Print the shell line to `eval` in the current shell instead of
    /// writing the durable environment.
    #[arg(long)] pub eval: bool,
}
```

Three fields, **no `--yes` and no `--dry-run`**. I checked `--eval` specifically
because it is the closest candidate and it is not the thing: its own doc says it
prints the line **instead of** writing the durable environment — a non-mutating
*alternative*, opt-in, not consent on the mutating default. So the verdict's
conclusion — *«the command that ordinarily changes a user's durable environment
is the one without the gate»* — stands, and stands more sharply now that the
gate is known to exist four doors down.

**Clause 2, PRINT THE DIFF — genuinely absent, everywhere:**

```console
$ grep -rn -iE "diff|preview|would (write|apply|add)|plan\b|before/after" crates/vibe-cli/src/commands/vvm/env.rs
(no output)
```

Nothing previews the rc-file or registry edit. The raw material is there and
unused — `set_vibevm_home` / `ensure_on_path` return `Persisted::{Changed,
Unchanged}` (`env.rs:205,219,243,248`), so the writer already knows whether it is
about to change anything — but the answer is produced by writing, not before it.
`relocate --dry-run` (`relocate/mod.rs:304-317`) is the one place a plan is
printed ahead of a mutation, and it is a different command.

**Clause 3, HONESTY — holds, on both persisters:**

```console
$ sed -n '228,233p;259,261p' crates/vibe-cli/src/commands/vvm/env.rs
    fn activation_hint(&self) -> String {
        format!("source `{}` (or open a new shell) to apply now", self.rc_path.display())
    }
    …
    fn activation_hint(&self) -> String {
        "open a new terminal (the registry change reaches new processes)".to_string()
    }
```

printed on the ordinary path at `mod.rs:325-328`. The POSIX and Windows
persisters each say plainly that the change reaches only new shells or new
processes — exactly the clause.

**Which layer has it, if any:** **host driver** for the consent gate itself
(`vvm/mod.rs:441-455`) and for its four applications
(`doctor.rs:103`, `relocate/mod.rs:325`, `remove.rs:102`, `remove.rs:208`), and
for the honesty clause (`env.rs:228-233`, `:259-261`). **Nowhere** for the diff.
**Missing at one call site** for consent — `vvm/mod.rs:311-312`.

**Why this is route (b) and not a demotion.** The same two reasons as F-342, and
one more that is specific to this anchor.

The document is the generic-lessons flow described in F-342's entry — Context →
Law → Mechanics → Symptoms, vocabulary deliberately portable — and it ships no
CLI of its own, so its sentence is false about the *host's* `vibe self use` and
not about anything in its own tree. §3.6 does not let it yield.

The extra reason is that **this is the clearest compliance gap in the batch, not
an absence at all.** The consumer built the mechanism the lesson prescribes,
built it well — `--yes` bypass, unattended bypass, non-TTY hard error — and
applied it to every mutating path it thought of except one. A demotion would
append *«specified, not built»* to a lesson whose mechanism is running four times
in the same command family, and would tell a reader looking for a consent helper
that none exists. The honest record is that the rule is right, the mechanism is
there, and one call site skipped it. That is a bug in `run_use_cmd`, and the
package is not where it gets fixed.

The host says the same thing, at the same strength, and is equally unkept:
`spec/common/PROP-019-version-manager.md:221` `##RULE-CONSENT` — *«consent +
honesty (mutating edits need a confirm / `-y` / `self doctor --fix`, print the
diff, and say the change reaches only new shells)»*, marked `@spec/done`.

**Verdict recommendation, per anchor:**
`##S5-MECHANICS-CONSENT-AND-HONESTY` → **route (b)**, not drift — of the three
clauses, honesty holds, the diff is genuinely absent, and consent is **present
as a mechanism and missing at one call site**, which is compliance rather than
absence. The obligation is a host bug (`run_use_cmd` must gate `set_vibevm_home`
/ `ensure_on_path`) plus a small host build (the diff).

**New obligations noticed:** (1) The host bug is small and worth stating as one
line for the queue: `run_use_cmd` (`crates/vibe-cli/src/commands/vvm/mod.rs:311-312`)
writes the user's durable environment with no confirm and no `--yes`, while
`confirm()` sits at `mod.rs:441` in the same file and four sibling paths use it;
`VvmUseArgs` (`crates/vibe-cli/src/cli/vvm.rs:121-132`) would need a `yes` field.
(2) `spec/common/PROP-019-version-manager.md:221` `##RULE-CONSENT` asserts the
same three clauses and is equally unkept on the same path — a host-side
obligation on a *host* document, outside my edit scope, recorded. (3) The diff
clause has an unusually cheap implementation available: both writers already
return `Persisted::{Changed, Unchanged}`, so the decision «is this a change» is
computed today and simply discarded before the user could act on it.

---

## Batch summary

| id | outcome | anchors touched / total | marker moves |
|---|---|---:|---:|
| F-327 | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-328 | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-287 | **CORRECTED** | 1 / 1 | 0 |
| F-288 | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-244 | ROUTE-B CANDIDATE (PARTIAL: 1 of 2 halves fails) | 0 / 2 | 0 |
| F-342 | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| F-343 | ROUTE-B CANDIDATE | 0 / 1 | 0 |
| **total** | | **1 / 8** | **0** |

**Two of nine clause-level absence claims did not survive re-verification** — a
touch over a fifth, which is the ratio §3.7 predicted:

1. **F-287** — *«the spec recording what verifies it is inverted … the spec marks
   neither»*. The record exists at scale: `#[verifies(...)]` is used **228
   times**, producing **223 edges over 83 spec units**, and the spec-side
   question *"which test verifies this unit"* is answered **from the spec URI**
   by `explain_unit` (`core-ai-native-specmap/src/explain.rs:91-118`), which has
   a test asserting that exact output and a driver at `xtask/src/main.rs:343`.
   Only the *authoring side* of the record differs from the illustration.
2. **F-343's CONSENT clause** — *«CONSENT does not [exist] … no confirm, no
   `--yes`»*. The gate ships with the exact semantics the lesson prescribes,
   including the part most tools get wrong — a non-TTY run without `--yes` is a
   hard error, never a silent apply (`vvm/mod.rs:441-455`) — and is applied on
   **four** mutating paths (`doctor.rs:103`, `relocate/mod.rs:325`,
   `remove.rs:102`, `remove.rs:208`). What is missing is its application to a
   fifth (`run_use_cmd`, `mod.rs:311-312`). That is a compliance gap in one
   consumer function, not an absent mechanism.

**Zero demotions, and the reason is structural rather than a run of luck.** §3.3
closes a `missing-support` by demoting a package that describes something as
**BUILT**. Six of these seven obligations sit on **prompt-only `world` flows that
ship no code at all** — `secrets-hygiene`, `addressable-specs`,
`qualified-naming`, `tool-design-lessons` have no crate, no checker, no artefact
of their own. Every surviving absence is therefore an absence *in the consumer*,
which §3.6 routes to the host and never to the package, under the rule the §7 LOG
recorded for waves 2–4: **a package moves only where its own sentence is false
about something inside its own tree.** The one edit made is the one case that
passes that test in the other direction — a package sentence that was *less true
than the thing it describes*, repaired by addition rather than retraction.

**The one thing in this batch that is not verdict bookkeeping.** F-327/F-328's
re-verification measured the ACL on the sanctioned credential directory, because
on Windows a POSIX mode is not the mechanism. `C:\Users\olegc\.vibe` carries an
**explicit, non-inherited** ACE granting `overwatch\CodexSandboxUsers`
**ReadAndExecute**, with `ContainerInherit, ObjectInherit` so it propagates to the
files inside; the group holds two accounts, `CodexSandboxOffline` and
`CodexSandboxOnline`, and it is **not present on the profile directory**. By
three of our own documents that directory is the single sanctioned at-rest
location for every credential this tooling handles, and «readable only by the
owner» is false of it. That is a live exposure and it should reach the owner
ahead of anything else in this record. No credential file was read, listed or
printed to establish it; the measurement is directory metadata only, and its one
bound — that a per-file `Deny` overriding the inherited grant would not have been
visible at directory grain — is stated in the entry.

**Three sets that must move together, and two of them cross a route the boss does
not own.**

1. **F-327 + F-328 are two of four anchors on one clause**, and the other two are
   already **`confirmed`**: `SECRETS-HYGIENE-PROTOCOL.md#ROW-LAW-NEVER-PERSISTED`
   and `57-flow-secrets-hygiene.md#NEVER-PERSIST-OUTSIDE-THE-SANCTIONED-LOCATION`.
   One clause, one body of evidence, four anchors, two opposite verdicts. The
   already-confirmed pair was confirmed on a reason that names the permission
   qualifier as unsupported and rules for the package anyway — i.e. two of the
   four are already recorded the way this batch recommends.
2. **F-244 is half of a claim whose other half is F-178, on `sync-from-code` —
   an owner route.** F-244 holds the headline (`ref-grammar.md:65`) and the
   summary (`:183`); F-178 holds the body (`:67`, which F-244's own verdict names
   as its root) and the table row (`:52`). Demoting F-244 alone would author a
   self-contradiction two lines wide. §6.1's `##ROUTE-BEFORE-FALSIFIER` arriving
   from the opposite side: honouring the route cut one claim in half.
3. **F-287 is one anchor of five in the `{#graph}` section, and the other four
   are `confirmed`** — including the section lead and `##THESE-ARE-BIDIRECTIONAL-EDGES`.
   `##NO-TOOLING-IS-REQUIRED-TO-BENEFIT`'s verdict says in its own words that the
   `Test:`-line concern was *deliberately filed onto* F-287's anchor, so F-287 is
   an accumulator, not an independent finding.

**What was deliberately not done.**

- **No verdict JSON was written and nothing under `campaigns/packages-2026-09/run/`
  was touched.** Five obligations are recommended for route (b); recording a
  routing decision is the boss's, per §7's exit gate.
- **`vibe progress check` was not run.** Its `--path` is a tree root, not a file,
  and its own `--no-cache` documentation states that the run *"leaves the
  campaign's records and state projections exactly as a warm run would"* — i.e.
  it writes the campaign cache, which this worker is forbidden to touch. The one
  edit was verified by inspection instead: the anchor at line 221 is intact and
  unchanged, the `@impl/done` marker is intact and unchanged, both example fences
  are byte-identical, and no anchor was added or removed.
- **No `git` command that writes was run.** No credential file was read, printed,
  echoed or copied.

**Host obligations this batch opens, in the order they deserve attention.**

1. **The `.vibe` ACL** (F-327/F-328) — a live credential exposure, above.
2. **`run_use_cmd` writes the user's durable environment ungated** (F-343) —
   `crates/vibe-cli/src/commands/vvm/mod.rs:311-312`, while `confirm()` sits at
   `mod.rs:441` in the same file and four siblings use it. A small, contained
   bug with an obvious fix.
3. **Kind validation is specified in two host documents and built in neither**
   (F-244) — `PROP-008 ##KIND-VALIDATION` (`@impl/done`, so the *marker* is wrong
   too) and `spec/design/workspace-and-qualified-naming.md:81`. The prefix is
   already parsed onto `PackageRef.kind` and survives `qualify()`, so the missing
   work is one comparison and one error variant.
4. **Nine spec documents sit over the hard size budget with no checker**
   (F-288) — while the identical trigger runs mechanically on the code side
   (`FileLength { max_lines: 600 }`, `conform.toml:35`) and has caused real
   splits. The gap is a spec-side counterpart to a checker that already exists.
5. **`VIBE_PROJECT_ROOT` needs a `HookContext` field before it can be a
   variable** (F-342) — neither PROP mentions that, and `build_env` cannot emit a
   value the context does not carry. Already scheduled host-side at `@spec/done`.
6. **`PROP-000.md:303` says «chmod-protected»** on a Windows-primary box, where
   the consumer's own `RUNTIME-GUIDE.md:53` already states it correctly in both
   idioms. A one-word host wording fix.
