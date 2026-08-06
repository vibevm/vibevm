# `flow:operating-modes` — codeword-triggered work postures {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-OPERATING-MODES A vibevm `flow` package that installs **operating modes**: a project
keeps one safe default posture and lets the owner flip into an
alternate posture for one work cycle by speaking a catalogued
codeword. @status:impl/done

@fact:A-MODE-CHANGES-CONFIRMATION-BEHAVIOUR-ONLY A mode changes *confirmation behaviour* — it never changes
what is off-limits. @status:impl/done

@fact:A-SMALL-SET-OF-RED-LINES-SURVIVES-EVERY-MODE A small set of red lines survives every mode. @status:impl/done

@fact:THE-DEFAULT-POSTURE-PREFERS-A-QUESTION-TO-A-MISTAKE The default posture prefers a clarifying question to an irreversible
mistake — right for most sessions, wrong for sustained heads-down
work the owner has already thought through and approved. @status:impl/done

@fact:the-handshake-is-overhead-already-paid-for There, every
"shall I proceed?" is overhead already paid for. @status:spec/done

@fact:A-CODEWORD-REMOVES-THE-OVERHEAD-NOT-THE-GUARDRAILS A codeword removes
that overhead without removing the guardrails that matter. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/operating-modes/OPERATING-MODES-PROTOCOL.md` — what a
  mode is, the five-part codeword shape, and the red-lines law that
  no codeword can erode. @status:impl/done
- @fact:CONTENT-THE-MFBT-MODE `spec/flows/operating-modes/mfbt-mode.md` — a worked reference
  codeword, «move fast and break things»: pre-authorised heads-down
  execution, fully specified in the five-part shape. @status:impl/done
- @fact:CONTENT-WRITING-A-CODEWORD `spec/flows/operating-modes/writing-a-codeword.md` — the procedure
  for adding a new mode to a project's catalogue, with anti-patterns. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/45-flow-operating-modes.md` — boot snippet: the default
  posture, how codewords fire, the red-lines law, and the never-do
  list. @status:impl/done

## Install {#install}

```bash
vibe install flow:operating-modes
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:operating-modes
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-CONFLICT-PROTOCOL `flow:conflict-protocol` — a mode never overrides the
  uncertainty protocol; genuine ambiguity still takes the
  conservative default and a REVIEW marker, even at full speed. @status:impl/done
- @fact:COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — an accelerated mode still commits one
  logical unit at a time; its frozen-history rule is one of the red
  lines every mode preserves. @status:impl/done
- @fact:COMPOSES-SECRETS-HYGIENE-AND-ATTRIBUTION-POLICY `flow:secrets-hygiene` and `flow:git-attribution-policy` — standing
  invariants that are red lines for mode purposes: no posture relaxes
  them. @status:impl/done
- @fact:COMPOSES-CAMPAIGN-PLANS `flow:campaign-plans` — a campaign is often run under an
  accelerated mode; the plan's phase gates supply the "status, not
  requests" cadence. @status:spec/done

## Philosophical background {#background}

@fact:extracted-from-the-origin-projects-law Extracted from the origin project's operating-modes law, which
generalized its own session-end codeword into a family of postures. @status:spec/done

@fact:collections-spirit-is-the-redbook
The collection's spirit is the book *AI-native development*, shipped
in Russian inside `flow:redbook` at `spec/book/ru/`. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done
