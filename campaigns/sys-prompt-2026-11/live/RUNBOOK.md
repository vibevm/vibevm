# P9 — the live check, Claude Code only: runbook

**Authorisation.** `HARNESS-DELIVERY.md` D13, as the owner narrowed it on
2026-11-10: **Claude Code only**, rows C1–C8; before the release is
published; after P7; and run by the **central session**, because a worker
neither starts an agent session nor spends the owner's subscription. Codex,
Qwen Code, Gemini CLI, OpenCode, pi and dsh stay **unauthorised** — their
rows remain research and their delivery default stays `read`. The one Codex
row that needs no authorisation is the offline X0, which is already done
(`cargo test -p vibe-cli --test cli_prompt_stale` with
`VIBE_PROMPT_CODEX_PROBE=1`).

**Launcher.** Every session below starts through the owner's own launcher,
`vibe agent claude2` — a wrapper that sets `CLAUDE_CONFIG_DIR=~/.claude2`
and execs the real program. Declare it once in `<settings>/agents.toml`:

```toml
[launchers.claude2]
kind = "claude"
command = ["claude2"]
config_dir = "~/.claude2"
```

`config_dir` matters here and nowhere else: the wrapper sets the variable
internally, so without it `vibe` cannot tell where the session logs are, and
the cache diagnostics would have to guess. Confirm with `vibe agent list`
that the row reads `claude2 (kind claude)` with that config directory before
starting step 1.

**Order is the owner's, and it is not negotiable.** The short sample first,
its cache repeat second, and only then anything at size. The reason is that a
failure at size is ambiguous — delivery, truncation or cache could each
explain it — while a failure on a 3 KB lane is not.

**Fixtures.** `campaigns/sys-prompt-2026-11/live/sample/` holds the sample
project: a static boot dependency so a real `STATIC.md` lane exists, authored
boot sources, an entrypoint, a template, and one nested sub-project. Every
string in it is a canary or neutral filler; there is **no personal data**.
`make-sample.sh` copies it to a scratch directory, points its registry at the
copy, optionally pads it, and installs. Build a fresh copy per step — a
session that reuses a directory reuses its project-scoped log directory too.

The canaries, and what each one answers:

| Canary | Where | Answers |
|---|---|---|
| `CANARY-TEMPLATE-SLATE` | `SYSTEM-TEMPLATE.md` | did the API prefix's frame reach anything it should not? (It must **not** be in the agent lane.) |
| `CANARY-ENTRYPOINT-QUARTZ` | `AGENTS.md`, outside the block | did the harness load the instruction file itself? |
| `CANARY-LANE-HEAD-OBSIDIAN` | first lane segment's body | did the lane arrive at all? |
| `CANARY-LANE-TAIL-PUMICE` | last line of that body | did the **tail** arrive, or was the slot truncated? |
| `CANARY-INDEX-CORE-BASALT` | `00-core.md` | did band U arrive inside the lane? |
| `CANARY-INDEX-USER-GNEISS` | `90-user.md` | the same, for the second band-U file |
| `CANARY-NESTED-TUFF` | `nested/AGENTS.md` | did downward discovery pull a sub-project's instruction file in? |
| `CANARY-NESTED-LANE-SCORIA` | the nested project's lane | did the agent read a nested project's lane? It must not. |

**Cost rule, owner 2026-11-11 — small projects and local inference.**
«для экономии токенов пожалуйста запускай qwen на супер маленьких проектах и
на локальном инференсе … Все равно для задач вычисления префикса нам
неинтересно гонять большие проекты.» So, for every harness this campaign
measures from here on:

- **Live sessions use the local model** — `homelocal/qwen3.8-27b`, a
  56,000-token context — and the **tiny** sample, the unpadded ~2.5 KB lane,
  or at most a medium one that fits 56k with headroom.
