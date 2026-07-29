# Fan-out mechanics {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines the machinery of the
fan-out: the committed **target manifest**, the fan-out **procedure**
(fetch, verify ancestry, push fast-forward-only, report), the
**fail-loud** semantics of a diverged target, the read-only **drift
check**, bringing an accepted web merge **home**, why deletions and
rewrites **do not propagate**, a **reference implementation**, and how
to pin the never-`--force` invariant as a **test**. @impl/done

##sibling-document-pointers The model this
serves is in [`SOURCE-MIRRORS-PROTOCOL.md`](SOURCE-MIRRORS-PROTOCOL.md);
the daily rhythm in [`daily-loop.md`](daily-loop.md). @impl/done

## The committed target manifest {#manifest}

##THE-HOST-SET-LIVES-IN-A-SMALL-COMMITTED-FILE The set of hosts lives in a small **committed** file at the repo root —
TOML or YAML, whatever the project already reads. @impl/done

##THE-MANIFEST-IS-REVIEWED-LIKE-CODE It is reviewed like
code, because it *is* infrastructure. @impl/done

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

##A-MACHINE-LOCAL-GIT-REMOTE-CANNOT-SERVE-THIS-ROLE A **machine-local git remote cannot serve this role.** Two reasons: @impl/done

| Requirement | Local remote | Committed manifest |
|-------------|--------------|--------------------|
| ##ROW-REQ-SHARED-WITH-EVERY-CONTRIBUTOR Shared with every contributor @spec/done | No — lives in one `.git/config` @spec/done | Yes — checked into the tree @spec/done |
| ##ROW-REQ-VERIFIABLE-IN-CI Verifiable in CI / a sweep @spec/done | No — not visible to CI @spec/done | Yes — a file CI can read @spec/done |
| ##ROW-REQ-REVIEWED-WHEN-A-HOST-CHANGES Reviewed when a host changes @spec/done | No — silent edit @spec/done | Yes — shows up in a diff @spec/done |

##THE-MANIFEST-CARRIES-NO-CREDENTIALS The manifest carries **no credentials.** @impl/done

##AUTHENTICATION-IS-THE-MAINTAINERS-PER-HOST-KEYS Authentication is the
maintainer's per-host keys, held in the agent or SSH config — never in
the tree. @impl/done

##ADDING-A-HOST-IS-ONE-TARGET-BLOCK Adding a host is one `[[target]]` block. @impl/done

##A-SELF-PULL-TARGET-IS-ONLY-VERIFIED A `self-pull` target
(a host that mirrors *itself* from elsewhere) is listed with
`mode = "self-pull"`: the tool does not push to it, only verifies it is
level with mainline. @impl/done

## The fan-out procedure {#procedure}

##FAN-OUT-PUSHES-MAINLINE-TO-EVERY-PUSH-TARGET Fan-out pushes mainline to every `push` target. @impl/done

##the-shape-is-always-the-same-four-steps The shape is always the
same four steps, per target: @impl/done

1. ##STEP-FETCH **Fetch** the target's `main` (by URL, read-only). @impl/done
2. ##STEP-VERIFY-ANCESTRY **Verify ancestry** — the target's `main` must be an *ancestor* of
   local mainline. If it is not, the target has diverged: abort *that*
   target, loud (§fail-loud). Do not touch it. @impl/done
3. ##STEP-PUSH-FAST-FORWARD-ONLY **Push fast-forward-only** — `main` and tags, never `--force`. @impl/done
4. ##STEP-REPORT **Report** the target as `ok`, `sync` (already level), or a named
   `DRIFT`. @impl/done

##THE-PUSH-IS-BY-URL-FROM-THE-MANIFEST The push is **by URL**, from the manifest, so the manifest is the one
source of truth for the target set — not a pile of git remotes that can
drift out of step with it. @impl/done

##THE-FAN-OUT-IS-THE-ONLY-WAY-HISTORY-REACHES-A-HOST This is the *only* way history reaches a host:
not `git push host-a`, not a click in a web UI — the fan-out. @impl/done

## Fail-loud semantics {#fail-loud}

##A-NON-FAST-FORWARD-MEANS-THE-HOST-CARRIES-A-MAIN-YOU-LACK A non-fast-forward on a target means that host carries a `main` your
mainline does not — almost always a direct write or a force-push to
that host. @impl/done

##the-fan-outs-response-is-fixed The fan-out's response is fixed: @impl/done

