# The daily loop {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file is the maintainer's operating
guide: the *shape of a normal day* (commit on mainline, fan out at
natural checkpoints), *handling reported drift* (investigate, reconcile
into mainline, re-fan — never clobber), *onboarding a host*, and
*offboarding a host*. @status:impl/done

@fact:sibling-document-pointers The model behind it is in
[`SOURCE-MIRRORS-PROTOCOL.md`](SOURCE-MIRRORS-PROTOCOL.md); the machinery
in [`fanout-mechanics.md`](fanout-mechanics.md). @status:impl/done

## The loop in one line {#loop}

> @fact:THE-LOOP-IN-ONE-LINE A change arrives somewhere → you review and accept it there → you
> bring it into local `main` → you fan out → it is everywhere. @status:impl/done

@fact:everything-below-is-that-sentence-expanded Everything below is that sentence, expanded. @status:impl/done

## A normal day {#normal-day}

@fact:YOU-WORK-ON-MAINLINE-THE-WAY-YOU-ALWAYS-HAVE You work on mainline the way you always have. @status:impl/done

@fact:COMMIT-LOCALLY-IN-ATOMIC-COMMITS Commit locally, in atomic
commits, with well-formed messages. @status:impl/done

@fact:MULTI-HOMING-DOES-NOT-CHANGE-HOW-YOU-AUTHOR-HISTORY Nothing about being multi-homed
changes how you *author* history. @status:impl/done

@fact:FAN-OUT-AT-NATURAL-CHECKPOINTS What is new is one habit: **fan out at natural checkpoints.** @status:impl/done

@fact:not-after-every-commit Not after
every commit — after a coherent unit of work, the same moments you would
have pushed in a single-host project: @status:impl/done

| Moment | Action |
|--------|--------|
| @fact:ROW-MOMENT-FINISHED-A-FEATURE-SLICE Finished a feature slice @status:impl/done | Fan out @status:impl/done |
| @fact:ROW-MOMENT-CUT-A-RELEASE Cut a release / tag @status:impl/done | Fan out (tags travel with `main`) @status:impl/done |
| @fact:ROW-MOMENT-END-OF-A-WORK-SESSION End of a work session @status:impl/done | Fan out as the wind-down step @status:impl/done |
| @fact:ROW-MOMENT-BEFORE-STEPPING-AWAY Before stepping away for a while @status:impl/done | `--check`, then fan out if behind @status:impl/done |

```sh
# ... a session's worth of commits on mainline ...
project-mirror --check     # optional: confirm where the hosts stand
project-mirror             # push main + tags to every host, ff-only
```

@fact:THE-FAN-OUT-IS-A-DELIBERATE-ACT-NOT-A-DAEMON The fan-out is a **deliberate act, not a daemon.** @status:impl/done

@fact:NO-BACKGROUND-JOB-RACES-YOUR-COMMITS-TO-THE-HOSTS There is no
background job racing your commits to the hosts; you decide when the
world sees the new history. @status:impl/done

@fact:THAT-IS-WHAT-KEEPS-MAINLINE-THE-SINGLE-WRITER That is what keeps mainline the single
writer — nothing pushes but you, when you say so. @status:impl/done

## Integrating an accepted change {#integrate}

@fact:a-contribution-reaches-mainline-one-of-two-ways A contribution reaches mainline one of two ways, depending on where you
accepted it: @status:impl/done

