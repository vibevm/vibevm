# `flow:source-mirrors` — one mainline, hosts as replicas {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-SINGLE-WRITER-SOURCE-MIRRORS-DISCIPLINE A `flow` package that installs the **single-writer source mirrors**
discipline into a project. @status:impl/done

@fact:the-naive-fix-is-multi-master-replication When the same source must live on more than
one git host, the naive fix — let each host accept writes and mirror to
the others — is multi-master replication, and two writes to the same
branch diverge. @status:spec/done

@fact:this-flow-dissolves-the-failure-mode This flow dissolves that failure mode instead of
managing it: @status:impl/done

- @fact:MODEL-ONE-MAINLINE-NO-HOST-PRIMARY there is one **mainline** (the maintainer's local `main`,
  no host primary), @status:impl/done
- @fact:MODEL-EVERY-HOST-IS-A-DOWNSTREAM-READ-REPLICA every host is a downstream **read-replica**, @status:impl/done
- @fact:MODEL-FAN-OUT-IS-FF-ONLY-AND-FAILS-LOUD and
  history reaches a host only through a **fast-forward-only fan-out that
  fails loud** on any divergence and never `--force`s. @status:impl/done

@fact:the-cost-is-paid-once-in-the-model The cost is paid once, in the model, not continuously, in conflict
resolution. @status:spec/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md` — the problem
  (multi-homing without multi-master), the model (single-writer
  mainline; hosts as replicas; contributions in via any inbox), what it
  buys, what it costs, and a re-derive prompt. @status:impl/done
- @fact:CONTENT-THE-FANOUT-MECHANICS `spec/flows/source-mirrors/fanout-mechanics.md` — the committed
  credential-free manifest, the fetch/verify/push/report procedure,
  fail-loud semantics, the read-only drift check, bringing a web merge
  home, and a ~15-line reference script with the never-`--force`
  invariant pinned by a test. @status:impl/done
- @fact:CONTENT-THE-DAILY-LOOP `spec/flows/source-mirrors/daily-loop.md` — the maintainer's day,
  handling reported drift, onboarding a host, offboarding a host. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/62-flow-source-mirrors.md` — boot snippet: the core rule
  and the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:source-mirrors
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:source-mirrors
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Audience {#audience}

@fact:THIS-FLOW-IS-MAINTAINER-FACING This flow is **maintainer-facing**: the fan-out, the manifest, and drift
reconciliation are the integrator's job. @status:impl/done

@fact:A-CONTRIBUTOR-NEEDS-ONLY-THE-TWO-NEVERS A day-to-day contributor needs
only the two nevers — **never push directly to a replica host**, and
**never `--force` anything** — plus the knowledge that their PR is an
inbox item the maintainer integrates, not a direct write. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — pushed history is frozen; a fast-forward-only
  fan-out is that rule's multi-host corollary (a replica only ever
  advances, never rewrites). @status:impl/done
- @fact:COMPOSES-WAL `flow:wal` — the fan-out is a natural session wind-down step; the WAL
  entry notes "fanned out at <checkpoint>". @status:spec/done
- @fact:COMPOSES-DECISION-RECORDS `flow:decision-records` — the host set and the single-writer choice
  are recorded decisions, each with a revisit trigger (e.g. "revisit
  when parallel full-time integrators exceed one"). @status:impl/done

## Philosophical background {#background}

@fact:practice-crystallized-from-the-origin-projects-law The practice is crystallized from the origin project's source-mirrors
law — a hub-and-spoke, benevolent-dictator model in the Linux-kernel
tradition (patches arrive as inboxes, the maintainer's tree is the merge
authority, a hub mirrors out). @status:spec/done

@fact:collections-spirit-is-the-redbook The collection's spirit is the book
*AI-native development*, which ships in Russian inside `flow:redbook` at
`spec/book/ru/`. @status:spec/done

@fact:MAKE-DIVERGENCE-IMPOSSIBLE-AND-PROVE-THE-INVARIANT Short version: make divergence impossible by
construction, and prove the invariant with a test rather than a promise. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @status:impl/done
