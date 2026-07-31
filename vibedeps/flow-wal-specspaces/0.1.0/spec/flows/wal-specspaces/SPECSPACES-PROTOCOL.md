# The Specspaces Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *what* a specspace is,
the registry file that names them, how a session phrase resolves to a
target, the scoped session grammar, and the five laws that keep a
specspace's state from bleeding into its host. @impl/done

##WAL-MECHANICS-ARE-NOT-REDEFINED-HERE The per-specspace WAL
mechanics are not redefined here — each specspace runs the two-file
model owned by `flow:org.vibevm.world/wal`. @impl/done

## What a specspace is {#what}

##A-SPECSPACE-IS-WORKED-ON-AS-AN-INDEPENDENT-PROJECT A specspace is a sub-project that lives inside a host repository but
is **worked on as an independent project**: its own boot contract,
its own WAL, its own cold-resume file, its own plans and specs, its
own definition of a green floor. @impl/done

##THE-HOST-CARRIES-IT-BUT-A-SPECSPACE-SESSION-IGNORES-HOST-CONTEXT The host carries it (one git
history, one set of repo-wide rules), but a session working on the
specspace neither loads nor scans the host's project context. @impl/done

##A-SPECSPACE-IS-ITS-OWN-INSTALLABLE-PUBLISHABLE-THING A specspace is its own installable, publishable thing in the general
case; that a host repository happens to author one under `packages/`
(as vibevm does with `fractality`) is incidental. @impl/done

##PEOPLE-INSTALL-SPECSPACES-AS-ORDINARY-PACKAGES People install
specspaces as ordinary packages, and two of them coinciding inside
one host means nothing in general. @impl/done

##the-problem-this-solves The problem this solves: a central WAL describes one project. @spec/done

##sessions-otherwise-face-a-bad-choice The
moment a repository hosts a second, independently-evolving effort,
sessions face a bad choice — boot the whole host corpus to work on a
corner of it, or work blind. @spec/done

##SPECSPACES-MAKE-THE-SECOND-EFFORT-A-FIRST-CLASS-PROJECT Specspaces make the second effort a
first-class project with first-class session continuity, at the cost
of one registry file and one grammar extension. @impl/done

## The registry: `SPECSPACES.md` {#registry}

##ONE-FILE-AT-THE-HOST-ROOT-NAMES-EVERY-SPECSPACE One file at the host root names every specspace. @impl/done

##AN-OPTIONAL-DEFAULT-LINE-THEN-ONE-ROW-PER-SPECSPACE An optional
`default:` line may precede the table; then one table row per
specspace: @impl/done

```markdown
default: host

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-09 — ignition PLANNED; next: Phase 0 |
```