- ##RESPONSE-ABORT-THAT-TARGET **Abort that target**, with a message naming the divergence (the
  host, and the commits it has that mainline lacks). *Half built: the host is
  named, the commits are not — by anything. This document's own reference
  script below aborts the target and prints `"$name: DRIFT — host has commits
  mainline lacks"`, which states that a divergence exists without enumerating
  one commit of it; and the only port in the perimeter, `xtask/src/mirror.rs`,
  names the failed `<target>:<ref>` pairs and relays git's rejection text
  (`:296-303`, `:313-320`) while performing no `ls-remote` and no `merge-base`
  on the push path, so it never learns the target's tip and cannot compute the
  range. Searched for the commit-range computation, not the wording:
  `merge-base` · `is-ancestor` · `ls-remote` · `rev-list` over the standing
  perimeter — the only `ls-remote` in the fan-out is `remote_main`
  (`mirror.rs:157`), reached solely from `probe` and the `self-pull` arm, and
  `rev-list` appears nowhere.* @spec/done
- ##RESPONSE-NEVER-FORCE **Never `--force`.** The tool has no force path to reach for. @impl/done
- ##RESPONSE-DO-NOT-BLOCK-THE-OTHER-TARGETS **Do not block the other targets.** A divergence on host B does not
  stop host A from receiving its legitimate fast-forward. @impl/done

> ##A-DIVERGED-TARGET-IS-A-SIGNAL-TO-INVESTIGATE A diverged target is a signal to investigate, never something to
> silently clobber. @impl/done

