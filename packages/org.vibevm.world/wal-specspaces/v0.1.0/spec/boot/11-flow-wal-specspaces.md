# Flow: WAL Specspaces {#root}

<status stage="impl" state="done"/>

##THE-PROJECT-MAY-HOST-SPECSPACES This project may host **specspaces** — sub-projects nested in the
repository that are worked on as independent projects, each with its
own boot contract, WAL, and cold-resume file. @impl/done

##THE-REGISTRY-IS-SPECSPACES-MD-AT-THE-HOST-ROOT The registry is
`SPECSPACES.md` at the host root; if that file is absent, no
specspaces exist and this snippet is inert. @impl/done

## Recognising a specspace session {#recognise}

1. ##RECOGNISE-A-PHRASE-CARRYING-A-SPECSPACE-NAME A resume or wind-down phrase carrying a specspace name —
   `RESUME SESSION <name>`, `END SESSION <name>`, or the project's
   language twins — targets that specspace, not the host project. @impl/done
2. ##RECOGNISE-A-TASK-INSIDE-A-REGISTERED-SPECSPACE-ROOT A session whose task clearly lives inside a registered specspace
   root follows that specspace's boot contract, even without the
   phrase. When in doubt, ask which project the session is for. @impl/done

## Which project a bare phrase targets {#default}

##A-PHRASE-RESOLVES-TO-EXACTLY-ONE-TARGET A session phrase resolves to exactly **one** target — the host
project or a single specspace — by this order: @impl/done

1. ##TARGET-EXPLICIT-WINS-ALWAYS **Explicit target wins, always.** A phrase that names a specspace
   (`RESUME SESSION <name>`), or that names an explicit directory,
   targets that specspace or directory — regardless of any declared
   default. This is how the user forces restoration from an arbitrary
   specspace or directory. A name matching no registry row is
   surfaced, not guessed. @impl/done
2. ##TARGET-DECLARED-DEFAULT **Declared default.** A **bare** phrase (no name) uses the
   specspace named by the `default:` line of `SPECSPACES.md`, if one
   is declared. @impl/done
3. ##TARGET-HOST-FALLBACK **Host fallback.** With no name and no declared default, a bare
   phrase targets the **host project** — restore the host root's own
   WAL and cold-resume file, per the host contract's session-command
   sections. @impl/done

##A-BARE-PHRASE-NEVER-SILENTLY-SELECTS-A-SPECSPACE A bare phrase therefore **never silently selects a specspace**. @impl/done

##AT-THE-HOST-ROOT-A-BARE-PHRASE-RESTORES-THE-HOST-WAL At the
host root, a bare `восстанови сессию` / `RESUME SESSION` restores the
**host** WAL — not a registered specspace such as `fractality`. @impl/done

##TARGETING-A-SPECSPACE-REQUIRES-NAMING-IT Targeting a specspace requires naming it (or declaring it the default). @impl/done

## The boot-scoping law {#scoping}

##A-SPECSPACE-SESSION-READS-IN-ORDER A specspace session reads, in order: @impl/done

1. ##READS-THE-HOSTS-REPO-WIDE-RULES the host's repo-wide non-negotiable rules (the section the host
   contract marks as binding for every commit), @impl/done
2. ##READS-THE-SPECSPACES-OWN-BOOT-CONTRACT the specspace's own boot contract (`CLAUDE.md` at the specspace
   root, or the equivalent boot contract living there), @impl/done
3. ##READS-THE-SPECSPACE-WAL the specspace WAL, @impl/done
4. ##READS-THE-SPECSPACE-COLD-RESUME-FILE the specspace cold-resume file (the WAL wins where they diverge), @impl/done
5. ##READS-ANY-ACTIVE-PLAN-THE-WAL-NAMES any active plan the specspace WAL names. @impl/done

##IT-DOES-NOT-LOAD-THE-HOSTS-FULL-BOOT-OR-SPECS It does **not** load the host's full boot sequence, the host WAL, or
host specs — unless the task explicitly crosses into the host
project, and then the session says so before touching host files. @impl/done

## Session commands, scoped {#commands}

##SCOPED-PHRASES-OPERATE-ON-THE-SPECSPACES-OWN-FILES Wind-down and resume phrases carrying a specspace name operate on
that specspace's WAL and cold-resume file, and refresh the
specspace's one-line status in `SPECSPACES.md`. @impl/done

##THE-HOST-WAL-IS-UPDATED-ONLY-WHEN-HOST-FILES-CHANGED The host WAL is
updated only when host files actually changed in the session. @impl/done

##RESUME-REMAINS-REPORT-THEN-WAIT Resume remains report-then-wait: restore, verify state empirically,
report, stop. @impl/done

##sibling-document-pointers Full protocol:
@spec://org.vibevm.world/wal-specspaces/flows/wal-specspaces/SPECSPACES-PROTOCOL#root. @impl/done
