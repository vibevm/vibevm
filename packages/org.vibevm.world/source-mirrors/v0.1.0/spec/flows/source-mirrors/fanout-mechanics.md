# Fan-out mechanics {#root}

**Scope of this document.** This file defines the machinery of the
fan-out: the committed **target manifest**, the fan-out **procedure**
(fetch, verify ancestry, push fast-forward-only, report), the
**fail-loud** semantics of a diverged target, the read-only **drift
check**, bringing an accepted web merge **home**, why deletions and
rewrites **do not propagate**, a **reference implementation**, and how
to pin the never-`--force` invariant as a **test**. The model this
serves is in [`SOURCE-MIRRORS-PROTOCOL.md`](SOURCE-MIRRORS-PROTOCOL.md);
the daily rhythm in [`daily-loop.md`](daily-loop.md).

## The committed target manifest {#manifest}

The set of hosts lives in a small **committed** file at the repo root —
TOML or YAML, whatever the project already reads. It is reviewed like
code, because it *is* infrastructure.

```toml
schema = 1

[[target]]
name = "host-a"
url  = "git@host-a.example:org/project.git"
mode = "push"            # the maintainer pushes mainline here
refs = ["main", "tags"]  # what to mirror
audience = "region-1"

[[target]]
name = "host-b"
url  = "git@host-b.example:org/project.git"
mode = "push"
refs = ["main", "tags"]
audience = "region-2"
```

A **machine-local git remote cannot serve this role.** Two reasons:

| Requirement | Local remote | Committed manifest |
|-------------|--------------|--------------------|
| Shared with every contributor | No — lives in one `.git/config` | Yes — checked into the tree |
| Verifiable in CI / a sweep | No — not visible to CI | Yes — a file CI can read |
| Reviewed when a host changes | No — silent edit | Yes — shows up in a diff |

The manifest carries **no credentials.** Authentication is the
maintainer's per-host keys, held in the agent or SSH config — never in
the tree. Adding a host is one `[[target]]` block. A `self-pull` target
(a host that mirrors *itself* from elsewhere) is listed with
`mode = "self-pull"`: the tool does not push to it, only verifies it is
level with mainline.

## The fan-out procedure {#procedure}

Fan-out pushes mainline to every `push` target. The shape is always the
same four steps, per target:

1. **Fetch** the target's `main` (by URL, read-only).
2. **Verify ancestry** — the target's `main` must be an *ancestor* of
   local mainline. If it is not, the target has diverged: abort *that*
   target, loud (§fail-loud). Do not touch it.
3. **Push fast-forward-only** — `main` and tags, never `--force`.
4. **Report** the target as `ok`, `sync` (already level), or a named
   `DRIFT`.

The push is **by URL**, from the manifest, so the manifest is the one
source of truth for the target set — not a pile of git remotes that can
drift out of step with it. This is the *only* way history reaches a host:
not `git push host-a`, not a click in a web UI — the fan-out.

## Fail-loud semantics {#fail-loud}

A non-fast-forward on a target means that host carries a `main` your
mainline does not — almost always a direct write or a force-push to
that host. The fan-out's response is fixed:

- **Abort that target**, with a message naming the divergence (the
  host, and the commits it has that mainline lacks).
- **Never `--force`.** The tool has no force path to reach for.
- **Do not block the other targets.** A divergence on host B does not
  stop host A from receiving its legitimate fast-forward.

> A diverged target is a signal to investigate, never something to
> silently clobber.