- **The real-size proofs are offline**, against the fake provider, where a
  202 KB lane costs nothing: `opencode-prove.sh` (the whole-request proof of
  `#SYS-SUPPLIED-BASE`, five legs in one run, judged by `opencode-prove.py`),
  and the narrower `opencode-capture.sh`, `qwen-capture.sh` and
  `opencode-subagent-capture.sh` it supersedes for OpenCode. A delivery
  question is about *bytes in a request*, and a recorder answers it better
  than a model does.
- A live run on a large lane or a metered model needs the owner to ask for
  it. The rows that were taken that way before this rule are kept as
  evidence and are **not** to be repeated.

**The one prompt.** Every delivery step uses the same prompt, so the answers
compare:

> Quote the last `vibe:lane-end` line you can see, verbatim. Then list every
> canary word of the form `CANARY-…` that is anywhere in your context. Then
> list every file you have read with a tool in this session, exactly, and name
> none you did not open. Do not read any file to answer this.

**Two amendments the OpenCode `launch` tier forced on that prompt**, both
findings rather than conveniences, and both in `opencode-live-check.sh`:

- **"The last `vibe:lane-end` line" is ambiguous in `launch` delivery**, and a
  model that answers it with the managed block's own indented *format example*
  is **right**: the harness injects `AGENTS.md` after the agent's `system`, so
  that example genuinely is the last such line. Measured on the local 27B
  model, which did exactly that. The question must name the marker's own file
  — "the line containing both `vibe:lane-end` and `90-user`" — which only the
  delivered marker carries.
- **A supplied base needs a fourth question**, because conditions (0)–(v) are
  byte comparisons and none of them can tell a base that arrived from a base
  that is still obeyed. So the session is also asked to state a rule that
  exists **only** in the harness's own base text — for OpenCode, the edit
  tool's `oldString`-occurs-once rule — and the script refuses to run if the
  lane mentions it, so the premise is checked rather than assumed.

**Recording.** For each step write, into this directory's `RESULTS.md`
(created by the first step): the step id, the exact command, the harness
version (`claude2 --version`), the model if the session reports it, the
answer verbatim, the usage rows `read-usage.sh` printed, and the verdict.
A step with no recorded answer did not run.

---

## Step 1 — the short sample: does the lane arrive, and does the router stop there?

Rows: **C1** (delivery) and the "reads nothing more" half of the router.

```sh
cd /fast/tmp                     # or any scratch location outside the repo
VIBE=/path/to/vibe campaigns/sys-prompt-2026-11/live/make-sample.sh live-1
cd live-1
vibe prompt check                # must print `system prompt: equal`
tail -1 vibevm/vibespecs/boot/AGENT-LANE.md    # note this line
vibe agent claude2
```

Then issue **the one prompt** and exit.

**Pass**, all four:

1. The quoted `vibe:lane-end` line is byte-identical to the one `tail -1`
   printed — including the node id `org.live/sample`, the path, and the
   12-digit id.
2. The canaries listed include `CANARY-LANE-HEAD-OBSIDIAN`,
   `CANARY-LANE-TAIL-PUMICE`, `CANARY-INDEX-CORE-BASALT` and
   `CANARY-INDEX-USER-GNEISS` — the lane and the whole of band U.
3. The files-read list is **empty of lane files**: no `STATIC.md`, no
   `INDEX.md`, no `00-core.md`, no `90-user.md`. Router rule 1 says a file
   whose pair is present is already read, and this is the only way to see
   whether the model obeyed it.
4. `CANARY-TEMPLATE-SLATE` is **absent**. The slot carries the lane only;
   the template is written for an API runner with no base prompt and must
   not reach a harness's own.

**Fail** ⇒ `launch` defaults back to `read` for Claude Code, and
`[system_prompt.harness.claude] delivery`'s default changes in code, not in
prose. Distinguish the two failures before concluding: if the canaries are
absent the **slot** did not arrive (check `vibe agent claude2 --help`-level
argv with `vibe prompt flags --harness claude2`); if the canaries are present
but the files were read as well, delivery works and the **router's rule 1**
did not persuade the model, which is a wording problem and not a mechanism
one.

