# Flow: Source Mirrors {#root}

<status stage="impl" state="done"/>

##PROJECT-SOURCE-IS-MULTI-HOMED This project's source is **multi-homed**: the same history lives on
more than one git host. @impl/done

##COPIES-ARE-KEPT-IN-STEP-UNDER-A-SINGLE-WRITER-MODEL It is kept in step under a **single-writer**
model, so the copies never diverge. @impl/done

## Core rule {#core-rule}

##THERE-IS-ONE-MAINLINE There is one **mainline** — the maintainer's integrated local `main`. @impl/done

##NO-HOST-IS-PRIMARY No host is primary. @impl/done

##EVERY-HOST-IS-A-DOWNSTREAM-READ-REPLICA Every host in the target manifest is a downstream
**read-replica** of mainline. @impl/done

##HISTORY-REACHES-A-HOST-ONLY-THROUGH-THE-FAN-OUT History reaches a host only through the
project's **fan-out** procedure, which is fast-forward-only and never
uses `--force`. @impl/done

##full-protocol-pointer Full protocol:
[`spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md`](../flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md). @impl/done

##fanout-mechanics-pointer Fan-out mechanics and the reference script:
[`spec/flows/source-mirrors/fanout-mechanics.md`](../flows/source-mirrors/fanout-mechanics.md). @impl/done

##maintainers-day-pointer The maintainer's day:
[`spec/flows/source-mirrors/daily-loop.md`](../flows/source-mirrors/daily-loop.md). @impl/done

## In session {#in-session}

- ##COMMIT-ON-MAINLINE-ROLLOUT-IS-A-SEPARATE-STEP Commit on mainline as usual. Rollout to the hosts is a **separate,
  deliberate** step — the fan-out procedure, run at a natural
  checkpoint, not a daemon and not `git push` to each host. @impl/done
- ##A-WEB-UI-MERGE-IS-NOT-INTEGRATED-UNTIL-BROUGHT-HOME A web-UI merge on a host (a clicked "Merge" button) is **not**
  integrated until it has been brought home into mainline first; only
  then does it fan out to the other hosts. @impl/done
- ##REPORTED-DRIFT-IS-A-SIGNAL-TO-INVESTIGATE If a host reports **drift** (it carries a `main` mainline does not),
  treat it as a signal to investigate — fetch, inspect, reconcile
  *into* mainline, then re-fan. Never overwrite the host to make the
  warning go away. @impl/done

## Never {#never}

- ##NEVER-PUSH-DIRECTLY-TO-A-REPLICA-HOST Never push directly to a replica host — rollout goes through the
  fan-out procedure, which is the single source of truth for targets. @impl/done
- ##NEVER-FORCE-ANY-TARGET Never `--force` any target, for any ref, for any reason. The
  fan-out is fast-forward-only by law. @impl/done
- ##NEVER-RESOLVE-A-DIVERGENCE-BY-CLOBBERING Never resolve a divergence by clobbering the target. A diverged
  target is investigated and reconciled into mainline, never silently
  overwritten. @impl/done
- ##NEVER-TREAT-A-WEB-UI-MERGE-AS-INTEGRATED Never treat a web-UI merge as integrated until its commits are in
  mainline. @impl/done
