# `flow:source-mirrors` — one mainline, hosts as replicas {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-THE-SINGLE-WRITER-SOURCE-MIRRORS-DISCIPLINE A `flow` package that installs the **single-writer source mirrors**
discipline into a project. @impl/done

##the-naive-fix-is-multi-master-replication When the same source must live on more than
one git host, the naive fix — let each host accept writes and mirror to
the others — is multi-master replication, and two writes to the same
branch diverge. @spec/done

##this-flow-dissolves-the-failure-mode This flow dissolves that failure mode instead of
managing it: @impl/done

- ##MODEL-ONE-MAINLINE-NO-HOST-PRIMARY there is one **mainline** (the maintainer's local `main`,
  no host primary), @impl/done
- ##MODEL-EVERY-HOST-IS-A-DOWNSTREAM-READ-REPLICA every host is a downstream **read-replica**, @impl/done
- ##MODEL-FAN-OUT-IS-FF-ONLY-AND-FAILS-LOUD and
  history reaches a host only through a **fast-forward-only fan-out that
  fails loud** on any divergence and never `--force`s. @impl/done

##the-cost-is-paid-once-in-the-model The cost is paid once, in the model, not continuously, in conflict
resolution. @spec/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-PROTOCOL `spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md` — the problem
  (multi-homing without multi-master), the model (single-writer
  mainline; hosts as replicas; contributions in via any inbox), what it
  buys, what it costs, and a re-derive prompt. @impl/done
- ##CONTENT-THE-FANOUT-MECHANICS `spec/flows/source-mirrors/fanout-mechanics.md` — the committed
  credential-free manifest, the fetch/verify/push/report procedure,
  fail-loud semantics, the read-only drift check, bringing a web merge
  home, and a ~15-line reference script with the never-`--force`
  invariant pinned by a test. @impl/done
- ##CONTENT-THE-DAILY-LOOP `spec/flows/source-mirrors/daily-loop.md` — the maintainer's day,
  handling reported drift, onboarding a host, offboarding a host. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/62-flow-source-mirrors.md` — boot snippet: the core rule
  and the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:source-mirrors
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:source-mirrors
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Audience {#audience}

##THIS-FLOW-IS-MAINTAINER-FACING This flow is **maintainer-facing**: the fan-out, the manifest, and drift
reconciliation are the integrator's job. @impl/done

##A-CONTRIBUTOR-NEEDS-ONLY-THE-TWO-NEVERS A day-to-day contributor needs
only the two nevers — **never push directly to a replica host**, and
**never `--force` anything** — plus the knowledge that their PR is an
inbox item the maintainer integrates, not a direct write. @impl/done

## Composition {#composition}

- ##COMPOSES-ATOMIC-COMMITS `flow:atomic-commits` — pushed history is frozen; a fast-forward-only
  fan-out is that rule's multi-host corollary (a replica only ever
  advances, never rewrites). @impl/done
- ##COMPOSES-WAL `flow:wal` — the fan-out is a natural session wind-down step; the WAL
  entry notes "fanned out at <checkpoint>". @spec/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — the host set and the single-writer choice
  are recorded decisions, each with a revisit trigger (e.g. "revisit
  when parallel full-time integrators exceed one"). @impl/done

## Philosophical background {#background}

##practice-crystallized-from-the-origin-projects-law The practice is crystallized from the origin project's source-mirrors
law — a hub-and-spoke, benevolent-dictator model in the Linux-kernel
tradition (patches arrive as inboxes, the maintainer's tree is the merge
authority, a hub mirrors out). @spec/done

##collections-spirit-is-the-redbook The collection's spirit is the book
*AI-native development*, which ships in Russian inside `flow:redbook` at
`spec/book/ru/`. @spec/done

##MAKE-DIVERGENCE-IMPOSSIBLE-AND-PROVE-THE-INVARIANT Short version: make divergence impossible by
construction, and prove the invariant with a test rather than a promise. @spec/done

## License {#license}

##license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @impl/done