Reconciliation is deliberate and manual: fetch the host, inspect the
divergent commits, merge what is wanted **into mainline**, then re-fan
([`daily-loop.md` §drift](daily-loop.md#drift)).

## The read-only drift check {#drift-check}

A `--check` mode answers "is everyone level?" without pushing anything:

```sh
project-mirror --check     # read-only; non-zero exit on drift
```

It fetches each target and compares to mainline. `sync` everywhere means
all hosts equal mainline; a `DRIFT` line names a host that has moved. It
writes nothing — safe as a pre-flight before a fan-out or inside a sweep.

## Bringing an accepted web merge home {#bring-home}

When a PR is merged through a host's web UI, *that host's* `main` is now
ahead of mainline. It is **not integrated** until it is brought home.
Do that first, then fan out:

```sh
git fetch <host-a-url> main
git merge --ff-only FETCH_HEAD   # fast-forward local mainline
project-mirror                   # now fan out to everyone else
```

The `--ff-only` is load-bearing: if local mainline cannot fast-forward
to the host's `main`, your tree has commits the host lacks, and the
merge must be reconciled by hand before fan-out. The host you pulled
from becomes a no-op on the next fan-out; every other host catches up.

## Deletions and history rewrites do not propagate {#no-propagate}

Two things the fan-out deliberately will **not** carry:

| Action on one host | Propagates? | Why |
|--------------------|-------------|-----|
| Delete a branch | **No** | An accidental deletion must not cascade to every host |
| Rewrite / force-push history | **No** | The fan-out is fast-forward-only; a rewrite is a divergence, and divergences fail loud |

Deleting a branch everywhere is a deliberate, per-host act. The fan-out
only ever *advances* refs, so no single mistake can subtract history
from the whole fleet at once.

## Reference implementation {#reference}

A fan-out is about fifteen lines of `sh`. There is **no `--force` in it,
by law** — the absence is the invariant, not an oversight:

```sh
#!/bin/sh
# Fan out local mainline to every push target in the manifest.
# There is deliberately NO --force here. A non-fast-forward target
# is a divergence to investigate by hand, never something to clobber.
set -eu
branch=main

# read_targets emits: "<name> <url> <mode>" per line from the manifest.
read_targets | while read -r name url mode; do
    [ "$mode" = "push" ] || { echo "$name: skip ($mode)"; continue; }

    # Fail-loud ancestry gate: the target's main must be an ancestor
    # of local mainline, or we refuse to touch it.
    remote_head=$(git ls-remote "$url" "refs/heads/$branch" | cut -f1)
    if [ -n "$remote_head" ] && ! git merge-base --is-ancestor "$remote_head" "$branch"; then
        echo "$name: DRIFT — host has commits mainline lacks; reconcile by hand"
        continue
    fi

    # Fast-forward-only push. No '+', no --force, ever.
    if git push "$url" "$branch:$branch" && git push --tags "$url"; then
        echo "$name: ok"
    else
        echo "$name: push failed"
    fi
done
```

Adapt `read_targets` to the manifest format the project uses. The two
invariants to preserve when you port it: the **ancestry gate** before
every push, and the **absence of any force path**.

## Pin the invariant with a test {#never-force-test}

A rule with no checker is a wish. Build the push command in one place
and assert, in a test or a CI step, that it can never emit a force:

```sh
# CI guard: no force flag may appear in the fan-out script.
if grep -nE -- '--force|[[:space:]]-f([[:space:]]|$)|push[^|]*\+' fanout.sh; then
    echo "FAIL: a force path exists in the fan-out — remove it" >&2
    exit 1
fi
```

Better still, in a project with a real test suite, factor the push
arguments into one pure function and unit-test that its output never
contains `--force`, `-f`, or a `+`-prefixed refspec for any ref shape.
The invariant is then **runnable capital, not a prose promise** — a rule
you cannot run is a rule you cannot trust.

## Summary {#summary}

- The target set is a committed, credential-free manifest — shared,
  CI-visible, reviewed like code. A local git remote cannot serve it.
- Fan-out per target: fetch, verify the target is an ancestor of
  mainline, push fast-forward-only by URL, report.
- A non-fast-forward aborts *that* target loud, names the divergence,
  and never forces. Reconcile into mainline by hand, then re-fan.
- A `--check` mode probes drift read-only; bring a web merge home with
  `merge --ff-only` before fanning out.
- Deletions and history rewrites do not propagate — a safety choice.
- The reference fan-out is ~15 lines with no force path; pin that
  absence with a test or a CI grep.