- @fact:ROUTE-YOU-MERGED-IT-VIA-A-HOSTS-WEB-UI **You merged it via a host's web UI.** That host's `main` is now
  ahead. Bring it home *before* fanning out: @status:impl/done
  ```sh
  git fetch <that-host-url> main
  git merge --ff-only FETCH_HEAD
  project-mirror
  ```
  @fact:bring-home-pointer See [`fanout-mechanics.md` §bring-home](fanout-mechanics.md#bring-home). @status:impl/done
- @fact:ROUTE-YOU-INTEGRATE-LOCALLY **You integrate locally** — a fork branch, or an emailed patch. On
  `main` with a clean tree: @status:impl/done
  ```sh
  git fetch <contributor-url> <their-branch>   # or: git am < patch.eml
  git merge --ff-only FETCH_HEAD               # or your chosen merge
  project-mirror
  ```

@fact:EITHER-WAY-THE-CHANGE-LANDS-IN-MAINLINE-FIRST Either way, the shape is identical: the change lands in **mainline
first**, and only the fan-out puts it on the hosts. @status:impl/done

@fact:A-WEB-UI-MERGE-BUTTON-IS-AN-INBOX-EVENT A web-UI "Merge"
button is an *inbox event*, not integration. @status:impl/done

## Handling reported drift {#drift}

@fact:DRIFT-MEANS-THE-HOST-CARRIES-A-MAIN-YOU-DO-NOT `--check` (or a host warning) reports **DRIFT** on a host: it carries a
`main` your mainline does not. @status:impl/done

@fact:drift-is-almost-always-a-direct-write-or-a-force-push Almost always this is a direct write or a
force-push to that host — exactly the thing the model forbids, surfaced
loud instead of silently reconciled. @status:spec/done

@fact:DO-NOT-RE-RUN-THE-FAN-OUT-HOPING-IT-CLEARS Do **not** re-run the fan-out hoping it clears. @status:impl/done

@fact:THE-FAN-OUT-REFUSES-TO-FORCE-BY-DESIGN It will not — the
fan-out refuses to force, by design. @status:impl/done

@fact:reconcile-deliberately Reconcile deliberately: @status:impl/done

1. @fact:DRIFT-STEP-INVESTIGATE **Investigate.** Fetch the host and look at what it has that you do
   not. @status:impl/done
   ```sh
   git fetch <host-url> main
   git log --oneline main..FETCH_HEAD    # the host's extra commits
   ```
2. @fact:DRIFT-STEP-DECIDE **Decide.** Are those commits wanted? Usually someone pushed a real
   fix directly. Sometimes it is junk to discard. @status:impl/done
3. @fact:DRIFT-STEP-RECONCILE-INTO-MAINLINE **Reconcile *into* mainline.** Merge or cherry-pick the wanted
   commits onto mainline. Now mainline is ahead of the host again. @status:impl/done
   ```sh
   git merge FETCH_HEAD                  # or cherry-pick the good ones
   ```
4. @fact:DRIFT-STEP-RE-FAN **Re-fan.** `project-mirror` — the host fast-forwards cleanly,
   because mainline now contains its history. @status:impl/done

> @fact:A-DIVERGED-TARGET-IS-A-SIGNAL-TO-INVESTIGATE A diverged target is a signal to investigate, never something to
> silently clobber. @status:impl/done

@fact:NEVER-FIX-DRIFT-BY-OVERWRITING-THE-HOST The one thing you never do is "fix" drift by overwriting the host to
match mainline. @status:impl/done

@fact:overwriting-discards-the-work-that-caused-the-drift That discards whatever real work caused the drift —
which is precisely the data the loud failure was protecting. @status:spec/done

## Onboarding a new host {#onboard}

@fact:adding-a-host-is-deliberately-small Adding a host is deliberately small: @status:impl/done

1. @fact:ONBOARD-STEP-CREATE-AN-EMPTY-REPO **Create an empty repo** on the new host (no README, no initial
   commit — it must be empty so the first fan-out is a clean
   fast-forward from nothing). @status:impl/done
2. @fact:ONBOARD-STEP-ADD-ONE-MANIFEST-ENTRY **Add one manifest entry** — name, url, mode, refs — and commit it.
   The commit is the audit trail for "when did this host join". @status:impl/done
   ```toml
   [[target]]
   name = "host-c"
   url  = "git@host-c.example:org/project.git"
   mode = "push"
   refs = ["main", "tags"]
   audience = "region-3"
   ```
3. @fact:ONBOARD-STEP-ENSURE-ACCESS **Ensure access** — your key can push to it (`push` mode), or the
   host is configured to mirror itself (`self-pull` mode). @status:impl/done
4. @fact:ONBOARD-STEP-FIRST-FAN-OUT **First fan-out.** `project-mirror`. The new host receives the full
   history; every existing host is a no-op. @status:impl/done

@fact:THAT-IS-THE-ENTIRE-ONBOARDING That is the entire onboarding. @status:impl/done

@fact:the-model-was-built-for-a-living-host-set No re-architecture, no cutover — the
model was built for the host set to be *living*. @status:spec/done

## Offboarding a host {#offboard}

@fact:removing-a-host-is-smaller-still Removing a host is smaller still: @status:impl/done

1. @fact:OFFBOARD-STEP-DELETE-ITS-TARGET-ENTRY **Delete its `[[target]]` entry** from the manifest and commit the
   removal. The fan-out stops targeting it immediately. @status:impl/done
2. @fact:OFFBOARD-STEP-OPTIONALLY-ARCHIVE-THE-HOST-COPY **Optionally archive the host copy** — leave it read-only as a
   historical mirror, or delete the repo on that host. @status:impl/done

@fact:NOTHING-IS-LOST-EITHER-WAY Nothing is lost either way: **every remaining host
holds the full history of the declared refs, and mainline holds the whole
tree.** @status:impl/done

@fact:OFFBOARDING-NEVER-SUBTRACTS-A-COMMIT Offboarding a mirror never subtracts a commit from
the project — it only stops one replica from being kept current. @status:impl/done

@fact:the-host-set-can-shrink-as-freely-as-it-grew That
is the payoff of "every host is a complete replica": the host set can
shrink as freely as it grew. @status:spec/done

## Summary {#summary}

- @fact:SUM-AUTHOR-ON-MAINLINE-AND-FAN-OUT-AT-CHECKPOINTS Author on mainline as normal; fan out at natural checkpoints, never as
  a background daemon. @status:impl/done
- @fact:SUM-A-CHANGE-LANDS-IN-MAINLINE-FIRST A change lands in mainline *first*; the fan-out is what puts it on the
  hosts. A web-UI merge is an inbox event, not integration. @status:impl/done
- @fact:SUM-DRIFT-IS-RECONCILED-INTO-MAINLINE Drift is investigated and reconciled *into* mainline, then re-fanned —
  never cleared by clobbering the host. @status:impl/done
- @fact:SUM-ONBOARD-AND-OFFBOARD Onboard a host: empty repo → one manifest entry → first fan-out.
  Offboard: remove the entry, optionally archive the copy. @status:impl/done
- @fact:SUM-EVERY-HOST-HOLDS-THE-FULL-HISTORY Every host holds the full history of
  the refs the manifest declares for it (`main` + tags today; any deliberately
  declared branch later), so the set grows and shrinks without data loss. Refs
  outside that set live only where they were authored — the fan-out is
  replication of a declared line, not a backup of the whole tree. @status:impl/done
