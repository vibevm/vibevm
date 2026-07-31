# D7f — the tail of the `sync-from-code` route: five small packages, sixteen verdicts, re-measured before any diff is prepared

_Worked 2026-07-31 at `HEAD = 596588fb` (`docs(campaign): two more queue figures
restated…`, 2026-07-31 13:42:53 +0300), 2 198 commits on `main`. Subjects:
`packages/org.vibevm.world/health-audit/v0.1.0/`,
`packages/org.vibevm.world/secrets-hygiene/v0.1.0/`,
`packages/org.vibevm.world/licensing/v0.1.0/`,
`packages/org.vibevm.world/dev-runtime-docs/v0.1.0/`,
`packages/org.vibevm.world/redbook/v0.2.0/`. Seven obligations, **16 drift
verdicts**, all on the `sync-from-code` route:_

| id | type | falsifier | anchors | package |
|---|---|---|---:|---|
| F-097 | `reality-mismatch` | mixed | 4 | `health-audit` |
| F-203 | `reality-mismatch` | host | 3 | `secrets-hygiene` |
| F-330 | `reality-mismatch` | host | 1 | `secrets-hygiene` |
| F-236 | `contradiction` | mixed | 2 | `licensing` |
| F-239 | `reality-mismatch` | mixed | 2 | `licensing` |
| F-227 | `reality-mismatch` | host | 2 | `dev-runtime-docs` |
| F-114 | `contradiction` | mixed | 2 | `redbook` |

**Nothing in this record was edited.** No file under `packages/` was touched, no
verdict JSON was written, nothing under `campaigns/packages-2026-09/run/` was
modified, and no `git` command that writes was run. **This route's whole
economy is that a re-judge which edits nothing produces no spec diff and
therefore needs no owner approval** — an edit would destroy that, so the
deliverable is evidence and a recommendation, and the verdict is the boss's.

**No credential file was read, opened, listed, printed or copied.** The
`secrets-hygiene` entries reason about paths, ACLs and code only. The ACL
exposure on `~/.vibe` established by wave 6 is not re-derived here and no
permission was changed.

## What this route asks, and what makes a verdict on it false

`sync-from-code` obligations are overwhelmingly **`reality-mismatch`**: the fact
*describes* something that exists and describes it wrongly — a path, a count, a
roster, a version, a name, a signature, a behaviour. The defect is a
**discrepancy**, not an absence, so the question is not «does it exist» but
**«is the description actually wrong, today, measured?»**

Four ways a verdict on this route turns out false, all four paid for in wave 6
and all four checked here:

1. **The number moved, or was never right.** Wave 6 re-measured two recorded
   figures and both were wrong, one by roughly seventy. **Every count in this
   record was re-measured, and every one names the HEAD it was measured at.**
2. **The searched perimeter was wrong** (§3.7 in both directions — see below).
3. **The verdict's own command does not reproduce.** Wave 6 caught three
   verdicts this way at near-zero cost. **Where a verdict quotes a command, it
   is re-run verbatim below before anything else.**
4. **The fact and the evidence are about different things** — a rename, an
   alias, a roster of a different population, a README mistaken for a boot
   snippet.

