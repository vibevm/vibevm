# `flow:two-process-model` — human and AI as coprocessors

A vibevm `flow` package that installs the **two-process model** — the
foundational mental model of the redbook collection. The human and
the AI are two processors with radically different architectures
working one task: the human owns meaning, memory between sessions,
and coherence; the AI owns throughput, mechanical consistency, and
formal structure. Files are the only memory the two share.

The two default metaphors — "boss and subordinate", "human and
tool" — both collapse on real projects, and both for the same
reason: the human ends up carrying one hundred percent of the
cognitive load. This package installs the alternative as standing
session instructions plus three reference documents.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/two-process-model/TWO-PROCESS-MODEL.md` — the model:
  why the common metaphors fail, the complementary strengths table,
  the productive cycle, and the one assignment that never moves
  (the human owns coherence).
- `spec/flows/two-process-model/cognitive-load-split.md` — the
  operational responsibility table: human-only work, AI-only work,
  shared work split by nature; and the four consequences of the
  AI's zero cross-session memory.
- `spec/flows/two-process-model/files-as-ipc.md` — the reframe of
  spec files from "documentation" to the inter-process channel:
  three planes, their budgets, and the four IPC requirements.
- `spec/boot/05-flow-two-process-model.md` — boot snippet loaded at
  session start: the architecture in brief and the never-do list.

## Install

```bash
vibe install flow:two-process-model
```

## Uninstall

```bash
vibe uninstall flow:two-process-model
```

Uninstalling removes every file the package wrote, including the
boot snippet. User-owned files are never touched.

## Composition

This flow is the root of the redbook collection — the other members
are its consequences:

- The four IPC requirements map to `flow:addressable-specs`
  (addressability), `flow:atomic-commits` (atomicity),
  `flow:conflict-protocol` (conflict rules), and `flow:wal` plus
  `flow:sync-from-code` (visibility).
- The memory asymmetry is operationalized by
  `flow:decision-records` (record decisions, not facts) and
  `flow:wal` (the checkpoint that survives the session).
- Coherence at multi-session scale is `flow:campaign-plans`.
- Programming the AI process's reasoning posture for research is
  `flow:discovery-prompt`.

## Philosophical background

Distilled from *AI-native development*, chapter 1 («Два процесса,
одна задача») and chapter 2 («Shared state: файлы как IPC»). The
book ships in Russian inside `flow:redbook` at `spec/book/ru/`; the
collection takes the general spirit of the process from it.

## License

UPL-1.0. See `LICENSE.md`.
