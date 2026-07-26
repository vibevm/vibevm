# Phase T — the worker prompt, for an account that cannot see the other one {#root}

<status stage="spec" state="plan" comment="drafted 2026-07-26; template, not yet exercised"/>

**Why this file exists.** Two worker accounts run in parallel and **know nothing
about each other** — native sub-agent orchestration works only *inside* one
account, so there is no coordinator spanning them and no runtime channel between
them. Everything they need must therefore be **decided in advance and baked into
two self-contained prompts**. This is the template; §4 is the fill-in.

Design it sits on: [`PHASE-T-SPEC.md` §13](PHASE-T-SPEC.md#parallel).

---

## 1. What the no-coordination constraint actually changes {#constraint}

- ##W-STATIC-PARTITION-ONLY **The split is static or it does not exist.** Neither account can ask
  «which cells are yours?», so the answer must already be in its prompt as a
  literal path list. §13.2's component packing runs **before** either starts.
- ##W-SCAFFOLD-RUNS-ON-MAIN-FIRST **The scaffold pass is the boss's, on `main`, committed and pushed
  before either account is launched.** It cannot be one of the two — the other
  would have to wait for it, and waiting needs a channel neither has. Both
  prompts then name **the same starting commit**, which is the only
  synchronisation point in the whole design and it happens before parallelism.
- ##W-NEITHER-AGENT-MERGES **Neither agent merges into `main`.** Each commits on its own branch
  and stops. Not because the merge is hard — the file sets are disjoint, so git
  resolves it automatically — but because **§13.5's check requires seeing both
  branches**, and an agent that merges its own can only see one side. The
  verification is the point of the scaffold design; handing it to someone who
  structurally cannot perform it throws the design away and keeps the ceremony.

## 2. The gotcha that would otherwise cost an hour {#worktree-gotcha}

##W-LONGPATHS **`git worktree add` on this repository overflows Windows MAX_PATH** on the
deep `vibedeps/` paths, and the failure is opaque. Provisioning must use
`-c core.longpaths=true`. This is a paid-for fact from the fractality work
(F19), recorded in the host `CLAUDE.md` ledger, and it applies verbatim here.

```bash
git -c core.longpaths=true worktree add -b <BRANCH> <WORKTREE_DIR> <START_COMMIT>
```

A separate worktree is not a nicety: it gives the run its **own `target/`**, so
two accounts on one machine do not contend on the cargo build lock for every
compile.

## 3. The template {#template}

*Fill `<BRANCH>`, `<WORKTREE_DIR>`, `<START_COMMIT>` and the scope list; hand
the result to one account. Nothing else in it varies between the two.*

---

> You are writing tests for the vibevm project, Phase T of campaign
> `packages-2026-09`. You are one of **two** independent workers running at the
> same time. **You cannot see the other and must never wait for it.** Everything
> you need is in this prompt.
>
> **Set up an isolated worktree first, and use the flag — without it the command
> fails opaquely on this repository:**
>
> ```bash
> cd <REPO_ROOT>
> git fetch
> git -c core.longpaths=true worktree add -b <BRANCH> <WORKTREE_DIR> <START_COMMIT>
> cd <WORKTREE_DIR>
> ```
>
> Work only inside `<WORKTREE_DIR>` from that point on.
>
> **Your scope — you may write ONLY these files. Every one already exists and is
> already declared; you fill them, you never create or delete one:**
>
> ```
> <LITERAL PATH LIST, one per line>
> ```
>
> A file outside that list is out of bounds, including any `Cargo.toml`, any
> `mod.rs`, and anything under `spec/`, `campaigns/`, `crates/vendor/`. If you
> believe you need one, **stop and report** — that is a design question and the
> reviewer answers it. Do not add a dev-dependency; if a test needs one that is
> absent, that fact is not testable in this pass and you report it.
>
> **How to write each test — follow the routine, do not improvise.** It is
> `campaigns/packages-2026-09/PHASE-T-SPEC.md` §3.1, and §3.2's banned list and
> §3.3's worked pair are the parts to read twice. In one line: **the fact's text
> is the oracle, the code is not** — you write the expected value as a literal
> *before* running anything, and if you cannot name it from the fact's words,
> the fact is not testable and you say so instead of guessing.
>
> **Do not read the implementation body.** Read the signature, the types, the
> doc comment, and the fact. Before submitting any test, run the hand test
> (§3.4): cover the implementation and say why the assertion is right. If the
> only answer is «that is what it returned», delete it.
>
> **Three tests per fact, of three different kinds** — `canonical_…`,
> `boundary_…`, `negative_…` (`property_…` where a fact names no failure). Two
> honest kinds and a stated reason beat three where one is invented.
>
> **Per file, exhibit one test red.** After it passes, change its expected
> literal to a wrong value, confirm it fails, restore it, and paste both outputs
> in your report. A file with no red exhibit is not accepted. This is not
> mutation coverage — it is the check that the test runs at all.
>
> **Tier every test as you write it**, never afterwards: `#[ignore]` marks the
> slow tier (needs an external tool, a network, or real sample counts); a test
> touching a tempdir or a fixture goes in `spec_tests_io.rs`; everything pure
> stays in `spec_tests.rs`.
>
> **Commit on your branch. Do NOT merge, do NOT push to `main`, do NOT rebase,
> never force anything.** Conventional Commits: `test(<crate>): <why>` under 60
> characters, blank line, a body explaining why. **Never name a model, an agent,
> or an AI tool anywhere** — not in a commit message, not in a comment, not in a
> test name. That is a hard repository rule.
>
> **Verify before you report:**
>
> ```bash
> cargo fmt --all
> cargo test --workspace          # your new tests green
> ```
>
> **Report:** per file — tests written, kinds used, tier assigned; every fact you
> returned as untestable with the sentence from routine step 1; the red exhibit
> for each file; every semantic problem you saw and did **not** fix; and your
> branch name and commit hashes. **Do not fix a semantic problem** — a fact that
> looks wrong is a finding, and a test that fails against working code is the
> most valuable thing you can produce, not a failure.

---

## 4. What differs between the two prompts {#fill-in}

**Only two things.** Everything above is identical, which is what makes the two
runs independent.

| slot | worker A | worker B |
|---|---|---|
| `<BRANCH>` | `phase-t/batch-a` | `phase-t/batch-b` |
| `<WORKTREE_DIR>` | `../vibevm-t-a` | `../vibevm-t-b` |
| `<START_COMMIT>` | **the scaffold commit — the same sha for both** | ditto |
| scope list | component set A | component set B |

- ##W-BRANCH-NAMES-CARRY-NO-TOOL Branch names carry the **scope**, never the tool. The attribution
  policy forbids model, agent and AI-tool names in branch names as plainly as
  in commit messages.
- ##W-SAME-START-COMMIT **Both start from the same commit, and it is the scaffold's.** Any
  later commit and the two trees disagree about which files exist, which is the
  one thing the scaffold pass was run to guarantee.

## 5. The boss's half {#boss}

Before: pack the components (§13.2), run the scaffold (§13.3), commit, push,
record the sha. After **both** report:

```
1. git merge phase-t/batch-a          # fast-forward
2. git merge phase-t/batch-b          # disjoint files → automatic
3. files(A) ∩ files(B)                → MUST be empty
4. files(A) ∪ files(B) == scaffold    → MUST be equal; name any gap
5. bash tools/self-check.sh           → real exit code
```

- ##W-CHECK-EVEN-WHEN-CLEAN **Run step 3 even when the merge was clean — especially then.** A
  clean merge over an overlapping partition means one worker's file silently
  won; it does not mean the partition held. That is precisely the shape this
  campaign has now found seven times: a green result that reports what was
  checked and says nothing about what was covered.

## 6. If the accounts are on different machines {#different-machines}

The design does not change — the split is static, so nothing needs a shared
filesystem. Replace the worktree step with a clone at the scaffold commit, and
have each account push its branch. **`main` still moves only through the boss**,
and §5's checks are unchanged.
