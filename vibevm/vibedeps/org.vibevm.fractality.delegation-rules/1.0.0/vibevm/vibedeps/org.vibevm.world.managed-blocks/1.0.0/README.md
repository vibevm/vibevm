# `flow:managed-blocks` — write into files you do not own {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-MANAGED-BLOCKS-DISCIPLINE A `flow` package that installs one discipline: **how a tool writes
into a file it does not own** — an agent-instruction file, a shell rc,
an ssh config, a shared project config — without destroying what the
other tenants put there. @status:impl/done

@fact:FIRST-INSTALL-INTO-A-NON-TRIVIAL-HOST-FILE-IS-A-DATA-LOSS-EVENT The first install of a whole-file-overwriting
tool into a project with a non-trivial host file is a data-loss event;
this practice is how it stops being one. @status:spec/done

@fact:AUDIENCE-IS-TOOL-AUTHORS **Audience: tool authors.** Anyone whose software writes into files it
does not solely own. @status:impl/done

@fact:IF-YOUR-TOOL-WRITES-A-FILE-OTHERS-EDIT-THIS-IS-FOR-YOU If your tool `write()`s a file a human or another
tool also edits, this is for you. @status:impl/done

@fact:the-law-fits-on-one-line The law fits on one line: @status:impl/done

```
Own exactly one delimited block; never touch a byte outside it.
```

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL.xml` — the full
  protocol: the co-tenant law, marker design (unique, greppable,
  paired, self-documenting, versioned), the absent / present /
  malformed state machine, the three verbs (create / update / remove),
  plan-time classification, the byte-identical no-op, and multi-tool
  cohabitation. @status:impl/done
- @fact:CONTENT-THE-REJECTED-DESIGNS `spec/flows/managed-blocks/rejected-designs.xml` — four designs that
  look reasonable and are wrong (sidecar, model-based detection,
  auto-repair, whole-file ownership), each with its full why, plus the
  malformed-state hard-stop drill. @status:impl/done
- @fact:CONTENT-THE-ADOPTION-GUIDE `spec/flows/managed-blocks/adoption-guide.xml` — migrating an
  overwriting tool onto a block, a fixture table that pins the state
  machine, and what belongs inside the block versus a tool-owned file
  it points at. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/65-flow-managed-blocks.xml` — boot snippet loaded at
  session start: the one-line law, when to read the protocol, and the
  never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:managed-blocks
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:managed-blocks
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched — which is, fittingly, the
whole point of the practice. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-TOOL-DESIGN-LESSONS `flow:tool-design-lessons` — the sibling package for tool authors;
  managed blocks is one deep-dive lesson from it, extracted so it can
  be installed on its own. @status:impl/done
- @fact:COMPOSES-ATTRIBUTION-POLICY-AND-WAL `flow:git-attribution-policy` and `flow:wal` — their instruction files
  are exactly the kind of shared, hand-authored file this protocol
  protects. A tool that writes an attribution snippet or a WAL redirect
  into `CLAUDE.md` must do it as a block, not an overwrite. @status:impl/done
- @fact:COMPOSES-CONFLICT-PROTOCOL `flow:conflict-protocol` — a malformed block is a conflict surfaced
  to the human, never silently resolved; both practices refuse to guess
  when two writers disagree, and both hard-stop instead of auto-fixing. @status:impl/done

## Philosophical background {#background}

@fact:practice-crystallized-from-the-origin-projects-law The practice is crystallized from the origin project's
managed-redirect-block law — the rule that a tool owns one delimited,
machine-findable region of a shared file and never touches a byte
outside it, with a hard stop on any malformed state. @status:spec/done

@fact:ORIGIN-MARKERS-ARE-ONE-EXAMPLE-AND-THE-RULE-IS-GENERIC The origin's own
markers (`<vibevm>`) and host file (`CLAUDE.md`) appear here only as
one worked example among others (shell rc, ssh config); the rule is
generic to any tool and any shared file. @status:impl/done

@fact:collections-spirit-is-the-redbook The collection's spirit is the
book *AI-native development*, which ships in Russian inside
`flow:redbook` at `spec/book/ru/`. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done

