# The daily loop {#root}

**Scope of this document.** This file is the maintainer's operating
guide: the *shape of a normal day* (commit on mainline, fan out at
natural checkpoints), *handling reported drift* (investigate, reconcile
into mainline, re-fan — never clobber), *onboarding a host*, and
*offboarding a host*. The model behind it is in
[`SOURCE-MIRRORS-PROTOCOL.md`](SOURCE-MIRRORS-PROTOCOL.md); the machinery
in [`fanout-mechanics.md`](fanout-mechanics.md).

## The loop in one line {#loop}

> A change arrives somewhere → you review and accept it there → you
> bring it into local `main` → you fan out → it is everywhere.

Everything below is that sentence, expanded.

## A normal day {#normal-day}

You work on mainline the way you always have. Commit locally, in atomic
commits, with well-formed messages. Nothing about being multi-homed
changes how you *author* history.

What is new is one habit: **fan out at natural checkpoints.** Not after
every commit — after a coherent unit of work, the same moments you would
have pushed in a single-host project:

| Moment | Action |
|--------|--------|
| Finished a feature slice | Fan out |
| Cut a release / tag | Fan out (tags travel with `main`) |
| End of a work session | Fan out as the wind-down step |
| Before stepping away for a while | `--check`, then fan out if behind |

```sh
# ... a session's worth of commits on mainline ...
project-mirror --check     # optional: confirm where the hosts stand
project-mirror             # push main + tags to every host, ff-only
```

The fan-out is a **deliberate act, not a daemon.** There is no
background job racing your commits to the hosts; you decide when the
world sees the new history. That is what keeps mainline the single
writer — nothing pushes but you, when you say so.

## Integrating an accepted change {#integrate}

A contribution reaches mainline one of two ways, depending on where you
accepted it:

- **You merged it via a host's web UI.** That host's `main` is now
  ahead. Bring it home *before* fanning out:
  ```sh
  git fetch <that-host-url> main
  git merge --ff-only FETCH_HEAD
  project-mirror
  ```
  See [`fanout-mechanics.md` §bring-home](fanout-mechanics.md#bring-home).
- **You integrate locally** — a fork branch, or an emailed patch. On
  `main` with a clean tree:
  ```sh
  git fetch <contributor-url> <their-branch>   # or: git am < patch.eml
  git merge --ff-only FETCH_HEAD               # or your chosen merge
  project-mirror
  ```

Either way, the shape is identical: the change lands in **mainline
first**, and only the fan-out puts it on the hosts. A web-UI "Merge"
button is an *inbox event*, not integration.

## Handling reported drift {#drift}

`--check` (or a host warning) reports **DRIFT** on a host: it carries a
`main` your mainline does not. Almost always this is a direct write or a
force-push to that host — exactly the thing the model forbids, surfaced
loud instead of silently reconciled.

Do **not** re-run the fan-out hoping it clears. It will not — the
fan-out refuses to force, by design. Reconcile deliberately:

1. **Investigate.** Fetch the host and look at what it has that you do
   not.
   ```sh
   git fetch <host-url> main
   git log --oneline main..FETCH_HEAD    # the host's extra commits
   ```
2. **Decide.** Are those commits wanted? Usually someone pushed a real
   fix directly. Sometimes it is junk to discard.
3. **Reconcile *into* mainline.** Merge or cherry-pick the wanted
   commits onto mainline. Now mainline is ahead of the host again.
   ```sh
   git merge FETCH_HEAD                  # or cherry-pick the good ones
   ```
4. **Re-fan.** `project-mirror` — the host fast-forwards cleanly,
   because mainline now contains its history.

> A diverged target is a signal to investigate, never something to
> silently clobber.

The one thing you never do is "fix" drift by overwriting the host to
match mainline. That discards whatever real work caused the drift —
which is precisely the data the loud failure was protecting.

## Onboarding a new host {#onboard}

Adding a host is deliberately small:

1. **Create an empty repo** on the new host (no README, no initial
   commit — it must be empty so the first fan-out is a clean
   fast-forward from nothing).
2. **Add one manifest entry** — name, url, mode, refs — and commit it.
   The commit is the audit trail for "when did this host join".
   ```toml
   [[target]]
   name = "host-c"
   url  = "git@host-c.example:org/project.git"
   mode = "push"
   refs = ["main", "tags"]
   audience = "region-3"
   ```
3. **Ensure access** — your key can push to it (`push` mode), or the
   host is configured to mirror itself (`self-pull` mode).
4. **First fan-out.** `project-mirror`. The new host receives the full
   history; every existing host is a no-op.

That is the entire onboarding. No re-architecture, no cutover — the
model was built for the host set to be *living*.

## Offboarding a host {#offboard}

Removing a host is smaller still:

1. **Delete its `[[target]]` entry** from the manifest and commit the
   removal. The fan-out stops targeting it immediately.
2. **Optionally archive the host copy** — leave it read-only as a
   historical mirror, or delete the repo on that host.

Nothing is lost either way: **every remaining host, and mainline, holds
the full history.** Offboarding a mirror never subtracts a commit from
the project — it only stops one replica from being kept current. That
is the payoff of "every host is a complete replica": the host set can
shrink as freely as it grew.

## Summary {#summary}

- Author on mainline as normal; fan out at natural checkpoints, never as
  a background daemon.
- A change lands in mainline *first*; the fan-out is what puts it on the
  hosts. A web-UI merge is an inbox event, not integration.
- Drift is investigated and reconciled *into* mainline, then re-fanned —
  never cleared by clobbering the host.
- Onboard a host: empty repo → one manifest entry → first fan-out.
  Offboard: remove the entry, optionally archive the copy.
- Every host holds the full history, so the set grows and shrinks
  without data loss.
