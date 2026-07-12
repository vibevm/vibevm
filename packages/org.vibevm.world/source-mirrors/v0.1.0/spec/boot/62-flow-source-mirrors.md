# Flow: Source Mirrors {#root}

This project's source is **multi-homed**: the same history lives on
more than one git host. It is kept in step under a **single-writer**
model, so the copies never diverge.

## Core rule {#core-rule}

There is one **mainline** — the maintainer's integrated local `main`.
No host is primary. Every host in the target manifest is a downstream
**read-replica** of mainline. History reaches a host only through the
project's **fan-out** procedure, which is fast-forward-only and never
uses `--force`.

Full protocol:
[`spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md`](../flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md).
Fan-out mechanics and the reference script:
[`spec/flows/source-mirrors/fanout-mechanics.md`](../flows/source-mirrors/fanout-mechanics.md).
The maintainer's day:
[`spec/flows/source-mirrors/daily-loop.md`](../flows/source-mirrors/daily-loop.md).

## In session {#in-session}

- Commit on mainline as usual. Rollout to the hosts is a **separate,
  deliberate** step — the fan-out procedure, run at a natural
  checkpoint, not a daemon and not `git push` to each host.
- A web-UI merge on a host (a clicked "Merge" button) is **not**
  integrated until it has been brought home into mainline first; only
  then does it fan out to the other hosts.
- If a host reports **drift** (it carries a `main` mainline does not),
  treat it as a signal to investigate — fetch, inspect, reconcile
  *into* mainline, then re-fan. Never overwrite the host to make the
  warning go away.

## Never {#never}

- Never push directly to a replica host — rollout goes through the
  fan-out procedure, which is the single source of truth for targets.
- Never `--force` any target, for any ref, for any reason. The
  fan-out is fast-forward-only by law.
- Never resolve a divergence by clobbering the target. A diverged
  target is investigated and reconciled into mainline, never silently
  overwritten.
- Never treat a web-UI merge as integrated until its commits are in
  mainline.