**Consistency propagates an error** (§3.7's corollary): where a verdict says it
was restated to match its siblings, the whole set is re-verified, not the row.
Three of this batch's seven obligations are explicitly restatement sets
(F-097's four anchors, F-236/F-239's summary rows, F-114's causal clause), and
each is judged as a set below.

## The perimeter

Run from the repository root, and it must contain **every project that adopted
the discipline, wherever it sits** — this repository holds at least two, the
host and the `fractality` specspace inside `packages/`:

```
packages/**  (INCLUDING packages/org.vibevm.fractality/**)
vibedeps/**  crates/**  xtask/**  tools/**  spec/**  discipline/**  terraform/**
research/**  campaigns/**  legacy-spec/**  fixtures/**  schemas/**  docs/**  manual-tests/**
and the repository root's own *.md / *.toml / *.json / *.sh / *.ps1
minus  **/target/**  .git/**  **/node_modules/**  campaigns/*/run/**
```

`refs/**` is searched but reported **separately**: it is a third-party study
corpus, not our shipped surface, and a hit there is not an implementation of
ours.

**A `not-found` is a fact about the search perimeter until the perimeter has
been checked**, and **an `##ANCHOR` inside backticks is a citation, not a
definition** — every anchor below was verified to be a real definition at a
named `file:line` before being judged.

---

## F-097 — the breach is real and now measurably wider, but all four sentences are prescriptions, and the second consumer breaks them too

**Outcome:** SURVIVES — ROUTE (b), 4/4 · **one recorded number moved; one description checked out as accurate**
**Anchors:** 4 of 4, by name:
`42-flow-health-audit.md#AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR` — route (b) (definition at that file's **line 21**)
`42-flow-health-audit.md#A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE` — route (b) (**line 24**)
`42-flow-health-audit.md#USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE` — route (b) (**line 44**)
`42-flow-health-audit.md#NEVER-DECLARE-A-MILESTONE-DONE-ON-AN-UN-AUDITED-BASE` — route (b) (**line 57**)
All four verified to be real definitions, not citations.
**Perimeter searched:** the standing perimeter, **specifically including
`packages/org.vibevm.fractality/fractality/v0.1.0/`** — which turns out to be a
second consumer of this exact flow — for `Audit run —` · `AUDIT.md` ·
`health-audit` · `skills/health-audit` · `[[skill]]`, plus every harness skill
home in both projects and the `ROADMAP.md` milestone table.

**The verdict's own commands, re-run:**

```console
$ git log --oneline --since=2026-06-12 | wc -l
1659

$ grep -nE "^## " AUDIT.md
20:## Audit run — 2026-05-23 (seed)
154:## Audit run — 2026-06-10 (terraform close-out, instrumented category C)
191:## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)

$ find . -maxdepth 4 -path '*skills/health-audit*'
(no output)

$ grep -rn 'health-audit' .claude/
(no output — exit 1)
```

**The one number that moved: 1 546 → 1 659.** The verdict recorded «across
1 546 commits»; at `HEAD = 596588fb` the same command returns **1 659**, 113
more. Nothing else about the floor measurement changed — still three dated
audit runs, still last on 2026-06-12, still the two most recent `ROADMAP.md`
ship lines after it:

```console
$ grep -nE "^### M[0-9.]+ .*SHIPPED" ROADMAP.md | tail -3
661:### M1.26 — MCP sovereignty (the `mcp` kind + standalone discipline servers) — SHIPPED (2026-07-07)
707:### M1.24 — the agentic tcg line (`vibe-agentic-tcg-ts`) — SHIPPED (2026-07-07)
938:### M2.10 — `vibe search` registry inspector — ✅ SHIPPED (2026-05-22)
```

Both `SHIPPED (2026-07-07)`, both after the last audit, with no audit section
between. **The breach is real and is 19 days older than when it was recorded.**

**What the measurement shows, per clause.**

*The trigger half is accurate and stays accurate.* `AUDIT.md:193` records the
2026-06-12 run as owner-requested with the instruction quoted, and no agent
contract carries an audit trigger phrase. Nothing here is wrong.

*The floor half is breached — by both consumers.* This is the §3.7 perimeter
check, and it went the way that widens the finding rather than the way that
kills it. `packages/org.vibevm.fractality/fractality/v0.1.0/` is the second
project that adopted this discipline: it carries `flow-health-audit` in its own
`vibedeps/` (its `vibe.lock:30` reads
`"flow:org.vibevm.world/health-audit@=0.1.0"`) and compiles this very snippet
into its own boot lane —

```console
$ sed -n '56,58p' packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/INDEX.md
path = "vibedeps/flow-health-audit/0.1.0/spec/boot/42-flow-health-audit.md"
kind = "static"
```

— and it has **no `AUDIT.md` at all** (`ls AUDIT.md` in that tree fails).
Across the whole perimeter there is exactly one live audit record, the host's:

```console
$ grep -rln "Audit run —" --include='*.md' .          # .git excluded
./AUDIT.md
./campaigns/packages-2026-09/harvest/d2-wal-audit-manual-repairs.md
./campaigns/packages-2026-09/harvest/d6c-mirrors-licensing-absences.md
./campaigns/packages-2026-09/harvest/world-w5-project-practice-i.md
```

(the three campaign files are this campaign's own records quoting the host, not
audit runs). So the second consumer does not falsify the verdict; it is a
**second non-adopter of the same rule, booting the same sentence**.

*The skill clause: the description of the skill is accurate — what fails is a
consumer projection step, and it fails for six skills, not one.* The snippet
says the skill «reads the category checklist, walks it against the repository,
and drafts the `AUDIT.md` section for your approval». Read against
`packages/org.vibevm.world/health-audit/v0.1.0/spec/skills/health-audit/SKILL.md`
that is **exact**: step 1 reads `audit-checklist.md` and `running-an-audit.md`
in full (`SKILL.md:21-23`), step 4 walks the checklist breadth-first against the
repository (`SKILL.md:28-33`), the Output section produces a draft `AUDIT.md`
section (`SKILL.md:41-42`), and the Do-not section forbids committing before the
owner approves (`SKILL.md:48-49`). The skill ships and installs — it exists at
that path and at `vibedeps/flow-health-audit/0.1.0/spec/skills/health-audit/SKILL.md`,
and `packages/org.vibevm.world/health-audit/v0.1.0/vibe.toml:19-22` declares it
as a `[[skill]]` named `health-audit`.

What is missing is the *projection*, and it is not specific to this package:

```console
$ grep -rn -A2 '^\[\[skill\]\]' packages/org.vibevm.*/*/*/vibe.toml | grep 'name = '
org.vibevm.world/health-audit/v0.1.0/vibe.toml-20-name = "health-audit"
org.vibevm.world/licensing/v0.1.0/vibe.toml-20-name = "draft-eula"
org.vibevm.world/wal/v0.2.0/vibe.toml-20-name = "wal-status"
org.vibevm.ai-native/go-ai-native-lang/v0.1.0/vibe.toml-50-name = "go-ai-native-sweep"
org.vibevm.ai-native/go-ai-native-lang/v0.1.0/vibe.toml-55-name = "go-ai-native-terraform"
org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibe.toml-50-name = "rust-ai-native-sweep"
org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibe.toml-55-name = "rust-ai-native-terraform"
org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/vibe.toml-50-name = "typescript-ai-native-sweep"
org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/vibe.toml-55-name = "typescript-ai-native-terraform"
org.vibevm.fractality/fractality/v0.1.0/vibe.toml-36-name = "fractality-delegate"
                                                     (10 declared, across 7 packages)

$ ls -1 .claude/skills/          $ ls -1 .agents/skills/        $ ls -1 .opencode/skills/
rust-ai-native-sweep             rust-ai-native-sweep           rust-ai-native-sweep
rust-ai-native-terraform         rust-ai-native-terraform       rust-ai-native-terraform
typescript-ai-native-sweep       typescript-ai-native-sweep     typescript-ai-native-sweep
typescript-ai-native-terraform   typescript-ai-native-terraform typescript-ai-native-terraform
vibevm                                                (5)                          (4)   (4)
```

**Six of the ten package-declared skills are projected into no harness home** —
`health-audit`, `draft-eula`, `wal-status`, `go-ai-native-sweep`,
`go-ai-native-terraform`, `fractality-delegate`. The host has run
`vibe skill install` for the rust and typescript stacks and for nothing else.
`vibe.lock:307` records `files_written = []` for this package, which is correct:
projection is a separate command (`##CMD-SKILL-INSTALL`,
`spec/common/PROP-018-agentic-standalone-modes.md:212`), not an install effect.
So the instruction does fail when followed here — and it fails for the same
reason five sibling instructions would.

**Is this §3.6(c), a marked exception?** No. I searched `CLAUDE.md`,
`AGENTS.md`, `GEMINI.md`, `DEV-GUIDE.md`, `RUNTIME-GUIDE.md`, `README.md` and
`spec/**` for a recorded decision not to project skills; the only hits are
PROP-018's specification of the command itself
(`PROP-018-agentic-standalone-modes.md:203-217`). An unmarked omission, which
Phase C's own ruling calls drift on the consumer's side — not a package defect.

**Why all four are route (b), decided by reading the sentences rather than the
evidence.** Every one is a prescription, and three say so grammatically:

- line 21 sits under `## When it fires {#when}` and its own words are «a
  **floor** of **at least once per milestone** — run as part of, or right after,
  a milestone close-out». A floor is this flow's word for a minimum, and «run …»
  is imperative.
- line 24: «A milestone **is never** declared done on an un-audited base.»
- line 57 is under `## Never {#never}`: «**Never** declare a milestone done on
  an un-audited base — the audit is part of the close-out, not an optional
  extra.»
- line 44 is an instruction («**Use** the … skill») whose descriptive tail was
  measured accurate above.

§3.6 does not let a prescription yield to a consumer that simply does not keep
it, and the wave-2–4 rule in the §7 LOG is the same test from the other side: *a
package moves only where its own sentence is false about something inside its
own tree.* `health-audit` is a prompt-only `world` flow with no crate; its own
tree holds a protocol, a checklist, a run procedure and a skill, and this batch
found nothing in it that contradicts these four sentences. What is false is
**two consumers' practice**, and neither of them is this package.

**Proposed correction (NOT APPLIED):** none — correct as written. The stale
figure lives in the *verdict* (1 546), not in the document.

**Recommendation per anchor:**
`##AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR` → drift stands, route (b)
`##A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE` → drift stands, route (b)
`##USE-THE-HEALTH-AUDIT-SKILL-TO-RUN-ONE` → drift stands, route (b)
`##NEVER-DECLARE-A-MILESTONE-DONE-ON-AN-UN-AUDITED-BASE` → drift stands, route (b)

**Host obligations this opens (recorded, not acted on).** (1) Two milestones —
M1.26 and M1.24, both `SHIPPED (2026-07-07)` — were declared done on an
un-audited base, and the gap is now 1 659 commits. (2) Six of ten
package-declared skills are projected into no harness home, so six boot-lane
instructions fail when followed; `vibe skill install` is the one command that
closes all six. (3) The `fractality` specspace boots this flow and keeps no
`AUDIT.md` — the same obligation, in the second project.

---

## F-203 — the verdict concedes on one anchor and rules drift anyway; on the other two it reproduces exactly and turns out sharper than recorded

**Outcome:** MIXED — 1/3 FALSE, 2/3 SURVIVE — ROUTE (b)
**Anchors:** 3 of 3, by name:
`third-party-code-consent.md#GATE-ALLOW-LISTED-PUBLISHERS-RUN-SILENTLY` — **SURVIVES — ROUTE (b)** (definition at that file's **line 41**)
`third-party-code-consent.md#GATE-EVERYONE-ELSE-GETS-FIRST-RUN-CONSENT` — **SURVIVES — ROUTE (b)** (**line 46**)
`third-party-code-consent.md#the-prompt-points-at-a-real-path` — **FALSE** (**line 108**)
All three verified to be real definitions, not citations.
**Perimeter searched:** the standing perimeter, and deliberately **off the
verdict's string** for the second half: a configurable hook-trust list can ship
under any name, so I searched `allow[_-]?hooks` · `allow[_-]?list` · `allowlist`
· `trusted[_-]?groups` · `hook[_-]?trust` · `trust[_-]?policy` ·
`allowed_groups` · `DEFAULT_ALLOWED_GROUPS` across `crates xtask tools schemas
fixtures docs manual-tests discipline terraform` over `*.rs *.toml *.json *.md
*.sh *.ps1`, then read the **config type itself** (`UserConfig`), the **loader**
(`load_from`), and the **hook path resolver** (`select_invocation`) — because a
list that is "configured" must have a schema, and a prompt that "can point at a
real path" must have a resolver.
**No credential file was read, opened, listed or printed.**

**The verdict's own commands, re-run — all three reproduce exactly:**

```console
$ grep -rn "allowed_groups" crates/
crates/vibe-cli/src/commands/install/mod.rs:268:    let mut allowed_groups = allowed;
crates/vibe-cli/src/commands/install/mod.rs:269:    allowed_groups.extend(consented);
crates/vibe-cli/src/commands/install/mod.rs:271:        allowed_groups,
crates/vibe-install/tests/incremental_in_place.rs:187:        allowed_groups: vec!["org.vibevm".to_string()],
crates/vibe-workspace/src/hooks/tests.rs:103:        allowed_groups: vec!["org.vibevm".to_string()],
crates/vibe-workspace/src/hooks/tests.rs:112:        allowed_groups: Vec::new(),
crates/vibe-workspace/src/hooks.rs:126:         fix: allow-list `{group}` in [hooks].allowed_groups or pass --allow-hooks)"
crates/vibe-workspace/src/hooks.rs:299:    pub allowed_groups: Vec<String>,
crates/vibe-workspace/src/hooks.rs:311:        if self.allow_hooks || self.allowed_groups.iter().any(|g| g == group.as_str()) {
crates/vibe-workspace/src/install/tests_hooks.rs:72:        allowed_groups: vec!["org.vibevm".to_string()],

$ grep -rn "DEFAULT_ALLOWED_GROUPS" crates/ xtask/ tools/
crates/vibe-workspace/src/hooks.rs:26:pub const DEFAULT_ALLOWED_GROUPS: &[&str] = &["org.vibevm"];
crates/vibe-cli/src/commands/install/mod.rs:32,213,224   (import, doc, the Vec built from it)
```

Ten hits, exactly the population the verdict described: the constant, its
tests, the vector built from that constant plus this-run consents, and the
struct field. **No config reader anywhere.**

**What the measurement shows — and on the allow-list it is sharper than the
verdict recorded, in a way that matters to whoever repairs it.**

`UserConfig` (`crates/vibe-core/src/user_config.rs:64-89`) has exactly three
fields — `env` (`:78`), `install` (`:82`), `init` (`:89`) — and no `hooks`, as
the verdict says. What the verdict did not reach is the attribute one line
above the struct:

```console
$ sed -n '62,64p' crates/vibe-core/src/user_config.rs
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {

$ sed -n '204,217p' crates/vibe-core/src/user_config.rs        # the loader
    let cfg: UserConfig = toml::from_str(&body).map_err(|source| UserConfigError::Parse {
        path: path.to_path_buf(), source,
    })?;
```

`deny_unknown_fields` plus a `?` on the parse means an operator who writes
`[hooks].allowed_groups` into `~/.vibe/config.toml` does not get a silently
ignored key — **they get `UserConfigError::Parse` and their whole user config
stops loading.** And two shipped error messages instruct them to do exactly
that:

```console
$ sed -n '126p' crates/vibe-workspace/src/hooks.rs
         fix: allow-list `{group}` in [hooks].allowed_groups or pass --allow-hooks)"

$ sed -n '259,264p' crates/vibe-cli/src/commands/install/mod.rs
            HookTrust::Refused => { bail!(
                "package group `{group}` declares install hooks but is not trusted to run \
                 them non-interactively (PROP-020 §2.3). Re-run interactively to consent, \
                 allow-list `{group}`, or pass `--allow-hooks`." ) }
```

So the fix surface the tool prints for its own hardest failure is a config key
whose presence is a parse error. The CLI's own help text states the real state
plainly — `crates/vibe-cli/src/cli/pkg.rs:285`: «`org.vibevm` is always
allow-listed and runs regardless.» The **silent-run arm itself works** —
`decide_trust` at `hooks.rs:311` returns `Allowed` when the group matches and
the caller falls straight through (`install/mod.rs:245`, `HookTrust::Allowed
=> {}`) — so the sentence's headline («Allow-listed publishers run silently»)
is built. What has no mechanism is «**A configured list** … goes on the list».

**The prompt, verbatim, and it is exactly what the verdict quoted:**

```console
$ sed -n '246,256p' crates/vibe-cli/src/commands/install/mod.rs
            HookTrust::NeedsConsent => {
                let ok = Confirm::new()
                    .with_prompt(format!(
                        "Package group `{group}` declares install hooks (PROP-020). \
                         Run them during this install?"
                    ))
                    .default(false)
                    .interact()
```

Group only — no phase, no script path. The decline half of that anchor **is**
met: a declined group is left out of the policy and the hook returns
`skipped-needs-consent` (`crates/vibe-workspace/src/hooks.rs:409-413`), a status
the report renders (`hooks.rs:113`), so a skipped hook is never silent.

**And now the anchor that is FALSE, on the verdict's own concession.**
`##the-prompt-points-at-a-real-path` (line 108) reads:

> The consent prompt **can** point at a real path, and the reviewer can open it.

It is a **modal capability claim**, and it is the payoff sentence of the section
it lives in — `## Hooks are versioned files, not inline strings {#versioned-files}`,
whose whole argument is that *because* a hook is a versioned file referenced by
path, a prompt has something openable to point at. The capability exists and I
verified the resolver:

```console
$ sed -n '327,353p' crates/vibe-workspace/src/hooks.rs
pub fn select_invocation(slot: &Path, base: &Path, platform: Platform, probe: &dyn InterpreterProbe)
    -> Option<HookInvocation> {
    let sh  = slot.join(base).with_extension("sh");
    let ps1 = slot.join(base).with_extension("ps1");
    …  Platform::Unix => sh_via_bash(),
       Platform::Windows => sh_via_bash().or_else(|| (ps1.is_file() && probe.has("powershell")) …
```

carrying `#[spec(implements = "spec://vibevm/modules/vibe-workspace/PROP-020#script-selection")]`
at `hooks.rs:323-326`. The manifest value is a genuine base path and it resolves
to a concrete file on disk, which is precisely and only what the sentence
claims.

The verdict says so itself, in its own first clause: *«The CAPABILITY is real —
the manifest value is a genuine base path and `select_invocation` resolves it to
a concrete file — so the anchor's claim that a prompt can point at something
openable is true of the design.»* It then rules `drift` because the capability
is unused. **That is the fourth failure mode in §3.7's list — the fact and the
evidence are about different things.** The claim «a prompt *can* point at a real
path» and the claim «the prompt *does* print the script path» are two
sentences, and the second one is `##GATE-EVERYONE-ELSE-GETS-FIRST-RUN-CONSENT`
(line 46), which carries its own drift verdict in this same obligation. One
defect, correctly filed once; the capability sentence was convicted of its
neighbour's offence.

**Why the other two are route (b) and not package defects.** `secrets-hygiene`
is a prompt-only `world` flow with no crate; it ships no installer, no config
type and no prompt. Both sentences sit under `## The consent gate
{#consent-gate}` whose lead (`##consent-gate-lead`, line 39) is *«Trust is
governed cheaply by an **allow-list plus first-run consent**:»* — the flow
specifying how the gate works, i.e. a prescription. And the host states the
identical rule at the identical strength and is equally unbuilt, at `@impl/done`
on both:

```console
$ sed -n '123,131p' spec/modules/vibe-workspace/PROP-020-install-hooks.md
- ##ALLOW-LIST **Allow-listed groups run silently.** A config key (global
  `~/.vibe/config.toml` `[hooks].allowed_groups`, with a project-level
  override) lists trusted package groups. … @impl/done
- ##FIRST-RUN-CONSENT **Other groups need consent.** On the first hook run of a non-allow-listed
  package, vibevm prints what will run (phase, script path, group) and asks
  `y/n`. … @impl/done
```

Under the §7 LOG's wave-2–4 rule — *a package moves only where its own sentence
is false about something inside its own tree* — nothing in this package's tree
contradicts either sentence. What is false is the **host's implementation**,
which also asserts the unbuilt behaviour in its own PROP at `@impl/done`.

**`refs/**` — searched, reported separately: no hit is an implementation of
ours.**

**Proposed correction (NOT APPLIED):** none for any of the three. Two are
prescriptions the consumer under-implements; the third is accurate as written.

**Recommendation per anchor:**
`##the-prompt-points-at-a-real-path` → **re-judge confirmed** — the sentence is
a modal capability claim, the capability is built and spec-marked at
`crates/vibe-workspace/src/hooks.rs:323-353`, and the verdict concedes it in its
own text before convicting the sentence of its neighbour's defect.
`##GATE-ALLOW-LISTED-PUBLISHERS-RUN-SILENTLY` → drift stands, route (b)
`##GATE-EVERYONE-ELSE-GETS-FIRST-RUN-CONSENT` → drift stands, route (b)

**Host obligations this opens (recorded, not acted on).** (1) The two shipped
error messages at `crates/vibe-workspace/src/hooks.rs:126` and
`crates/vibe-cli/src/commands/install/mod.rs:263` name `[hooks].allowed_groups`
as the fix, and `UserConfig`'s `deny_unknown_fields`
(`crates/vibe-core/src/user_config.rs:63`) turns following that instruction into
a config-load failure. That is worse than a missing feature: the tool's own
guidance breaks the operator's config. (2) `PROP-020 ##ALLOW-LIST` and
`##FIRST-RUN-CONSENT` are both `@impl/done` over behaviour no code performs — a
host marker defect on a host document. (3) The consent decision is per **group**,
keyed on a `BTreeSet` (`install/mod.rs:240-243`), so one answer covers every
hook-declaring package in that group — a granularity neither document states.

---

## F-330 — the gap is real, is twice as wide as recorded, and the sentence that names it is the one anchor of seven the section elected to carry it

**Outcome:** SURVIVES — ROUTE (b) · the verdict understates the gap by one adapter and misses the host-internal contradiction underneath it
**Anchors:** 1 of 1, by name:
`scope-discipline.md#the-check-runs-on-every-action-lead` — **SURVIVES — ROUTE (b)** (definition at that file's **line 61**, a lead-in to the four-bullet list at lines 63-66 — a real definition, not a citation)
**Perimeter searched:** the standing perimeter, narrowed to the surface that can
carry the guard and then widened inside it: **every** `RepoCreator` method on
**both** shipped adapters (`github.rs`, `gitverse.rs`) plus the trait's default
impls (`creator.rs`), **every call site of `push_url` in the tree**, and the
ordering of each call site against the nearest guarded call — because a claim
about *every action* is a claim about a set of code paths, not about a string.
**No credential file was read, opened, listed or printed.** The token value is
never reproduced below; only the shape of the URL that embeds it.

**The verdict's own command, re-run:** the verdict quotes no command; it quotes
two file:line facts, and both reproduce:

```console
$ sed -n '124,126p' crates/vibe-cli/src/commands/registry/publish.rs
    // per PROP-000 §20. Each adapter additionally validates the org
    // at every method call as defence in depth.

$ grep -n "fn host_name\|fn repo_exists\|fn create_repo\|fn push_url" crates/vibe-publish/src/github.rs
148:    fn host_name(&self) -> &str {
156:    fn push_url(&self, org: &str, name: &str) -> String {
173:    fn repo_exists(&self, org: &str, name: &str) -> Result<bool, PublishError> {
206:    fn create_repo(
```

`repo_exists` guards at `github.rs:174` and `create_repo` at `:212`, each as
the **first statement**, before the URL is even built. `push_url`
(`github.rs:156-171`) contains no `validate_scope` call.

**What the measurement shows — three things the verdict did not reach.**

**(1) It is two adapters, not one.** `RepoCreator` has two production
implementations and **neither** guards `push_url`:

```console
$ sed -n '135,140p' crates/vibe-publish/src/gitverse.rs
    fn push_url(&self, org: &str, name: &str) -> String {
        // GitVerse uses SSH for pushes — the user's SSH agent / key handles
        // authentication. The token is API-only, never embedded in URL.
        format!("git@{}:{}/{}.git", self.host_name, org, name)
    }
```

The GitVerse adapter interpolates `org` into an SSH remote with no scope check,
exactly as the GitHub one interpolates it into a credentialed HTTPS remote. So
«every action» is two-of-three **per adapter, twice**.

**(2) `push_url` cannot return a scope error without a signature change** — and
this is what makes the repair non-trivial:

```console
$ sed -n '141p' crates/vibe-publish/src/creator.rs
    fn push_url(&self, org: &str, name: &str) -> String;
```

It returns `String`, not `Result<String, PublishError>`. There is no channel
for `ScopeViolation` to travel down. Fixing this is a trait-signature change
plus two impls plus four call sites — not the one-line insertion the other two
methods needed.

**(3) The host contradicts itself about this, and the code follows the narrower
statement.** The comment the verdict quotes says the guard runs «at **every
method call**». The trait's own doc, sixteen lines above the guard, says
something different:

```console
$ sed -n '152,156p' crates/vibe-publish/src/creator.rs
    /// Refuse operations addressed to an org other than this adapter's
    /// configured scope. Default impl uses [`expected_org`](Self::expected_org).
    /// Concrete impls call this from `repo_exists` / `create_repo`
    /// before any side-effecting work.
    fn validate_scope(&self, org: &str) -> Result<(), PublishError> {
```

**«from `repo_exists` / `create_repo`»** — two methods, named. The code
implements the trait doc faithfully; it is `publish.rs:125-126` that overstates
it. That is a host-internal contradiction, and it is the more precise defect
than «one method is missing a call».

**(4) The «protected by sequence» defence holds only for the *use*, not the
construction — and on half the call sites the construction comes first.** Four
call sites:

```console
$ grep -rn "push_url(" crates/ --include='*.rs' | grep -v "fn push_url"
crates/vibe-cli/src/commands/registry/redirect/create.rs:239   (after create_repo at :230 — guarded first)
crates/vibe-cli/src/commands/registry/redirect/sync.rs:96      (BEFORE repo_exists at :99)
crates/vibe-cli/src/commands/registry/redirect/update.rs:96    (BEFORE repo_exists at :98)
crates/vibe-publish/src/orchestrator.rs:273                    (after repo_exists / create_repo — guarded first)
```

On `sync.rs` and `update.rs` the credentialed URL is **built** before any
guarded call runs. It is not *used* until after the guard passes
(`sync.rs:116`, `update.rs:111`), so no request reaches a wrong org today — but
the ordering means the sequence protection is an accident of statement order in
two of four sites, not a property anything enforces. A reordering during
refactor would silently remove it.

**Why this is route (b) and not a package defect, decided by reading the
sentence and then its six siblings.** The flow is prompt-only `world` — it
ships no adapter, no trait, no publish path. The sentence sits in a normative
run: `##THE-CHECK-IS-A-GUARD-AT-THE-BOUNDARY` (line 58) → `It runs on every
action:` (line 61) → the four bullets (63-66) → `##AN-UNGUARDED-CODE-PATH-IS-A-BUG`
(line 68), whose own words are *«A code path that reaches a host endpoint
without passing the check is **a bug**, caught in review»*. The flow does not
report what the consumer does; it says what an unguarded path **is**.

**And the consistency corollary fires here more decisively than anywhere else
in this batch.** Six of the seven anchors in this section are `confirmed`, and
**two of them say in their own verdict text that the `push_url` failure was
deliberately filed onto this row**:

```
THE-CHECK-IS-A-GUARD-AT-THE-BOUNDARY   confirmed  "the guard placement is literally the first statement of each boundary method"
the-check-runs-on-every-action-lead    drift      (F-330)
GUARDED-ACTION-CREATE                  confirmed  "create_repo calls the guard as its first statement (github.rs:212)"
GUARDED-ACTION-MODIFY                  confirmed  "…the other route is `git push` via `push_url`, which is unguarded and is
                                                   CARRIED AS DRIFT AT THE EVERY-ACTION ROW rather than counted twice here"
GUARDED-ACTION-DELETE                  confirmed  "vibevm implements NO delete … the guard cannot run on an action that does not exist"
GUARDED-ACTION-PROBE                   confirmed  "the probe is `repo_exists`, guarded first-statement at github.rs:174"
AN-UNGUARDED-CODE-PATH-IS-A-BUG        confirmed  "confirmed BY the failure it classifies … the failure itself is carried by
                                                   `the-check-runs-on-every-action-lead`"
```

So this anchor is the section's **designated accumulator**, exactly as F-287
turned out to be in wave 6 — one sub-claim filed onto one row by explicit Phase
C decision. It is not an independent finding, and it cannot be re-judged in
isolation from the six confirmations that lean on it. Most tellingly,
`##AN-UNGUARDED-CODE-PATH-IS-A-BUG` was ruled **confirmed *because* the host has
an instance of what it names** — which is the package being right about the
consumer, stated in the campaign's own record.

**Proposed correction (NOT APPLIED):** none — correct as written. The four
enumerated actions are the flow's own definition of «every action», and all four
bullets are already `confirmed`.

**Recommendation per anchor:**
`##the-check-runs-on-every-action-lead` → **drift stands, route (b)** — the
unguarded path is real, is two adapters wide, and belongs to the consumer;
the sentence is a prescription whose six siblings are confirmed and two of
which name this row as the carrier of exactly this failure.

**Host obligations this opens (recorded, not acted on).** (1) `push_url` is
unguarded on **both** adapters (`crates/vibe-publish/src/github.rs:156`,
`crates/vibe-publish/src/gitverse.rs:135`) and its trait signature
(`crates/vibe-publish/src/creator.rs:141`) returns `String`, so closing it is a
signature change across the trait, two impls and four call sites. (2)
`crates/vibe-cli/src/commands/registry/publish.rs:125-126` claims the guard runs
«at every method call»; `crates/vibe-publish/src/creator.rs:154-155` says it
runs «from `repo_exists` / `create_repo`». Two host statements, one code
behaviour — a host-internal contradiction to settle before either is trusted.
(3) On `redirect/sync.rs:96` and `redirect/update.rs:96` the credentialed URL is
constructed before the first guarded call; nothing enforces that ordering.

---

## F-236 — the contradiction is still there 19 days on, word for word; and the «cannot fail by construction» half of the second verdict turns out to have already failed

**Outcome:** SURVIVES — ROUTE (b), 2/2 · with a **FALSE PREMISE inside the second anchor's reasoning**, naming a defect nobody has filed
**Anchors:** 2 of 2, by name:
`LICENSING-PROTOCOL.md#THESE-MUST-NEVER-DISAGREE` — **SURVIVES — ROUTE (b)** (definition at that file's **line 112**)
`LICENSING-PROTOCOL.md#SUM-KEEP-EVERY-STATEMENT-IN-SYNC` — **SURVIVES — ROUTE (b)**, and its verdict's manifest-half reasoning is false (**line 169**)
Both verified to be real definitions, not citations.
**Perimeter searched:** the standing perimeter for `EULA` · `UPL-1.0` ·
`license` · `license-file`, over `*.md` and **every** `Cargo.toml` and
`vibe.toml` in the tree — and then, because the verdict's defence of the
manifest half is a *universal* claim («all … point at `LICENSE.md` … cannot
fail»), the workspace member list was enumerated and **each member checked for
the presence of a licence declaration**, which a grep for the string can never
do. A universal claim is falsified by a manifest that says nothing, and nothing
is not a string.

**The verdict's own facts, re-checked at `HEAD = 596588fb` — every one
reproduces verbatim:**

```console
$ sed -n '3p' LICENSE.md
The Universal Permissive License (UPL), Version 1.0

$ grep -n -i "EULA" README.md
164:vibevm itself ships under the proprietary EULA placeholder in [`LICENSE.md`](LICENSE.md)
    for the moment; the eventual target is UPL 1.0. …
```

**Nineteen days after the relicense the README still names the other licence,
and still links to the file that contradicts it.** Nothing in this entry is a
stale reading: both lines are as the verdict found them, at today's HEAD.

**What the measurement shows.**

*The exception ledger still does not cover the README, and it correctly covers
its neighbour.* `CLAUDE.md:132-137` enumerates the surviving `"EULA"` strings
that are deliberately off-limits — `refs/**`, `vibedeps/**` + `.vibe/cache/**`,
`fixtures/**` + `crates/**` test data, the `licensing` package's own template,
and `VIBEVM-SPEC.md` + specs. `README.md` is on none of them.
`VIBEVM-SPEC.md:8` says the same stale thing («proprietary EULA
(source-available)») and **is** on the list, so the ledger distinguishes the two
cases correctly and the README was simply missed. That is §3.6(c) working —
a marked exception is not drift — and it is precisely why the unmarked one is.

*The package manifests are clean; the sixteen that still say `EULA` are all
inside the two categories the ledger exempts:*

```console
$ grep -rhn '^license' --include='vibe.toml' packages/ vibe.toml | sed 's/^[0-9]*://' | sort | uniq -c
     16 license = "EULA"
    158 license = "UPL-1.0"

$ grep -rn '^license = "EULA"' --include='vibe.toml' packages/ vibe.toml
   …/fractality/v0.1.0/.vibe/cache/…          (8 hits)
   …/fractality/v0.1.0/vibedeps/…             (4 hits)
   …/delegation-rules/v0.1.0/{.vibe/cache,vibedeps}/…   (the other 4)
```

All sixteen are regenerated dependency copies inside the `fractality`
specspace's own `.vibe/cache/` and `vibedeps/` — exactly the two categories
`CLAUDE.md:131-133` declares off-limits. **No canonical package manifest names
the old licence.** (This is the second-consumer perimeter paying off in the
direction that *narrows* a finding rather than widening it.)

**And now the false premise, which is the one genuinely new thing in this
entry.** `##SUM-KEEP-EVERY-STATEMENT-IN-SYNC`'s verdict rests half its case on:

> «The manifest half is in sync BY CONSTRUCTION and cannot fail: all 18
> declaring manifests point at `LICENSE.md` via `license-file` rather than
> naming a licence, so they move when the file moves.»

The 18 reproduce — but 18 is not the population:

```console
$ grep -rn "^license" --include='Cargo.toml' Cargo.toml crates/ xtask/ tools/
Cargo.toml:55:license-file = "LICENSE.md"
crates/progress-core/Cargo.toml:7:license-file.workspace = true
…  (17 more crates, all identical)
xtask/Cargo.toml:7:license-file.workspace = true
                                            → 1 root + 18 members = 19 declarations

$ for f in $(find crates xtask -name Cargo.toml); do grep -q "^license" "$f" || echo "NO LICENCE: $f"; done
NO LICENCE: crates/vibe-index/Cargo.toml
```

**`crates/vibe-index` is a workspace member** (`Cargo.toml:15`, and again in
`default-members` at `:29`) and its manifest declares **no licence at all** —
neither `license` nor `license-file`, so it does not inherit
`[workspace.package] license-file = "LICENSE.md"` (`Cargo.toml:55`) either.
Read its header: `crates/vibe-index/Cargo.toml:1-13` carries `name`, `version`,
`edition`, `rust-version`, `authors`, `publish`, `description`, `homepage`,
`repository`, `keywords`, `categories`, `default-run` — and no licence line.

So the manifest half is **19 members, 18 declaring, one silent**, and the
«cannot fail by construction» argument fails on the one member that opted out
of the construction. `publish = false` is why no `cargo` command has ever
complained. The anchor's own words are «Keep `LICENSE.md` and **every** manifest
`license` field in sync» — a manifest with no field is not in sync, it is
absent, and this is a smaller and much cheaper host defect than the README one
that nobody has filed.

**Why both anchors are route (b).** `##THESE-MUST-NEVER-DISAGREE` (line 112) is
three words long and is a **prohibition**: «These must never disagree.» Its
neighbours make the register unambiguous — `##A-PRODUCT-STATES-ITS-LICENCE-IN-MORE-THAN-ONE-PLACE`
(line 108) lists the places («the `LICENSE.md` file, the manifest `license`
field, **sometimes a README badge**»), and
`##a-disagreement-is-a-contradiction-compliance-tooling-will-flag` (line 117)
says a disagreement is «a contradiction a consumer's compliance tooling will
flag — and rightly distrust». `##SUM-KEEP-EVERY-STATEMENT-IN-SYNC` (line 169)
opens with the imperative «**Keep** …».

The contradiction the verdict measured is entirely **host-internal**:
`README.md:164` against `LICENSE.md:3`, two host files. The package's sentence
is the rule that names it, and the rule is right — this is the same shape as
F-330's `##AN-UNGUARDED-CODE-PATH-IS-A-BUG`, a package confirmed *by* the
consumer failure it classifies. `licensing` is a prompt-only `world` flow that
ships no README of the product and no manifest of the product; under the §7
LOG's wave-2–4 rule its own tree contains nothing this sentence is false about.
Softening «must never disagree» because the consumer's README disagrees is the
*профанация* §3.6 exists to prevent, in the one place where the document being
wrong is legally visible.

**Proposed correction (NOT APPLIED):** none for either anchor — correct as
written. For the record, the host-side text a repair would use is the one line
already true everywhere else: `README.md:164`'s clause «vibevm itself ships
under the proprietary EULA placeholder in [`LICENSE.md`](LICENSE.md) for the
moment; the eventual target is UPL 1.0» → «vibevm itself ships under the
[Universal Permissive License 1.0](LICENSE.md) (relicensed 2026-07-12)». That
is a **host** edit, outside this worker's scope and not applied.

**Recommendation per anchor:**
`##THESE-MUST-NEVER-DISAGREE` → drift stands, route (b)
`##SUM-KEEP-EVERY-STATEMENT-IN-SYNC` → drift stands, route (b) — and its
verdict's «manifest half cannot fail» reasoning should not be carried forward,
because it already has.

**Host obligations this opens (recorded, not acted on).** (1) `README.md:164`
contradicts `LICENSE.md:3`, unmarked, 19 days on — the one statement of the
three that never caught up. (2) `crates/vibe-index/Cargo.toml` declares no
licence at all while its 18 sibling members inherit `license-file` from
`[workspace.package]`; a one-line addition, and the only reason it is invisible
is `publish = false`.

---

## F-239 — «ships with» is delivery, and the package's own three confirmed neighbours already ruled that distinction the other way; the real defect is in `SKILL.md` and is three-for-three across `world`

**Outcome:** MIXED — 1/2 **FALSE PREMISE, DIFFERENT DEFECT**, 1/2 SURVIVES — ROUTE (b)
**Anchors:** 2 of 2, by name:
`LICENSING-PROTOCOL.md#A-SKELETON-OF-THIS-TEXT-SHIPS-WITH-THE-DRAFT-EULA-SKILL` — **FALSE PREMISE, DIFFERENT DEFECT** (definition at that file's **line 56**)
`LICENSING-PROTOCOL.md#A-CHANGE-TO-ONE-IS-A-CHANGE-TO-ALL-IN-A-SINGLE-COMMIT` — **SURVIVES — ROUTE (b)** (**line 114**)
Both verified to be real definitions, not citations.
**Perimeter searched:** the standing perimeter, and then **the projection
mechanism itself**, because the verdict's whole case is a claim about what a
consumer receives: `vibe skill`'s enumerator
(`crates/vibe-cli/src/commands/skill/mod.rs`), the writer
(`crates/vibe-mcp/src/pkgskill.rs`), the `[[skill]]` schema
(`crates/vibe-core/src/manifest/package/skill.rs`), and **every `SKILL.md` any
package ships** — nine of them — read for the path form each uses. Plus the
package's own contents roster and the git history of the three licence
statements.

**The verdict's own commands, re-run — both reproduce exactly:**

```console
$ find packages/org.vibevm.world/licensing/v0.1.0/spec/skills -type f
packages/org.vibevm.world/licensing/v0.1.0/spec/skills/draft-eula/SKILL.md

$ sed -n '25p' packages/org.vibevm.world/licensing/v0.1.0/spec/skills/draft-eula/SKILL.md
   in `spec/flows/licensing/eula-template.md`: product name,
```

One file in the skill directory; the skeleton one directory over; the skill
reaching it by a package-relative path. **And the mechanism check confirms the
consequence the verdict inferred, which nobody had actually verified:**

```console
$ sed -n '257,270p' crates/vibe-mcp/src/pkgskill.rs
fn snapshot_source(source: &Path, include: &[String]) -> Result<BTreeMap<String, Vec<u8>>, …> {
    if source.is_dir() {
        collect_dir(source, source, &mut out)?;
        // PROP-015 §2.8: when `include` is set, keep only the files whose
        // relpath matches one of the patterns. …
        if !include.is_empty() { out.retain(|rel, _| include.iter().any(|pat| glob_match(pat, rel))); }
```

`collect_dir(source, source, …)` walks **only** the tree rooted at the skill's
declared `path`, and `include`
(`crates/vibe-core/src/manifest/package/skill.rs:75-80`) is a **filter, never a
widener** — its globs are relative to `path` and can only narrow. There is no
manifest key by which a file outside `path` reaches an agent. So a projected
`draft-eula` is one file, and `SKILL.md:25`'s instruction points at a path that
exists in neither the agent's skill home nor the consumer root — **the host has
no `spec/flows/` directory at all** (`ls -d spec/flows` → no such file;
`spec/` holds `WAL.md boot common design manual-tests modules terraforms`).

**So the consequence is real. What is false is that it is *this sentence's*
defect — and the package's own confirmed neighbours already settled the
distinction it turns on.**

Line 56 reads, in full:

> ##A-SKELETON-OF-THIS-TEXT-SHIPS-WITH-THE-DRAFT-EULA-SKILL A skeleton of this
> text ships with the `draft-eula` skill.

It is a claim about **shipping**. The skeleton ships: it is in the package
(`spec/flows/licensing/eula-template.md`), it is in the install slot
(`vibedeps/flow-licensing/0.1.0/spec/flows/licensing/eula-template.md`), and
this host received all eight of the package's files. The package's own contents
roster says so in the same words, and **all three of its anchors are
`confirmed`**:

```
README.md#package-contents-lead              confirmed  "«three pieces of content, a skill, and a boot snippet» is five
                                                        things … find … returns exactly 8 files … the consumer receives
                                                        all five pieces"
README.md#CONTENT-THE-EULA-TEMPLATE          confirmed  "spec/flows/licensing/eula-template.md — a copy-ready … skeleton"
README.md#CONTENT-THE-DRAFT-EULA-SKILL       confirmed  "the skill … IS delivered to this host at vibedeps/…/SKILL.md.
                                                        RECORDED … it is materialised into NONE of the three skill homes
                                                        this host runs … DELIVERED IS NOT INSTALLED, AND THE FACT ONLY
                                                        CLAIMS THE FORMER."
```

`##CONTENT-THE-DRAFT-EULA-SKILL` faced the identical question — a shipped thing
that reaches no agent — and was ruled **confirmed** on the explicit principle
**«delivered is not installed, and the fact only claims the former»**. F-239's
verdict reads the same verb the opposite way on the same package, on the same
day's evidence: it treats «ships with» as «is inside the skill's projected
payload». **One package, one body of evidence, two opposite readings of
"ships"** — which is §3.7's consistency corollary arriving with the sign
reversed, and three of the four rows are already recorded the way this entry
recommends.

The reading also does not survive the package's own layout as documented.
`README.md:19` says the package ships «three pieces of content, **a skill**, and
a boot snippet» — the template is enumerated as *content* (`README.md:26-27`)
and the skill separately as *a skill* (`README.md:32`). Under the package's own
taxonomy the skeleton was never claimed to be part of the skill's payload; it
was claimed to travel with it, and it does.

**The different defect, stated precisely, and it is not licensing's alone.**
The real fault is `SKILL.md:25` and `:34` using a **package-root-relative**
path that resolves from nowhere the skill is ever read. Swept across every
`SKILL.md` any package ships, the split is exact:

```console
$ for f in $(find packages/org.vibevm.* -path '*/spec/skills/*' -name SKILL.md \
      -not -path '*/vibedeps/*' -not -path '*/.vibe/*'); do grep -n "spec/flows/\|vibedeps/" "$f"; done

  go-ai-native-sweep      :12  `vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`
  go-ai-native-terraform  :12  `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`
  rust-ai-native-sweep    :12  `vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`
  rust-ai-native-terraform:12  `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`
  typescript-…-sweep      :12  `vibedeps/flow-core-ai-native/<version>/spec/04-SWEEP-PLAYBOOK.md`
  typescript-…-terraform  :12  `vibedeps/flow-core-ai-native/<version>/spec/mechanisms/`
  fractality-delegate          (cites neither)
  health-audit            :14  `spec/flows/health-audit/`          :21  `spec/flows/health-audit/audit-checklist.md`
  draft-eula              :25  `spec/flows/licensing/eula-template.md`  :34  `…/dependency-licenses.md`
  wal-status              :11  `spec/flows/wal/morning-routine.md` :16  `spec/flows/wal/WAL-PROTOCOL.md`
```

**Six for six, the `ai-native` stack skills use the consumer-resolvable
`vibedeps/<slot>/…` form. Three for three, the `world` flow skills use the
package-root-relative form that resolves from nowhere.** The correct form is
already in this repository, written six times, in the same shipped surface. So
this is one address-family defect with three members, on `SKILL.md` files —
kin to F-136 / F-145 and to `BACKLOG.md` B-004 — and not a defect of
`LICENSING-PROTOCOL.md:56`.

**The second anchor: the git facts reproduce, and the sentence is a rule.**

```console
$ git show --stat --format='%h %ad %s' --date=short 5086c5b5
5086c5b5 2026-07-12 chore(license): relicense vibevm to UPL-1.0
 LICENSE.md | 65 ++++++---   1 file changed, 44 insertions(+), 21 deletions(-)

$ git log -S'UPL-1.0' --format='%h %ad %s' --date=short -- spec/common/PROP-000.md
71d8383b 2026-07-25 docs(spec): Phase D d1b — the foundation catches up with reality
bf311a39 2026-04-17 docs(spec): bootstrap self-hosted vibevm spec tree per §14.1

$ git log -S'proprietary EULA placeholder' --format='%h %ad %s' --date=short -- README.md
d85f770a 2026-04-26 docs: top-level README at repo root

$ git log -1 --format='%h %ad %s' --date=short -- README.md
350cd8ce 2026-07-13 chore(repo): point all source URLs at the vibevm/vibevm repos
```

One licence file changed alone on 2026-07-12; the foundational record caught up
**thirteen days later**; the README was touched on 2026-07-13 — *the day after*
— and its licence line was not, and still has not been, **nineteen days on**.
The rule asked for one commit and got three, one of which has not happened.

`##A-CHANGE-TO-ONE-IS-A-CHANGE-TO-ALL-IN-A-SINGLE-COMMIT` (line 114) is a
prescription sitting directly under `##THESE-MUST-NEVER-DISAGREE` in the same
`## Keep the statements in sync {#sync}` section, and it is the *how* of that
prohibition. §3.6 does not let it yield to a consumer that took three commits,
and the package ships no product README or manifest of its own for the sentence
to be false about.

**Proposed correction (NOT APPLIED):** none for either anchor. The text a repair
would touch is `SKILL.md`, not `LICENSING-PROTOCOL.md`, and it is the same
edit in three packages — for the record, `draft-eula/SKILL.md:25`'s «fill the
skeleton in `spec/flows/licensing/eula-template.md`» would become an address of
the form the six stack skills already use. **Not applied, and it is a different
anchor in a different file from either of F-239's.**

**Recommendation per anchor:**
`##A-SKELETON-OF-THIS-TEXT-SHIPS-WITH-THE-DRAFT-EULA-SKILL` → **re-judge
confirmed** — «ships with» is delivery, the skeleton is delivered in the same
package and the same install slot, and the package's three confirmed contents
anchors already ruled that «delivered is not installed, and the fact only
claims the former». The projection gap is real and belongs to `SKILL.md:25`.
`##A-CHANGE-TO-ONE-IS-A-CHANGE-TO-ALL-IN-A-SINGLE-COMMIT` → drift stands, route (b)

**Host / package obligations this opens (recorded, not acted on).** (1) Three
`world` flow skills — `health-audit`, `draft-eula`, `wal-status` — cite
package-root-relative `spec/flows/…` paths that resolve from neither a
projected skill home nor the consumer root (the host has no `spec/flows/`); the
six `ai-native` stack skills already use the resolvable `vibedeps/<slot>/…`
form. One address-family repair, three members, and it is a **package** edit on
three packages, so it is a release event under §4.5 rather than a prose edit.
(2) `README.md:164`'s licence line is the one statement of the three that never
caught up — the same host obligation F-236 opens.

---

## F-227 — the deferral reproduces to the minute; the second anchor's evidence is the wrong rule's, and the right instance was sitting one paragraph away

**Outcome:** MIXED — 2/2 SURVIVE — ROUTE (b), but one on **FALSE-PREMISE evidence replaced by an instance the verdict did not find**
**Anchors:** 2 of 2, by name:
`58-flow-dev-runtime-docs.md#NEVER-SHIP-A-SETUP-CHANGE-WITH-THE-DOC-UPDATE-DEFERRED` — **SURVIVES — ROUTE (b)** (definition at that file's **line 15**)
`58-flow-dev-runtime-docs.md#NEVER-LET-THE-DOCS-DESCRIBE-AN-ABANDONED-TOOLCHAIN` — **SURVIVES — ROUTE (b)**, on **different evidence**; the verdict's own two exhibits are instances of the *sibling* rule (**line 23**)
Both verified to be real definitions, not citations.
**Perimeter searched:** the standing perimeter, plus a full re-measurement of
every figure the two verdicts assert, plus — because «an abandoned toolchain» is
a claim about things that *no longer exist* and cannot be grepped for — the two
guides read against the **tree** they describe: every path, tool and workspace
they name, checked for existence, tracked-ness and workspace membership.

**The verdict's own facts, re-run — the timing reproduces to the minute:**

```console
$ git log --format='%h %ad %s' --date=format:'%Y-%m-%d %H:%M' --since=2026-07-20T14:00 --until=2026-07-20T17:00
8b9b6304 2026-07-20 16:44 feat(registry): optional `enabled` flag …
e19efec6 2026-07-20 16:28 docs(registry): clarify ~/.vibe/registry.toml accepts any registry…
14e11747 2026-07-20 16:17 docs: point tokens, config, and aiui discovery at the canonical ~/.vibe
74f08cc7 2026-07-20 15:55 chore(discipline): satisfy the conform + specmap floor for the settings work
8aec7cc9 2026-07-20 15:38 feat(registry): machine-global registry config merged project-first
f0e89db5 2026-07-20 15:28 feat(settings): consolidate the settings home behind one chokepoint
```

15:28 → 16:17 is **49 minutes**; 15:38 → 16:28 is **50 minutes**. Exactly as
recorded. And the two setup commits carried **no** guide edit — `f0e89db5`
touched 14 files, all `crates/**` (`settings.rs`, `user_config.rs`, `token.rs`,
`loader.rs`); `8aec7cc9` touched 7, six `crates/**` plus one PROP. The guides
moved 49 and 50 minutes later, in commits of their own. **The rule's named
failure, performed twice inside one hour, on the day the settings home moved.**

**An independent measurement, because the verdict's «at most 3 of 36
setup-surface commits» defines neither the window nor «setup-surface» and so
cannot be reproduced as stated.** Substituted: the single cleanest setup
surface in the tree, whole history, mechanically decidable —

```console
$ for c in $(git log --format='%h' -- tools/self-check.sh); do
      git show --name-only --format='' $c | grep -qE '^(DEV-GUIDE|RUNTIME-GUIDE)\.md$' \
        && echo "GUIDE-TOO $c" || echo "deferred  $c"; done
  27 commits touched tools/self-check.sh
   2 updated a guide in the SAME commit   (2b815a47 2026-05-04, the file's own creation;
                                           9be4c4fd 2026-07-07, a naming refactor)
  25 did not
```

**2 of 27.** Same direction as the verdict, on a definition anyone can re-run.
`de14b27e` (2026-05-22, «build(self-check): gate cargo fmt --check») is in the
25: **one file changed, `tools/self-check.sh`**, and the guide was not touched —
which is how the fmt step came to be missing from §6 for 70 days.

**Anchor 2, and this is where the re-verification changes the record.** The
verdict's two exhibits both reproduce as *measurements* and neither is an
instance of the rule the anchor states.

*Exhibit one — three versus ten:*

```console
$ sed -n '330p' DEV-GUIDE.md
It runs three invariants in order, exiting non-zero on the first failure (pass `--keep-going` to run all three regardless):

$ sed -n '6,36p' tools/self-check.sh          # the script's own header
#   1. `cargo fmt --all --check`   2. `cargo test --workspace`   3. `cargo clippy …`
#   4. `vibe check --path . --quiet`   5. `cargo xtask conform check`
#   6. `cargo xtask sync-engines --check`   7. the core-ai-native package gate
#   8. the language-stack package gates   9. the packages' traceability self-trace
#  10. the mcp package gates                      … plus a step 0b denominator guard

$ grep -c "run_step" tools/self-check.sh
37                    # 36 invocations + the function definition at line 86
```

Ten enumerated invariants against the guide's three, and 36 `run_step`
invocations. **But the three the guide names are all still run** — `cargo test`
(step 2, `self-check.sh:264`), `clippy` (step 3, `:272`), `vibe check` (step 4,
`:282`) — and its account of the semantics is still exactly right: `run_step`
exits on the first failure unless `--keep-going` is passed
(`self-check.sh:86-99`, `if [ "$KEEP_GOING" -eq 0 ]; then exit "$rc"`). The
guide is **incomplete**, not describing anything abandoned.

*Exhibit two — 81 tests, and the number moved:*

```console
$ sed -n '84p' DEV-GUIDE.md
81 tests green on `main` as of the last checkpoint; clippy clean with `-D warnings`.

$ grep -rhoE "#\[(tokio::)?test\]" crates xtask --include='*.rs' | wc -l
2100                   # 2050 #[test] + 50 #[tokio::test], at HEAD 596588fb
```

**The verdict recorded 2 075; the same command returns 2 100 three days later.**
A stale figure, not an abandoned toolchain — and a figure that decays weekly,
which is the wave-6 lesson about naming a HEAD.

So both exhibits are the **sibling rule's** failures: the docs are stale
*because* the updates were deferred, which is
`##NEVER-SHIP-A-SETUP-CHANGE-WITH-THE-DOC-UPDATE-DEFERRED` (line 15) and
`##EVERY-SETUP-TOUCHING-CHANGE-UPDATES-THE-DOC-IN-THE-SAME-COMMIT` (line 11).
Neither is a toolchain the project *no longer uses*.

**And then the instance that is.** Reading the guides against the tree rather
than against a string:

```console
$ sed -n '265p' DEV-GUIDE.md
- Opening the **repo root** indexes the host workspace (`crates/`, `xtask`, `apps/`).
  That is what you want for `vibe-cli` work.

$ ls -la apps/
total 16      (nothing but . and ..)

$ git ls-files apps/
(no output — not tracked)

$ git show --stat --format='%h %ad %s' --date=short 7e46d841 | head -4
7e46d841 2026-07-22 chore(extract): drop the moved terminal/launcher sources
 apps/vibeframe/README.md   | 180 -
 apps/vibeframe/index.html  |  71 -

$ sed -n '406,410p' tools/self-check.sh
# 11. The vibeterm / vibeframe terminal products moved to a separate repo
# (`vibevm-term`); … The host's self-check no longer runs them.
```

`apps/` was emptied on **2026-07-22** when the terminal and launcher products
were extracted to `vibevm-term`. It is untracked, it holds nothing, and it is
not a member of the root `Cargo.toml` workspace (`Cargo.toml:8-27` lists
`crates/*` and `xtask`, no `apps/*`) — so it is not indexed by anything and
never was a Cargo tree. The setup doc still names it as one of the three things
the root workspace indexes, nine days after the extraction, and the file that
records the extraction is the very script §6 describes.

**That is `##NEVER-LET-THE-DOCS-DESCRIBE-AN-ABANDONED-TOOLCHAIN`, exactly and
literally**, and no verdict in this campaign had found it.

**Why both anchors are route (b).** They are the two entries of the flow's
`## Never {#never}` section (lines 22-23) and the restatement of its `## The
rule {#rule}` (lines 11-16): «Never ship…», «Never let…» — prohibitions, four
of four in the imperative. `dev-runtime-docs` is a prompt-only `world` flow with
no crate, no guide of its own and no gate; its own tree contains nothing either
sentence is false about, and every exhibit above is a **host** file. §3.6 does
not let a prohibition yield to a consumer that broke it 25 times out of 27.

The host has adopted both rules in its own words and breaks them anyway:
`DEV-GUIDE.md:7` — «Every change touching toolchain, prerequisites, env vars, or
bootstrap steps MUST update this file in the same commit. Never ship a dev-env
change and a doc update separately. Policy pinned in PROP-000 — the obligation
is load-bearing.» The consumer wrote the rule down, in the file the rule
protects, and deferred it anyway.

**Proposed correction (NOT APPLIED):** none in the package — both sentences are
correct as written. The corrections belong to **host** files and are recorded
here rather than applied: `DEV-GUIDE.md:330`'s «three invariants … all three»
understates a ten-invariant, 36-step gate; `DEV-GUIDE.md:84`'s «81 tests green»
stands against 2 100 test attributes; `DEV-GUIDE.md:265` names an emptied
`apps/` among the trees the root workspace indexes.

**Recommendation per anchor:**
`##NEVER-SHIP-A-SETUP-CHANGE-WITH-THE-DOC-UPDATE-DEFERRED` → drift stands, route (b)
`##NEVER-LET-THE-DOCS-DESCRIBE-AN-ABANDONED-TOOLCHAIN` → **drift stands, route (b) — on
substituted evidence.** The verdict's own two exhibits belong to the sibling
rule; the instance that fits this anchor is `DEV-GUIDE.md:265`'s `apps/`, and
the record should carry that one so a repair fixes the right sentence.

**Host obligations this opens (recorded, not acted on).** (1) `DEV-GUIDE.md:265`
names `apps/` — emptied 2026-07-22 by `7e46d841`, untracked, not a workspace
member — as one of three trees the repo-root workspace indexes. (2)
`DEV-GUIDE.md:330` describes `tools/self-check.sh` as three invariants; the
script's own header enumerates ten and it makes 36 `run_step` calls, and
`self-check.sh:4` points the reader back at that section. (3)
`DEV-GUIDE.md:84`'s «81 tests green» against 2 100 `#[test]` / `#[tokio::test]`
attributes — a figure with no HEAD and no way to age well.

---

## F-114 — the edition claim survives and is worse than recorded (four member sets under one version); the uninstall claim is FALSE and the host contract it was judged against is the thing that is unbuilt

**Outcome:** MIXED — 1/2 SURVIVES (the package's own defect, correction prepared), 1/2 **FALSE**
**Anchors:** 2 of 2, by name:
`README.md#AN-EDITION-IS-A-TESTED-SET-OF-EXACT-PINS` — **SURVIVES**, and it is this package's **own** defect, §3.6(a) (definition at that file's **line 25**)
`README.md#UNINSTALLING-THE-UMBRELLA-REMOVES-ONLY-ITS-OWN-FILES` — **FALSE** (**line 119**)
Both verified to be real definitions, not citations.
**Perimeter searched:** the standing perimeter, plus two things a string search
cannot reach. For the edition claim: the manifest's pin list **counted by hand
and by command at every commit that ever touched it**, since the claim is about
a set changing over time and no grep sees history. For the uninstall claim: the
**implementation**, not the host's prose about it — `vibe uninstall`'s whole
control flow read end to end, plus a search for a root/transitive refusal under
any name anywhere in `crates/` and `xtask/`, because a contract asserted in a
PROP is not evidence that code performs it.

**The verdict's own numbers, re-counted myself (§brief: «if a roster count is
your subject, count it yourself» — the redbook family has three rosters recorded
at 22 / 21 / 23):**

```console
$ grep -cE '^"flow:' packages/org.vibevm.world/redbook/v0.2.0/vibe.toml
22
$ grep -cE '^"flow:[^"]+" = "=[0-9]+\.[0-9]+\.[0-9]+"$' …/vibe.toml
22
$ grep -E '^"flow:' …/vibe.toml | grep -vcE '= "=[0-9]+\.[0-9]+\.[0-9]+"$'
0
$ grep -cE '^\| ##ROW-' packages/org.vibevm.world/redbook/v0.2.0/README.md
21
```

**22 pins, 22 exact, 0 caret or range** — the verdict's arithmetic reproduces
exactly. The README's member tables carry **21** rows, and the two rosters name
different sets: the README lists `git-atomic-commits` and
`git-attribution-policy`, which the manifest does not pin; the manifest pins
`git-practices`, `dev-runtime-docs` and `wal-specspaces`, which the README does
not list. (That divergence is F-113's subject and is recorded here only so the
count in this entry cannot be mistaken for it.) And the manifest comment the
verdict quotes is exactly where it says:

```console
$ sed -n '48,49p' packages/org.vibevm.world/redbook/v0.2.0/vibe.toml
# The cultural-extraction wave (edition bump to a clean 0.3.0 lands when the
# full new practice set has settled; accumulated here in place meanwhile).
```

**And the finding is materially worse than «members were added».** The edition's
member set has been **four different sets**, and names were **removed** as well
as added, all under one unmoving version:

```console
$ for c in 69708287 041ef527 c939951a 093c053c HEAD; do
      git show $c:packages/org.vibevm.world/redbook/v0.2.0/vibe.toml | grep -cE '^"flow:'; done
69708287  2026-07-12  pins=21  version = "0.2.0"
041ef527  2026-07-14  pins=22  version = "0.2.0"   -atomic-commits +git-practices +dev-runtime-docs
c939951a  2026-07-14  pins=21  version = "0.2.0"   -attribution-policy
093c053c  2026-07-15  pins=22  version = "0.2.0"   +wal-specspaces
HEAD      2026-07-31  pins=22  version = "0.2.0"

$ git log -p -- packages/org.vibevm.world/redbook/v0.2.0/vibe.toml | grep -E '^\+version|^-version'
+version = "0.2.0"          # written once, never changed
```

**21 → 22 → 21 → 22, four member sets published as one edition number, over
three days, with two members removed.** Two projects that ran
`vibe install flow:redbook` on 2026-07-12 and on 2026-07-16 hold the same
edition and different practice text — and one of them holds `atomic-commits`
and `attribution-policy`, which the current edition does not contain at all. The
causal clause «**so** two projects on the same edition run byte-identical
practice text» does not follow, and the exact-pin premise it is drawn from is
sound. The defect is precisely the «so».

**This one is the package's own, §3.6(a), and it is the only such anchor in this
batch.** The falsifying evidence sits **inside the package's own tree** — its
own `vibe.toml`, its own comment, its own history — not in any consumer. The §7
LOG's wave-2–4 test («a package moves only where its own sentence is false about
something inside its own tree») is satisfied here and nowhere else in D7f.

**Proposed correction (NOT APPLIED).** Exact replacement for
`packages/org.vibevm.world/redbook/v0.2.0/README.md:25-27`:

```markdown
##AN-EDITION-IS-A-TESTED-SET-OF-EXACT-PINS An edition is a
tested set: every member is pinned exactly (`=X.Y.Z`), so no member's
version can skew inside an edition. The *roster* is a second question —
while a wave settles, members are accumulated in place and the edition
number moves once at the end (see the manifest's own note above the
cultural-extraction wave), so two projects on the same edition run
byte-identical text of every member they share. @impl/done
```

*(The alternative repair is the practice rather than the prose — bump the
umbrella to `0.3.0` on the next member change and keep the original sentence
true. That is an owner decision about the package's release policy, not a
document edit, and it would also close F-113's roster divergence; it is stated
here rather than chosen.)*

**Anchor two — FALSE, and the measurement inverts what the verdict found.** The
sentence at `README.md:119-120` is:

> Uninstalling the umbrella removes its own files; member packages are
> removed by uninstalling them individually.

The verdict ruled its second half contradicted by the host's written contract:

```console
$ sed -n '530p' spec/modules/vibe-registry/PROP-002-decentralized-registry.md
- ##LF-ROOT-DEPENDENCIES … `vibe uninstall` of a root drops the entry from both
  files; `vibe uninstall` of a pure transitive is rejected with an explanation. @impl/done
```

That line reproduces, and so does the root/transitive arithmetic: **4 of the 22
members are lockfile roots** — `conflict-protocol`, `dev-runtime-docs`,
`git-practices`, `wal-specspaces` (`vibe.lock:5-16`) — so 18 are pure
transitives, and `redbook`'s own `files_written = []` (`vibe.lock:40`).

**But nothing rejects anything.** I read `vibe uninstall` end to end
(`crates/vibe-cli/src/commands/uninstall.rs:27-149`) and there is **no
root/transitive branch in it at all**. The only gate is existence:

```console
$ sed -n '43,50p' crates/vibe-cli/src/commands/uninstall.rs
    let locked = lockfile.find(group, &pkgref.name).ok_or_else(|| {
        anyhow!("package `{}/{}` is not installed in `{}`", …) })?;

$ sed -n '129,138p' crates/vibe-cli/src/commands/uninstall.rs
    lockfile.remove(group, &pkgref.name);
    lockfile.meta.root_dependencies
        .retain(|r| !(r.group.as_ref() == Some(group) && r.name == pkgref.name));
    …
    let manifest_changed = drop_from_manifest_requires(&mut manifest, group, &pkgref.name);
```

The `retain` and the manifest drop are **no-ops for a pure transitive**, and the
command proceeds to remove the slot and regenerate boot exactly as for a root.
Searching for the *thing* rather than the string — any refusal under any name
across `crates/` and `xtask/` — returns nothing; the two hits that look like it
are documentation, not code:

```console
$ grep -rn -iE "pure transitive|is not a root|not a declared root" crates/ xtask/ --include='*.rs'
crates/vibe-core/src/manifest/lockfile.rs:120:  /// … uninstalling a pure transitive is refused.       ← a doc comment
crates/vibe-cli/tests/cli_pkg_cycle.rs:719:    /// Pure transitives (never declared in the manifest)
                                              /// LEAVE THE MANIFEST UNTOUCHED.                        ← the test's expectation
```

The integration test's own doc says a pure transitive's uninstall **succeeds and
leaves the manifest untouched** — the opposite of refused. **So the code and the
tests agree with the package, and it is the host's `PROP-002
##LF-ROOT-DEPENDENCIES` (`@impl/done`) and `lockfile.rs:120` that assert a
refusal nothing performs.**

Measured today, `vibe uninstall org.vibevm.world/health-audit` — a pure
transitive of `redbook` — removes it. The package's sentence is **true of the
implementation**, and it was convicted against a host contract that is itself
the unbuilt thing. This is failure mode 4: the fact and the evidence are about
different things — one describes what the tool does, the other what a PROP says
it should do.

**Recommendation per anchor:**
`##AN-EDITION-IS-A-TESTED-SET-OF-EXACT-PINS` → **drift stands, correction
prepared** (route (a) — the package's own defect; the correction above is
written and **not applied**, and it needs owner approval because this obligation
sits on `sync-from-code`).
`##UNINSTALLING-THE-UMBRELLA-REMOVES-ONLY-ITS-OWN-FILES` → **re-judge
confirmed** — `vibe uninstall` performs no root/transitive check, its own
integration test expects a pure transitive to uninstall cleanly, and the
sentence describes what the tool does.

**Host obligations this opens (recorded, not acted on).** (1)
`spec/modules/vibe-registry/PROP-002-decentralized-registry.md:530`
`##LF-ROOT-DEPENDENCIES` is `@impl/done` over «`vibe uninstall` of a pure
transitive is rejected with an explanation», and no code rejects anything; the
same claim is repeated in a doc comment at
`crates/vibe-core/src/manifest/lockfile.rs:120`. Either the marker drops or the
check gets built — and if it gets built, **18 of the redbook umbrella's 22
members become uninstallable in this repository**, which is a product decision
rather than a bug fix. (2) The README's 21-row roster and the manifest's 22 pins
name different sets (`git-atomic-commits` / `git-attribution-policy` versus
`git-practices` / `dev-runtime-docs` / `wal-specspaces`) — F-113's subject,
re-measured here and unchanged.

---

## Batch summary

| id | package | outcome | anchors | FALSE | route (b) | route (a) |
|---|---|---|---:|---:|---:|---:|
| F-097 | `health-audit` | SURVIVES — ROUTE (b) | 4 | 0 | 4 | 0 |
| F-203 | `secrets-hygiene` | MIXED — 1/3 FALSE | 3 | **1** | 2 | 0 |
| F-330 | `secrets-hygiene` | SURVIVES — ROUTE (b) | 1 | 0 | 1 | 0 |
| F-236 | `licensing` | SURVIVES — ROUTE (b) | 2 | 0 | 2 | 0 |
| F-239 | `licensing` | MIXED — 1/2 FALSE PREMISE | 2 | **1** | 1 | 0 |
| F-227 | `dev-runtime-docs` | MIXED — evidence substituted | 2 | 0 | 2 | 0 |
| F-114 | `redbook` | MIXED — 1/2 FALSE | 2 | **1** | 0 | **1** |
| **total** | | | **16** | **3** | **12** | **1** |

**Three of sixteen verdicts turned out FALSE — 19 %.** That is below both
`build-or-demote` waves (wave 5: 18 of 76, 24 %; wave 6: 31 of 59, 53 %), and
the reason is structural rather than lucky. A `build-or-demote` verdict asserts
an **absence**, which a wider perimeter can overturn wholesale. A
`sync-from-code` verdict asserts a **discrepancy**, and a discrepancy measured
correctly on Monday is usually still a discrepancy on Thursday. What decays here
is the **number**, not the finding — and what turns out false is a
**misattribution**. All three falses below are misattributions, not
mis-measurements.

**Zero edits.** No file under `packages/` was touched, no verdict JSON written,
nothing under `campaigns/packages-2026-09/run/` modified, no writing `git`
command run, no credential file read or printed. **One correction is prepared
and not applied** (F-114's edition clause) — the only anchor in the batch whose
defect is the package's own.

### The three that are FALSE, and each is the same shape

**A sentence was convicted of its neighbour's defect.** In every case the
measurement the verdict took was correct; the anchor it attached that
measurement to was wrong.

1. **`third-party-code-consent.md#the-prompt-points-at-a-real-path`** (F-203) —
   the sentence is a **modal capability** claim: «The consent prompt *can* point
   at a real path.» The capability is built and spec-marked
   (`crates/vibe-workspace/src/hooks.rs:323-353`, `select_invocation` under
   `#[spec(implements = "…PROP-020#script-selection")]`). **The verdict concedes
   this in its own first clause** — *«the anchor's claim that a prompt can point
   at something openable is true of the design»* — then rules drift because the
   capability is unused. «The prompt *does* print the path» is a different
   sentence, `##GATE-EVERYONE-ELSE-GETS-FIRST-RUN-CONSENT` (line 46), which
   carries its own drift verdict in the same obligation. One defect, filed twice.

2. **`LICENSING-PROTOCOL.md#A-SKELETON-OF-THIS-TEXT-SHIPS-WITH-THE-DRAFT-EULA-SKILL`**
   (F-239) — «ships with» is **delivery**, and the skeleton is delivered: same
   package, same install slot, enumerated in the package's own contents roster.
   All three roster anchors are `confirmed`, and
   `##CONTENT-THE-DRAFT-EULA-SKILL`'s confirmation states the governing
   principle in its own words — **«delivered is not installed, and the fact only
   claims the former»** — on the identical question of a shipped thing that
   reaches no agent. F-239 reads the same verb the opposite way on the same
   package. The real defect is `SKILL.md:25`'s package-relative path, and it is
   **three-for-three across the `world` flow skills** (`health-audit`,
   `draft-eula`, `wal-status`) while the six `ai-native` stack skills already use
   the consumer-resolvable `vibedeps/…` form.

3. **`redbook/README.md#UNINSTALLING-THE-UMBRELLA-REMOVES-ONLY-ITS-OWN-FILES`**
   (F-114) — judged against the host's *written contract*
   (`PROP-002 ##LF-ROOT-DEPENDENCIES`: «a pure transitive is rejected with an
   explanation», `@impl/done`) rather than against the host's *code*. **`vibe
   uninstall` has no root/transitive branch at all**
   (`crates/vibe-cli/src/commands/uninstall.rs:27-149`): the `root_dependencies`
   `retain` and the manifest drop are no-ops for a transitive and the command
   proceeds to remove the slot. Its own integration test says a pure transitive's
   uninstall leaves the manifest untouched
   (`crates/vibe-cli/tests/cli_pkg_cycle.rs:719`). **The code agrees with the
   package; the unbuilt thing is the host contract the package was measured
   against.**

### Two more where the verdict's stated evidence is false and the anchor survives anyway

Recorded separately because the boss's action differs: the **record** needs
correcting, not the verdict reversing.

- **`58-flow-dev-runtime-docs.md#NEVER-LET-THE-DOCS-DESCRIBE-AN-ABANDONED-TOOLCHAIN`**
  (F-227). Both exhibits — «three invariants versus ten» and «81 tests green
  versus 2 075» — are *staleness and incompleteness*, which is the **sibling
  rule's** failure; the three invariants the guide names are all still run and
  its account of the run semantics is still exactly right. The instance that fits
  *this* anchor was one paragraph away and no verdict had found it:
  **`DEV-GUIDE.md:265` still names `apps/` among the trees the repo-root
  workspace indexes** — a directory emptied on 2026-07-22 by `7e46d841` when the
  terminal products moved to `vibevm-term`, untracked by git, and not a Cargo
  workspace member. `tools/self-check.sh:406-410` records the extraction.
- **`LICENSING-PROTOCOL.md#SUM-KEEP-EVERY-STATEMENT-IN-SYNC`** (F-236). Its
  verdict rests half its case on «the manifest half is in sync BY CONSTRUCTION
  and cannot fail». It already has: the workspace is **19 members**, 18 declare
  `license-file.workspace = true`, and **`crates/vibe-index/Cargo.toml` declares
  no licence at all** — it opted out of the construction, and only
  `publish = false` keeps `cargo` quiet about it.

### Every count re-measured, and which ones moved

| figure | as recorded | at `HEAD = 596588fb` | |
|---|---|---|---|
| commits since the last audit (F-097) | 1 546 | **1 659** | moved +113 |
| `#[test]` / `#[tokio::test]` (F-227) | 2 075 | **2 100** | moved +25 |
| host manifests declaring a licence (F-236) | «all 18 … cannot fail» | **19 declarations; 1 member declares none** | premise false |
| redbook exact pins (F-114) | 22 | **22, all `=X.Y.Z`, 0 caret** | exact |
| redbook members that are lockfile roots (F-114) | 4 of 22 | **4 of 22** | exact |
| `run_step` in `self-check.sh` (F-227) | 37 | **37** (36 invocations + the definition) | exact |
| deferral gap on 2026-07-20 (F-227) | 49 and 50 minutes | **49 and 50 minutes** | exact to the minute |
| self-check invariants vs the guide (F-227) | 10 vs 3 | **10 vs 3** | exact |
| skill homes (F-097) | 5 / 4 / 4, none `health-audit` | **5 / 4 / 4, none** | exact |
| `DEFAULT_ALLOWED_GROUPS` (F-203) | `["org.vibevm"]`, no config key | **identical** | exact |
| `LICENSE.md:3` vs `README.md:164` (F-236) | UPL-1.0 vs «proprietary EULA placeholder» | **unchanged, 19 days on** | exact |

**The two figures that moved are age, not error** — both of the genre wave 6
named: a count over a live window that decays within the week unless it names
its HEAD. Every figure in this record names one.

**A count no verdict took, and it is the sharpest thing in the batch.**
`redbook`'s edition 0.2.0 has carried **four different member sets** —
21 → 22 → 21 → 22 pins across `69708287` (2026-07-12), `041ef527` and
`c939951a` (both 2026-07-14) and `093c053c` (2026-07-15) — with
`version = "0.2.0"` written once and never changed, and with two members
(`atomic-commits`, `attribution-policy`) **removed**, not merely added. Two
projects that installed the same edition four days apart hold different practice
text, and one holds flows the current edition does not contain.

### The perimeter earned its keep twice, in opposite directions

- **Widening into `packages/`** (the wave-6 extension): the `fractality`
  specspace **installs `flow-health-audit`** (its `vibe.lock:30`) and
  **compiles this very snippet into its own boot lane** (its
  `spec/boot/INDEX.md:57`) while keeping **no `AUDIT.md` at all**. A second
  consumer failing the same rule — F-097 widens rather than falls.
- **Widening into `packages/` again, and it narrowed a finding:** the sixteen
  `vibe.toml` manifests still reading `license = "EULA"` are **all** inside that
  specspace's `.vibe/cache/` and `vibedeps/` — exactly the two categories
  `CLAUDE.md:131-133` declares off-limits. No canonical package manifest names
  the old licence.

### The consistency corollary fired three times, and twice it argued for FALSE

- **F-330** — six of the seven anchors in `scope-discipline.md`'s section are
  `confirmed`, and **two say in their own verdict text that the `push_url`
  failure was deliberately filed onto the drifting row**
  (`##GUARDED-ACTION-MODIFY`: «carried as drift at the every-action row rather
  than counted twice here»; `##AN-UNGUARDED-CODE-PATH-IS-A-BUG`: «the failure
  itself is carried by `the-check-runs-on-every-action-lead`»). That anchor is a
  designated accumulator, exactly as F-287 was in wave 6, and cannot be re-judged
  apart from the six confirmations leaning on it.
- **F-239** — the package's three contents anchors are `confirmed` on the very
  principle F-239 contradicts; two of the four rows are already recorded the way
  this entry recommends.
- **F-097** — all four anchors state one rule at four strengths and fail on the
  same two ship lines; judged as one set and ruled identically.

### Host obligations this batch opens, in the order they deserve attention

1. **The tool's own printed fix breaks the user's config** (F-203).
   `crates/vibe-workspace/src/hooks.rs:126` and
   `crates/vibe-cli/src/commands/install/mod.rs:263` tell the operator to
   allow-list a group in `[hooks].allowed_groups`;
   `crates/vibe-core/src/user_config.rs:63`'s `deny_unknown_fields` plus the
   `?` at `:212` turn doing so into `UserConfigError::Parse`. Worse than a
   missing feature.
2. **`push_url` is unguarded on both adapters** (F-330) —
   `crates/vibe-publish/src/github.rs:156`, `gitverse.rs:135`; and
   `creator.rs:141` returns `String`, so `ScopeViolation` has no channel. A
   trait-signature change across two impls and four call sites. Two host
   statements also disagree about whether the guard already runs everywhere
   (`registry/publish.rs:125-126` «every method call» vs `creator.rs:154-155`
   «`repo_exists` / `create_repo`»).
3. **`spec/modules/vibe-registry/PROP-002-decentralized-registry.md:530` is
   `@impl/done` over a refusal no code performs** (F-114), repeated as a doc
   comment at `crates/vibe-core/src/manifest/lockfile.rs:120`. If it were built,
   **18 of `redbook`'s 22 members become uninstallable in this repository** — a
   product decision, not a bug fix.
4. **`README.md:164` still names the proprietary EULA placeholder** (F-236),
   unmarked in `CLAUDE.md`'s off-limits ledger while its neighbour
   `VIBEVM-SPEC.md:8` is correctly on it. Nineteen days on.
5. **`DEV-GUIDE.md:265` names an emptied `apps/`** (F-227) — the one genuine
   instance of the abandoned-toolchain rule anywhere in this batch.
6. **Six of ten package-declared skills are projected into no harness home**
   (F-097) — `health-audit`, `draft-eula`, `wal-status`, `go-ai-native-sweep`,
   `go-ai-native-terraform`, `fractality-delegate`. Six boot-lane instructions
   fail when followed; one `vibe skill install` closes all six.
7. **Three `world` flow `SKILL.md` files cite package-root-relative
   `spec/flows/…` paths** resolving from neither a projected skill home nor the
   consumer root — the host has no `spec/flows/` (F-239). The six `ai-native`
   stack skills already use the resolvable `vibedeps/<slot>/…` form. This is a
   **package** edit in three packages, so §4.5 makes it a release event.
8. **Two milestones declared done on an un-audited base** (F-097) — M1.26 and
   M1.24, both `SHIPPED (2026-07-07)`, now 1 659 commits past the last audit;
   and the `fractality` specspace boots the same flow with no `AUDIT.md`.
9. **`crates/vibe-index/Cargo.toml` declares no licence** (F-236) — one line,
   invisible only because `publish = false`.
10. **`DEV-GUIDE.md:330` (three invariants vs ten) and `:84` (81 tests vs
    2 100)** (F-227) — in the guide `tools/self-check.sh:4` points its reader at.

### What was deliberately not done

- **No package file was edited.** F-114's correction is written out in its entry
  and **not applied**; it is the only anchor in the batch whose defect is the
  package's own, and on `sync-from-code` its diff needs the owner.
- **No verdict JSON was written and nothing under
  `campaigns/packages-2026-09/run/` was touched.** Twelve anchors are
  recommended for route (b) and three for re-judging `confirmed`; recording
  either is the boss's, per §7's exit gate.
- **`vibe progress check` was not run** — it writes the campaign cache, which
  this worker is forbidden to touch, and no document changed for it to check.
- **No `git` command that writes was run.** Every history read is
  `log` / `show` / `ls-files`.
- **No credential file was read, opened, listed, printed or copied**, and no
  permission was inspected or changed. Wave 6's `~/.vibe` ACL exposure is cited,
  not re-derived.
