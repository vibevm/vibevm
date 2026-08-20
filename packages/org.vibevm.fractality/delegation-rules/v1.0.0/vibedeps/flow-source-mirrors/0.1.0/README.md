# `flow:source-mirrors` — one mainline, hosts as replicas {#root}

A `flow` package that installs the **single-writer source mirrors**
discipline into a project. When the same source must live on more than
one git host, the naive fix — let each host accept writes and mirror to
the others — is multi-master replication, and two writes to the same
branch diverge. This flow dissolves that failure mode instead of
managing it: there is one **mainline** (the maintainer's local `main`,
no host primary), every host is a downstream **read-replica**, and
history reaches a host only through a **fast-forward-only fan-out that
fails loud** on any divergence and never `--force`s.

The cost is paid once, in the model, not continuously, in conflict
resolution.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md` — the problem
  (multi-homing without multi-master), the model (single-writer
  mainline; hosts as replicas; contributions in via any inbox), what it
  buys, what it costs, and a re-derive prompt.
- `spec/flows/source-mirrors/fanout-mechanics.md` — the committed
  credential-free manifest, the fetch/verify/push/report procedure,
  fail-loud semantics, the read-only drift check, bringing a web merge
  home, and a ~15-line reference script with the never-`--force`
  invariant pinned by a test.
- `spec/flows/source-mirrors/daily-loop.md` — the maintainer's day,
  handling reported drift, onboarding a host, offboarding a host.
- `spec/boot/62-flow-source-mirrors.md` — boot snippet: the core rule
  and the never-do list.

## Install {#install}

```bash
vibe install flow:source-mirrors
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:source-mirrors
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Audience {#audience}

This flow is **maintainer-facing**: the fan-out, the manifest, and drift
reconciliation are the integrator's job. A day-to-day contributor needs
only the two nevers — **never push directly to a replica host**, and
**never `--force` anything** — plus the knowledge that their PR is an
inbox item the maintainer integrates, not a direct write.

## Composition {#composition}

- `flow:atomic-commits` — pushed history is frozen; a fast-forward-only
  fan-out is that rule's multi-host corollary (a replica only ever
  advances, never rewrites).
- `flow:wal` — the fan-out is a natural session wind-down step; the WAL
  entry notes "fanned out at <checkpoint>".
- `flow:decision-records` — the host set and the single-writer choice
  are recorded decisions, each with a revisit trigger (e.g. "revisit
  when parallel full-time integrators exceed one").

## Philosophical background {#background}

The practice is crystallized from the origin project's source-mirrors
law — a hub-and-spoke, benevolent-dictator model in the Linux-kernel
tradition (patches arrive as inboxes, the maintainer's tree is the merge
authority, a hub mirrors out). The collection's spirit is the book
*AI-native development*, which ships in Russian inside `flow:redbook` at
`spec/book/ru/`. Short version: make divergence impossible by
construction, and prove the invariant with a test rather than a promise.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
