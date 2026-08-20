# `flow:managed-blocks` — write into files you do not own {#root}

A `flow` package that installs one discipline: **how a tool writes
into a file it does not own** — an agent-instruction file, a shell rc,
an ssh config, a shared project config — without destroying what the
other tenants put there. The first install of a whole-file-overwriting
tool into a project with a non-trivial host file is a data-loss event;
this practice is how it stops being one.

**Audience: tool authors.** Anyone whose software writes into files it
does not solely own. If your tool `write()`s a file a human or another
tool also edits, this is for you. The law fits on one line:

```
Own exactly one delimited block; never touch a byte outside it.
```

This package ships three pieces of content plus a boot snippet:

- `spec/flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL.md` — the full
  protocol: the co-tenant law, marker design (unique, greppable,
  paired, self-documenting, versioned), the absent / present /
  malformed state machine, the three verbs (create / update / remove),
  plan-time classification, the byte-identical no-op, and multi-tool
  cohabitation.
- `spec/flows/managed-blocks/rejected-designs.md` — four designs that
  look reasonable and are wrong (sidecar, model-based detection,
  auto-repair, whole-file ownership), each with its full why, plus the
  malformed-state hard-stop drill.
- `spec/flows/managed-blocks/adoption-guide.md` — migrating an
  overwriting tool onto a block, a fixture table that pins the state
  machine, and what belongs inside the block versus a tool-owned file
  it points at.
- `spec/boot/65-flow-managed-blocks.md` — boot snippet loaded at
  session start: the one-line law, when to read the protocol, and the
  never-do list.

## Install {#install}

```bash
vibe install flow:managed-blocks
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:managed-blocks
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched — which is, fittingly, the
whole point of the practice.

## Composition {#composition}

- `flow:tool-design-lessons` — the sibling package for tool authors;
  managed blocks is one deep-dive lesson from it, extracted so it can
  be installed on its own.
- `flow:attribution-policy` and `flow:wal` — their instruction files
  are exactly the kind of shared, hand-authored file this protocol
  protects. A tool that writes an attribution snippet or a WAL redirect
  into `CLAUDE.md` must do it as a block, not an overwrite.
- `flow:conflict-protocol` — a malformed block is a conflict surfaced
  to the human, never silently resolved; both practices refuse to guess
  when two writers disagree, and both hard-stop instead of auto-fixing.

## Philosophical background {#background}

The practice is crystallized from the origin project's
managed-redirect-block law — the rule that a tool owns one delimited,
machine-findable region of a shared file and never touches a byte
outside it, with a hard stop on any malformed state. The origin's own
markers (`<vibevm>`) and host file (`CLAUDE.md`) appear here only as
one worked example among others (shell rc, ssh config); the rule is
generic to any tool and any shared file. The collection's spirit is the
book *AI-native development*, which ships in Russian inside
`flow:redbook` at `spec/book/ru/`.

## License {#license}

UPL-1.0. See `LICENSE.md`.