- ##FIELD-DEFAULT **`default:`** (optional, above the table) — which target a **bare**
  session phrase resolves to (see [target resolution](#resolve)).
  `default: host` (or omitting the line entirely) makes a bare phrase
  target the host project; `default: <name>` makes a bare phrase
  target that specspace. Omit it unless the repository's primary work
  really lives in one specspace. @impl/done
- ##FIELD-NAME **name** — the word used in session phrases. Short, unique,
  lowercase. @impl/done
- ##FIELD-ROOT **root** — the specspace root, relative to the host root. The
  specspace's `CLAUDE.md` (or equivalent boot contract) lives here. @impl/done
- ##FIELD-WAL-AND-CONTINUE **wal**, **continue** — paths relative to root. Defaults are
  `spec/WAL.md` and `CONTINUE.md` per the wal flow; a specspace
  without a spec tree at its root may keep both flat at the root, as
  the example does. The registry entry is the truth. @impl/done
- ##FIELD-STATUS **status** — one line, refreshed at every specspace wind-down:
  date, campaign/phase state, the next step. A pointer for the
  *host's* readers; never the specspace's canonical state. @impl/done

## Target resolution {#resolve}

##A-PHRASE-RESOLVES-TO-EXACTLY-ONE-TARGET A resume or wind-down phrase resolves to exactly **one** target — the
host project or a single specspace — by this order: @impl/done

1. ##TARGET-EXPLICIT-WINS-ALWAYS **Explicit target wins, always.** A phrase that names a specspace
   (`RESUME SESSION <name>`), or that names an explicit directory,
   targets that specspace or directory — **regardless of any declared
   default or prior context**. This is the user's always-available
   escape hatch: an explicit command forces restoration from an
   arbitrary specspace or directory. A name matching no registry row
   is surfaced, not guessed. @impl/done
2. ##TARGET-DECLARED-DEFAULT **Declared default.** A **bare** phrase (no name) uses the
   specspace named by the `default:` line of `SPECSPACES.md`, when one
   is declared. @impl/done
3. ##TARGET-HOST-FALLBACK **Host fallback.** With no name and no declared default, a bare
   phrase targets the **host project** — restore the host root's own
   WAL and cold-resume file, per the host contract's session-command
   sections. @impl/done

##A-BARE-PHRASE-NEVER-SILENTLY-SELECTS-A-SPECSPACE A bare phrase therefore **never silently selects a specspace**. @impl/done

##AT-THE-HOST-ROOT-A-BARE-PHRASE-RESTORES-THE-HOST-WAL At the
host root, a bare `RESUME SESSION` / `ВОССТАНОВИ СЕССИЮ` restores the
**host** WAL — not a registered specspace such as `fractality`. @impl/done

##the-whole-point-of-rule-three This
is the whole point of rule 3: a registry with rows in it must not
tempt a session into resuming the wrong project. @spec/done

## The session grammar {#grammar}

##PHRASES-GAIN-AN-OPTIONAL-SPECSPACE-NAME The wal flow's wind-down and resume phrases gain an optional
specspace name. @impl/done

##recognise-the-intent-not-the-exact-wording-lead Recognise the intent, not the exact wording: @impl/done

| Intent | English | Russian |
|---|---|---|
| ##ROW-RESUME-INTO-SPECSPACE resume into specspace @impl/done | `RESUME SESSION <name>`, `RESTORE CONTEXT <name>` @impl/done | `ВОССТАНОВИ СЕССИЮ <name>`, `ПРОДОЛЖАЕМ <name>` @impl/done |
| ##ROW-WIND-DOWN-SPECSPACE wind down specspace @impl/done | `END SESSION <name>`, `WRAP UP <name>` @impl/done | `ЗАВЕРШИ СЕССИЮ <name>`, `ФИКСИРУЕМ <name>` @impl/done |

##THE-NAME-IS-OPTIONAL The name is optional; [target resolution](#resolve) decides what a
phrase with no name means. @impl/done

##RESUME-INTO-A-SPECSPACE **Resume into a specspace** (report-then-wait, as in the wal flow):
read the host's repo-wide rules, the specspace boot contract, the
specspace WAL, the specspace cold-resume file; verify the tree
empirically (branch, sync, working tree, recent commits touching the
specspace); report; stop and wait for direction. @impl/done

##WIND-DOWN-A-SPECSPACE **Wind down a specspace:** rewrite the specspace WAL; overwrite the
specspace cold-resume file wholesale; refresh the specspace's status
line in `SPECSPACES.md`; commit per the host's commit rules; update
the host WAL **only** if host files changed too. @impl/done

## The five laws {#laws}

1. ##LAW-BOOT-SCOPING **Boot scoping.** A specspace session loads the host's repo-wide
   rules and the specspace's own files — nothing else from the host.
   Crossing into host files mid-session is legal but announced. @impl/done
2. ##LAW-STATE-LOCALITY **State locality.** A specspace's canonical state lives in the
   specspace (its WAL). The registry status line is a pointer, not
   state; the host WAL never carries specspace detail beyond "the
   specspace exists; see its WAL". @impl/done
3. ##LAW-ONE-SESSION-ONE-FOCUS **One session, one focus.** A session serves the host or one
   specspace. Work for two projects in one session splits into
   commits per project and updates each project's WAL — and is the
   exception, said out loud, not the habit. @impl/done
4. ##LAW-HOST-RULES-SURVIVE **Host rules survive.** Repo-wide non-negotiables (authorship,
   commit conventions, secrecy rules) bind specspace sessions in
   full. A specspace may add rules; it may not subtract the host's. @impl/done
5. ##LAW-PACKAGE-STATE-STAYS-OUT **Package state stays out.** WAL, cold-resume, and registry files
   are project state. No installable package may create, overwrite,
   or delete them on install or uninstall — the same law the wal
   flow states for its two files. @impl/done

## Lifecycle {#lifecycle}

- ##LIFECYCLE-REGISTER **Register:** create the specspace root with its boot contract,
  WAL, and cold-resume file; add the registry row; mention the
  registry in the host boot contract so sessions recognise the
  grammar. First wind-down validates the loop. @impl/done
- ##LIFECYCLE-RETIRE **Retire:** the specspace graduates to its own repository (its
  files move wholesale; its WAL goes with it) or closes (final
  status line says so; the row moves to a "retired" section — the
  name stays reserved so old phrases fail loudly, not silently). @impl/done
- ##LIFECYCLE-NESTING **Nesting:** one level. A specspace hosting its own specspaces is
  a sign it wants to be a repository. @impl/done

## Re-derive for your project {#re-derive}

##run-this-prompt-once-lead Run this prompt once to adapt the protocol to a concrete host: @impl/done

```
Read SPECSPACES-PROTOCOL.md. Adapt it to this repository:
1. Create SPECSPACES.md at the host root (empty table if no
   specspaces exist yet). Add a `default:` line only if a bare
   session phrase should target a specspace instead of the host.
2. Add a short "Specspaces" section to the host boot contract:
   the grammar, target resolution (bare → default else host), the
   boot-scoping law, and a pointer to the registry.
3. If a sub-project already behaves like a specspace, register it:
   boot contract, WAL, cold-resume file, registry row.
4. Record which host sections are "repo-wide rules" that specspace
   sessions must still load — name them explicitly.
Do not move any existing project state while adapting.
```

## Summary {#summary}

- ##SUM-WHAT-A-SPECSPACE-IS A specspace is a sub-project with first-class session continuity:
  own boot contract, own WAL, own cold-resume file. @impl/done
- ##SUM-THE-REGISTRY `SPECSPACES.md` at the host root is the registry; an optional
  `default:` line sets what a bare phrase targets; its status column
  is a pointer, never canonical state. @impl/done
- ##SUM-TARGET-RESOLUTION Target resolution: an explicit name/directory always wins; a bare
  phrase takes the declared default, else the host — never a
  specspace by accident. @impl/done
- ##SUM-THE-SESSION-GRAMMAR Session phrases gain an optional specspace name; resume stays
  report-then-wait; wind-down operates on the specspace's files. @impl/done
- ##SUM-THE-FIVE-LAWS Five laws: boot scoping, state locality, one focus, host rules
  survive, package state stays out. @impl/done
- ##SUM-NESTING-AND-GRADUATION One nesting level; a specspace that outgrows the host graduates
  to its own repository. @impl/done