**Do not proceed to step 3 while step 1 fails.** Step 2 may still run: a
cache read of a lane the model ignores is still a cache read.

## Step 2 — the same again: is there a cache read?

Row: **C2**, with **C3** as its second leg.

```sh
date +%s > /tmp/live-2.since      # before the session
cd live-1 && vibe agent claude2   # a NEW session, same project, same prompt
# after it exits:
campaigns/sys-prompt-2026-11/live/read-usage.sh \
    ~/.claude2/projects "$(cat /tmp/live-2.since)"
```

Also re-read step 1's own numbers, with step 1's start time, so the pair can
be compared.

**Pass:** the second session's rows show `cache_read` at or above the lane's
token count, while the first session's showed `cache_creation` at a
comparable figure instead. Estimate the lane's token count from the
provider's own report, never from a bytes-per-token rule: the lane's byte
size is on `vibe prompt path --part lane --json`, and the usage rows are the
measurement.

**Second leg (C3).** Build a second copy at another path, with a different
`git status` — `make-sample.sh live-1b`, then `git init && git add -A` in it —
and run the same session there. A cache read of the lane **still** occurring
is what proves `--exclude-dynamic-system-prompt-sections` keeps per-machine
text out of the cached prefix. No cache read there, with one in `live-1`,
means that flag is not doing what its help text says, and the flag's value is
recorded as unproven rather than assumed.

**Fail** ⇒ the cache claim stays unmeasured. It does **not** by itself change
the delivery default: `launch` is justified by delivery, and the cache is the
economic argument on top. Record it as unmeasured and say so in the release
note.

## Step 3 — at size: the padded variant, then this repository

Row: **C1 again**, at the size a real project has. Only after steps 1 and 2.

```sh
VIBE=/path/to/vibe campaigns/sys-prompt-2026-11/live/make-sample.sh live-3 200000
cd live-3 && vibe prompt path --part lane --json   # confirm 150–300 KB
vibe agent claude2
```

The padding goes **before** the tail canary, so the last line of the lane's
own body is still `CANARY-LANE-TAIL-PUMICE` and quoting it still proves the
tail arrived.

**Pass:** the same four conditions as step 1, at a lane between 150 KB and
300 KB. The tail canary is the whole measurement: a truncating slot drops it
silently, exactly as Codex drops an instruction file's tail.

**Then this repository.** From a clean checkout of vibevm with a
`SYSTEM-TEMPLATE.md` in place:

```sh
cd /fast/git/vibevm
vibe prompt build && vibe prompt check
vibe agent claude2
```

**Pass:** the agent quotes the last `vibe:lane-end` line of this
repository's own ~312 KB lane, reports no read of `STATIC.xml` or any
`INDEX.md` file, and — the one extra question this leg asks — reports the
`@spec://` tokens it can see **without having tried to resolve any of them**
as paths. The lane carries 68 such tokens in `STATIC.xml` and 74 in all. A
reported attempt to resolve one is the evidence `#SYS-IMPORT-INERTNESS` is
waiting for, and it belongs in `RESULTS.md` whether it happens or not.

**Fail at size with step 1 passing** ⇒ Claude Code's file slot has a size
limit, and the number it broke at is the finding. Record the exact lane size
that failed and the largest that worked; the fallback is `read`, which has
no such limit because the files are read with a tool.

## Step 4 — classic mode: does the router work with no markers at all?

Row: **C4**, and the step that protects every project that never opted in.

```sh
VIBE=/path/to/vibe campaigns/sys-prompt-2026-11/live/make-sample.sh live-4
cd live-4
rm vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md
python3 - <<'PY'
import pathlib
m = pathlib.Path("vibe.toml")
t = m.read_text()
t = t[: t.index("[system_prompt]")]
m.write_text(t)
PY
vibe install --offline --assume-yes
ls vibevm/vibespecs/boot          # must show no SYSTEM*, no AGENT-LANE.md, no .gitignore
claude2                            # plain, NOT through `vibe agent`
```

