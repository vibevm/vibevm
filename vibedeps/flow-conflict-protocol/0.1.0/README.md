# `flow:conflict-protocol` — two writers, one truth {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-A-CONFLICT-RESOLUTION-DISCIPLINE A `flow` package that installs a conflict-resolution discipline for
projects where two writers — a human and a coding agent — edit one
file set. @impl/done

##CONTRADICTIONS-ARE-NORMAL-COOPERATION-NOT-AN-ERROR The two *will* write contradictory things; that is normal
cooperation, not an error. @spec/done

##WHAT-THE-DISCIPLINE-FORBIDS-IS-RESOLVING-A-CONTRADICTION-SILENTLY What the discipline forbids is resolving a
contradiction silently: priorities are fixed in advance, disagreement
travels through visible REVIEW markers, and the case where the spec
is simply silent gets a conservative-default ladder instead of
invented semantics. @impl/done

##the-core-law-fits-on-one-line The core law fits on one line: @impl/done

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

##HUMAN-CHANGES-THE-SPEC-CODE-CONFORMS-AND-STATE-FILES-ARE-LAST The human may change the spec; code must conform to the spec; tests
are the spec in executable form; volatile state files are records,
dead last. @impl/done

##RECENCY-DECIDES-NOTHING Recency decides nothing. @impl/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-PROTOCOL `spec/flows/conflict-protocol/CONFLICT-PROTOCOL.md` — the full
  protocol: why conflicts are normal, the hierarchy and what each
  relation means operationally, the four-step REVIEW-marker protocol,
  the marker lifecycle, and a worked example of what one silent
  change costs (one bug becoming three, plus two weeks of git
  archaeology). @impl/done
- ##CONTENT-THE-UNCERTAINTY-PROTOCOL `spec/flows/conflict-protocol/uncertainty-protocol.md` — the
  spec-is-silent path: re-read the spec, re-read the reference, check
  the closest analog, then conservative default + REVIEW + report;
  what "conservative" means (cheapest to reverse), and when to stop
  and ask instead. @impl/done
- ##CONTENT-THE-FAILURE-MODES `spec/flows/conflict-protocol/failure-modes.md` — three named
  failures with recovery drills: the silent spec edit, the stale
  state file after a crash, and the self-contradicting spec. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/35-flow-conflict-protocol.md` — boot snippet loaded at
  session start: the hierarchy one-liner, the REVIEW contract, the
  uncertainty ladder, and the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:conflict-protocol
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:conflict-protocol
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Composition {#composition}

- ##COMPOSES-TWO-PROCESS-MODEL `flow:two-process-model` — the hierarchy is that model's law #1;
  this package is its enforcement arm. @impl/done
- ##COMPOSES-SYNC-FROM-CODE `flow:sync-from-code` — the sanctioned *loud* path for code-first
  changes. This protocol is what makes the silent path forbidden;
  sync-from-code is where legitimate code-first reality goes. @impl/done
- ##COMPOSES-WAL `flow:wal` — the WAL sits at the hierarchy's tail as a volatile
  record; the stale-WAL recovery drill lives in this package's
  failure-modes document. @impl/done
- ##COMPOSES-ADDRESSABLE-SPECS `flow:addressable-specs` — a REVIEW marker and a correction both
  cite the violated anchor; addressability is what makes a dispute
  cost twenty tokens instead of two hundred. @spec/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — a resolved conflict often ends as a
  recorded decision, so the same question is not re-litigated next
  quarter. @spec/done

## Philosophical background {#background}

##practice-extracted-from-the-book The practice is extracted from *AI-native development*, chapters 1–2:
the priority hierarchy and the memory-fence framing come from
chapter 1 (two co-processors sharing files as their only IPC); the
REVIEW protocol, the silent-change data race, and the three failure
modes come from chapter 2. @spec/done

##CHAPTERS-SHIP-IN-RUSSIAN-INSIDE-REDBOOK Both chapters ship in Russian inside
`flow:redbook` at `spec/book/ru/`. @impl/done

## License {#license}

##license-line UPL-1.0. See `LICENSE.md`. @impl/done