##RECONCILIATION-IS-DELIBERATE-AND-MANUAL Reconciliation is deliberate and manual: fetch the host, inspect the
divergent commits, merge what is wanted **into mainline**, then re-fan
([`daily-loop.md` §drift](daily-loop.md#drift)). @impl/done

## The read-only drift check {#drift-check}

##a-check-mode-answers-is-everyone-level A `--check` mode answers "is everyone level?" without pushing anything: @impl/done

```sh
project-mirror --check     # read-only; non-zero exit on drift
```

##CHECK-FETCHES-EACH-TARGET-AND-COMPARES-TO-MAINLINE It fetches each target and compares to mainline. @impl/done

##SYNC-MEANS-LEVEL-DRIFT-NAMES-A-HOST-THAT-MOVED `sync` everywhere means
all hosts equal mainline; a `DRIFT` line names a host that has moved. @impl/done

##CHECK-WRITES-NOTHING-AND-IS-SAFE-AS-A-PRE-FLIGHT It writes nothing — safe as a pre-flight before a fan-out or inside a sweep. @impl/done

## Bringing an accepted web merge home {#bring-home}

##A-WEB-MERGE-PUTS-THAT-HOSTS-MAIN-AHEAD When a PR is merged through a host's web UI, *that host's* `main` is now
ahead of mainline. @impl/done

##IT-IS-NOT-INTEGRATED-UNTIL-IT-IS-BROUGHT-HOME It is **not integrated** until it is brought home. @impl/done

##do-that-first-then-fan-out Do that first, then fan out: @impl/done

```sh
git fetch <host-a-url> main
git merge --ff-only FETCH_HEAD   # fast-forward local mainline
project-mirror                   # now fan out to everyone else
```

##THE-FF-ONLY-IS-LOAD-BEARING The `--ff-only` is load-bearing: if local mainline cannot fast-forward
to the host's `main`, your tree has commits the host lacks, and the
merge must be reconciled by hand before fan-out. @impl/done

##THE-HOST-YOU-PULLED-FROM-BECOMES-A-NO-OP The host you pulled
from becomes a no-op on the next fan-out; every other host catches up. @impl/done

## Deletions and history rewrites do not propagate {#no-propagate}

##two-things-the-fan-out-will-not-carry Two things the fan-out deliberately will **not** carry: @impl/done

| Action on one host | Propagates? | Why |
|--------------------|-------------|-----|
| ##ROW-NO-PROPAGATE-DELETE-A-BRANCH Delete a branch @spec/done | **No** @spec/done | An accidental deletion must not cascade to every host @spec/done |
| ##ROW-NO-PROPAGATE-REWRITE-HISTORY Rewrite / force-push history @spec/done | **No** @spec/done | The fan-out is fast-forward-only; a rewrite is a divergence, and divergences fail loud @spec/done |

##DELETING-A-BRANCH-EVERYWHERE-IS-A-PER-HOST-ACT Deleting a branch everywhere is a deliberate, per-host act. @impl/done

##THE-FAN-OUT-ONLY-EVER-ADVANCES-REFS The fan-out
only ever *advances* refs, so no single mistake can subtract history
from the whole fleet at once. @impl/done

## Reference implementation {#reference}

##A-FAN-OUT-IS-ABOUT-FIFTEEN-LINES-OF-SH A fan-out is about fifteen lines of `sh`. @impl/done

##THERE-IS-NO-FORCE-IN-IT-BY-LAW There is **no `--force` in it,
by law** — the absence is the invariant, not an oversight: @impl/done

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

##ADAPT-READ-TARGETS-TO-THE-PROJECTS-MANIFEST-FORMAT Adapt `read_targets` to the manifest format the project uses. @impl/done

##the-two-invariants-to-preserve-when-you-port-it The two
invariants to preserve when you port it: @impl/done

- ##INVARIANT-THE-ANCESTRY-GATE the **ancestry gate** before
  every push, @impl/done
- ##INVARIANT-THE-ABSENCE-OF-ANY-FORCE-PATH and the **absence of any force path**. @impl/done

## Pin the invariant with a test {#never-force-test}

##A-RULE-WITH-NO-CHECKER-IS-A-WISH A rule with no checker is a wish. @impl/done

##build-the-push-command-in-one-place-and-assert Build the push command in one place
and assert, in a test or a CI step, that it can never emit a force: @impl/done

```sh
# CI guard: no force flag may appear in the fan-out script.
if grep -nE -- '--force|[[:space:]]-f([[:space:]]|$)|push[^|]*\+' fanout.sh; then
    echo "FAIL: a force path exists in the fan-out — remove it" >&2
    exit 1
fi
```

##BETTER-STILL-UNIT-TEST-A-PURE-PUSH-ARGUMENT-FUNCTION Better still, in a project with a real test suite, factor the push
arguments into one pure function and unit-test that its output never
contains `--force`, `-f`, or a `+`-prefixed refspec for any ref shape. @impl/done

##THE-INVARIANT-IS-RUNNABLE-CAPITAL The invariant is then **runnable capital, not a prose promise** — a rule
you cannot run is a rule you cannot trust. @impl/done

## Summary {#summary}

- ##SUM-THE-TARGET-SET-IS-A-COMMITTED-CREDENTIAL-FREE-MANIFEST The target set is a committed, credential-free manifest — shared,
  CI-visible, reviewed like code. A local git remote cannot serve it. @impl/done
- ##SUM-FAN-OUT-PER-TARGET-IS-FETCH-VERIFY-PUSH-REPORT Fan-out per target: fetch, verify the target is an ancestor of
  mainline, push fast-forward-only by URL, report. @impl/done
- ##SUM-A-NON-FAST-FORWARD-ABORTS-THAT-TARGET-LOUD A non-fast-forward aborts *that* target loud, names the divergence,
  and never forces. Reconcile into mainline by hand, then re-fan. *Three of the
  four are built and the fourth is half built. Loud per-target abort with the
  other targets still served, and a non-zero exit: `xtask/src/mirror.rs:296-320`.
  Never forces, and hardened past this document's ask into a unit test —
  `push_args` is a pure function and `push_args_never_force`
  (`mirror.rs:426-440`) asserts no `--force`, `-f` or `+`-refspec for four ref
  shapes, which `spec/common/PROP-016-source-mirrors.md:64` calls «runnable
  capital, not prose». «Reconcile by hand, then re-fan» is the failure message
  itself (`mirror.rs:315-319`). What is only half built is «names the
  divergence» in the sense `##RESPONSE-ABORT-THAT-TARGET` defines it: the
  diverged target is named, the commits it carries are not, by any
  implementation including this document's own reference script.* @spec/done
- ##SUM-CHECK-PROBES-DRIFT-AND-A-WEB-MERGE-COMES-HOME-FIRST A `--check` mode probes drift read-only; bring a web merge home with
  `merge --ff-only` before fanning out. @impl/done
- ##SUM-DELETIONS-AND-REWRITES-DO-NOT-PROPAGATE Deletions and history rewrites do not propagate — a safety choice. @impl/done
- ##SUM-THE-REFERENCE-FAN-OUT-HAS-NO-FORCE-PATH The reference fan-out is ~15 lines with no force path; pin that
  absence with a test or a CI grep. @impl/done