Issue the one prompt.

**Pass**, all three:

1. The canaries `CANARY-LANE-HEAD-OBSIDIAN`, `CANARY-LANE-TAIL-PUMICE`,
   `CANARY-INDEX-CORE-BASALT` and `CANARY-INDEX-USER-GNEISS` are all
   reported.
2. The files-read list names `vibevm/vibespecs/boot/STATIC.md` **first**, then
   `00-core.md` and `90-user.md` in INDEX order — which is router rule 4
   obeyed, with no markers in context to shortcut it.
3. Each lane file was read **once**. Two reads of one file means rule 4's
   "which you have not already read in this session" did not hold.

**Fail** ⇒ the router block's wording is wrong for the tier every
non-launching harness uses, and that is more serious than a launch failure:
it is the path a project that never opted in takes. It blocks the release
rather than only the default.

Run the same prompt once more through `vibe agent claude2` **in this same
opted-out project**: `vibe agent` must say in one line that the project has
not opted in and start the harness unchanged, and the session must behave
exactly as the plain one. That is the safe-by-construction property the whole
one-block design rests on.

## Step 5 — the rows the owner's three do not cover (C5, C6, C7, N1)

These are inside D13's C1–C8 authorisation and are run after step 3, in this
order, because each is cheap once a working session exists.

**C5 — a subagent must not inherit the slot.** In a `vibe agent claude2`
session on `live-1`, ask for one subagent whose only task is to report
whether it can see any line matching `vibe:lane-begin`, and to report nothing
else. **Pass:** it sees none. **Fail** ⇒ a quiet worker carries the lane in
context, against `#SYS-WORKER-EXCLUSION`, and `launch` must not be Claude
Code's default at all — this is the one failure that overrides a passing
step 1.

**N1 — a nested project must not be pulled in.** From `live-1`, ask the agent
to open `nested/AGENTS.md` with a tool and then report every canary it can
see. **Pass:** `CANARY-NESTED-TUFF` appears (it read the file it was asked
for), `CANARY-NESTED-LANE-SCORIA` does **not** (it did not follow the nested
project's router), and the root lane is not re-read. The nested block's
scope sentence and its different node id are what make this pass.

**C6 — a repeated file flag.** Run `claude2` by hand with
`--append-system-prompt-file` twice, naming two different small files, and
ask which text arrived. **Pass:** whichever answer, it is **recorded** —
`HARNESS-DELIVERY.md` §13 lists "last flag wins" as an incomplete claim, and
this closes it. `vibe agent` already refuses a user-supplied second flag, so
nothing depends on the answer; it is documentation.

**C7 — a resumed session after a lane change.** In `live-1`, edit the
carrier's boot source, `vibe install`, then **resume** the earlier session.
**Pass:** either the new lane arrives, or the old one does and the agent
notices the id mismatch against the router's expected value and says so.
**Record which**: Claude Code's `--system-prompt-snapshot` is on by default
and a resumed conversation resends the recorded prompt until compaction, so
the honest outcome is likely the second, and the staleness rule is then the
only thing covering it.

**C8 — Windows.** Not runnable on this machine. Record as **not run**, with
the reason, rather than as passed.

---

## Closing the step

P9 is complete when `RESULTS.md` carries a verdict for every row above,
including the ones that did not run and why. Then, in one commit:

- any change to a delivery **default** that a failure forces, in code;
- `HARNESS-DELIVERY.md` §10's table updated from research to measurement,
  row by row, with the evidence labels changed from `[I]`/`[R]` to `[V]`
  only where a session actually showed it;
- `PROP-076#SYS-HARNESS-DELIVERY` amended with what was measured, and
  `#SYS-CACHE-FACTS` left exactly as it is unless a cache read was observed;
- `docs/system-prompt/README.md`'s "not measured here" markers removed only
  for the rows that are now measured;
- `LOG.md` and `PLAN.md` §10.

Nothing here spends more than the sessions it names, and no step calls a
provider outside an interactive Claude Code session the owner started.
