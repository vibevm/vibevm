# Flow: Source Mirrors {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-SOURCE-IS-MULTI-HOMED This project's source is **multi-homed**: the same history lives on
more than one git host. @status:impl/done

@fact:COPIES-ARE-KEPT-IN-STEP-UNDER-A-SINGLE-WRITER-MODEL It is kept in step under a **single-writer**
model, so the copies never diverge. @status:impl/done

## Core rule {#core-rule}

@fact:THERE-IS-ONE-MAINLINE There is one **mainline** — the maintainer's integrated local `main`. @status:impl/done

@fact:NO-HOST-IS-PRIMARY No host is primary. @status:impl/done

@fact:EVERY-HOST-IS-A-DOWNSTREAM-READ-REPLICA Every host in the target manifest is a downstream
**read-replica** of mainline. @status:impl/done

@fact:HISTORY-REACHES-A-HOST-ONLY-THROUGH-THE-FAN-OUT History reaches a host only through the
project's **fan-out** procedure, which is fast-forward-only and never
uses `--force`. @status:impl/done

@fact:full-protocol-pointer Full protocol:
@spec://org.vibevm.world/source-mirrors/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL#root. @status:impl/done

@fact:fanout-mechanics-pointer Fan-out mechanics and the reference script:
@spec://org.vibevm.world/source-mirrors/flows/source-mirrors/fanout-mechanics#root. @status:impl/done

@fact:maintainers-day-pointer The maintainer's day:
@spec://org.vibevm.world/source-mirrors/flows/source-mirrors/daily-loop#root. @status:impl/done

## In session {#in-session}

- @fact:COMMIT-ON-MAINLINE-ROLLOUT-IS-A-SEPARATE-STEP Commit on mainline as usual. Rollout to the hosts is a **separate,
  deliberate** step — the fan-out procedure, run at a natural
  checkpoint, not a daemon and not `git push` to each host. @status:impl/done
- @fact:A-WEB-UI-MERGE-IS-NOT-INTEGRATED-UNTIL-BROUGHT-HOME A web-UI merge on a host (a clicked "Merge" button) is **not**
  integrated until it has been brought home into mainline first; only
  then does it fan out to the other hosts. @status:impl/done
- @fact:REPORTED-DRIFT-IS-A-SIGNAL-TO-INVESTIGATE If a host reports **drift** (it carries a `main` mainline does not),
  treat it as a signal to investigate — fetch, inspect, reconcile
  *into* mainline, then re-fan. Never overwrite the host to make the
  warning go away. @status:impl/done

## Never {#never}

- @fact:NEVER-PUSH-DIRECTLY-TO-A-REPLICA-HOST Never push directly to a replica host — rollout goes through the
  fan-out procedure, which is the single source of truth for targets. @status:impl/done
- @fact:NEVER-FORCE-ANY-TARGET Never `--force` any target, for any ref, for any reason. The
  fan-out is fast-forward-only by law. @status:impl/done
- @fact:NEVER-RESOLVE-A-DIVERGENCE-BY-CLOBBERING Never resolve a divergence by clobbering the target. A diverged
  target is investigated and reconciled into mainline, never silently
  overwritten. @status:impl/done
- @fact:NEVER-TREAT-A-WEB-UI-MERGE-AS-INTEGRATED Never treat a web-UI merge as integrated until its commits are in
  mainline. @status:impl/done
