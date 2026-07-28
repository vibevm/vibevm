# `flow:operating-modes` — codeword-triggered work postures {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-OPERATING-MODES A vibevm `flow` package that installs **operating modes**: a project
keeps one safe default posture and lets the owner flip into an
alternate posture for one work cycle by speaking a catalogued
codeword. @impl/done

##A-MODE-CHANGES-CONFIRMATION-BEHAVIOUR-ONLY A mode changes *confirmation behaviour* — it never changes
what is off-limits. @impl/done

##A-SMALL-SET-OF-RED-LINES-SURVIVES-EVERY-MODE A small set of red lines survives every mode. @impl/done

##THE-DEFAULT-POSTURE-PREFERS-A-QUESTION-TO-A-MISTAKE The default posture prefers a clarifying question to an irreversible
mistake — right for most sessions, wrong for sustained heads-down
work the owner has already thought through and approved. @impl/done

##the-handshake-is-overhead-already-paid-for There, every
"shall I proceed?" is overhead already paid for. @spec/done

##A-CODEWORD-REMOVES-THE-OVERHEAD-NOT-THE-GUARDRAILS A codeword removes
that overhead without removing the guardrails that matter. @impl/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-PROTOCOL `spec/flows/operating-modes/OPERATING-MODES-PROTOCOL.md` — what a
  mode is, the five-part codeword shape, and the red-lines law that
  no codeword can erode. @impl/done
- ##CONTENT-THE-MFBT-MODE `spec/flows/operating-modes/mfbt-mode.md` — a worked reference
  codeword, «move fast and break things»: pre-authorised heads-down
  execution, fully specified in the five-part shape. @impl/done
- ##CONTENT-WRITING-A-CODEWORD `spec/flows/operating-modes/writing-a-codeword.md` — the procedure
  for adding a new mode to a project's catalogue, with anti-patterns. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/45-flow-operating-modes.md` — boot snippet: the default
  posture, how codewords fire, the red-lines law, and the never-do
  list. @impl/done

## Install {#install}

```bash
vibe install flow:operating-modes
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:operating-modes
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Composition {#composition}

- ##COMPOSES-CONFLICT-PROTOCOL `flow:conflict-protocol` — a mode never overrides the
  uncertainty protocol; genuine ambiguity still takes the
  conservative default and a REVIEW marker, even at full speed. @impl/done
- ##COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — an accelerated mode still commits one
  logical unit at a time; its frozen-history rule is one of the red
  lines every mode preserves. @impl/done
- ##COMPOSES-SECRETS-HYGIENE-AND-ATTRIBUTION-POLICY `flow:secrets-hygiene` and `flow:git-attribution-policy` — standing
  invariants that are red lines for mode purposes: no posture relaxes
  them. @impl/done
- ##COMPOSES-CAMPAIGN-PLANS `flow:campaign-plans` — a campaign is often run under an
  accelerated mode; the plan's phase gates supply the "status, not
  requests" cadence. @spec/done

## Philosophical background {#background}

##extracted-from-the-origin-projects-law Extracted from the origin project's operating-modes law, which
generalized its own session-end codeword into a family of postures. @spec/done

##collections-spirit-is-the-redbook
The collection's spirit is the book *AI-native development*, shipped
in Russian inside `flow:redbook` at `spec/book/ru/`. @spec/done

## License {#license}

##license-line UPL-1.0. See `LICENSE.md`. @impl/done
