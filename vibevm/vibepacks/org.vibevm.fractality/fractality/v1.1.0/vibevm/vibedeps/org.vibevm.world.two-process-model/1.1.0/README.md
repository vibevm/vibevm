# `flow:two-process-model` — human and AI as coprocessors {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-TWO-PROCESS-MODEL A vibevm `flow` package that installs the **two-process model** — the
foundational mental model of the redbook collection. @status:impl/done

@fact:the-human-and-the-ai-are-two-processors The human and
the AI are two processors with radically different architectures
working one task: @status:spec/done

- @fact:THE-HUMAN-OWNS-MEANING-MEMORY-AND-COHERENCE the human owns meaning, memory between sessions,
  and coherence; @status:spec/done
- @fact:THE-AI-OWNS-THROUGHPUT-CONSISTENCY-AND-STRUCTURE the AI owns throughput, mechanical consistency, and
  formal structure. @status:spec/done

@fact:FILES-ARE-THE-ONLY-MEMORY-THE-TWO-SHARE Files are the only memory the two share. @status:spec/done

@fact:both-default-metaphors-collapse-the-same-way The two default metaphors — "boss and subordinate", "human and
tool" — both collapse on real projects, and both for the same
reason: the human ends up carrying one hundred percent of the
cognitive load. @status:spec/done

@fact:PACKAGE-INSTALLS-THE-ALTERNATIVE-AS-STANDING-INSTRUCTIONS This package installs the alternative as standing
session instructions plus three reference documents. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-MODEL `spec/flows/two-process-model/TWO-PROCESS-MODEL.xml` — the model:
  why the common metaphors fail, the complementary strengths table,
  the productive cycle, and the one assignment that never moves
  (the human owns coherence). @status:impl/done
- @fact:CONTENT-THE-COGNITIVE-LOAD-SPLIT `spec/flows/two-process-model/cognitive-load-split.xml` — the
  operational responsibility table: human-only work, AI-only work,
  shared work split by nature; and the four consequences of the
  AI's zero cross-session memory. @status:impl/done
- @fact:CONTENT-FILES-AS-IPC `spec/flows/two-process-model/files-as-ipc.xml` — the reframe of
  spec files from "documentation" to the inter-process channel:
  three planes, their budgets, and the four IPC requirements. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/05-flow-two-process-model.xml` — boot snippet loaded at
  session start: the architecture in brief and the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:two-process-model
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:two-process-model
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the
boot snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

@fact:this-flow-is-the-root-of-the-collection This flow is the root of the redbook collection — the other members
are its consequences: @status:spec/done

- @fact:COMPOSES-THE-FOUR-IPC-REQUIREMENTS The four IPC requirements map to `flow:addressable-specs`
  (addressability), `flow:git-atomic-commits` (atomicity),
  `flow:conflict-protocol` (conflict rules), and `flow:wal` plus
  `flow:sync-from-code` (visibility). @status:impl/done
- @fact:COMPOSES-THE-MEMORY-ASYMMETRY The memory asymmetry is operationalized by
  `flow:decision-records` (record decisions, not facts) and
  `flow:wal` (the checkpoint that survives the session). @status:impl/done
- @fact:COMPOSES-CAMPAIGN-PLANS Coherence at multi-session scale is `flow:campaign-plans`. @status:impl/done
- @fact:COMPOSES-DISCOVERY-PROMPT Programming the AI process's reasoning posture for research is
  `flow:discovery-prompt`. @status:impl/done

## Philosophical background {#background}

@fact:distilled-from-chapters-one-and-two Distilled from *AI-native development*, chapter 1 («Два процесса,
одна задача») and chapter 2 («Shared state: файлы как IPC»). @status:spec/done

@fact:collections-spirit-is-the-redbook The
book ships in Russian inside `flow:redbook` at `spec/book/ru/`; the
collection takes the general spirit of the process from it. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See `LICENSE.md`. @status:impl/done

