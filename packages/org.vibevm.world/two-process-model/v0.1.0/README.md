# `flow:two-process-model` — human and AI as coprocessors {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-INSTALLS-THE-TWO-PROCESS-MODEL A vibevm `flow` package that installs the **two-process model** — the
foundational mental model of the redbook collection. @impl/done

##the-human-and-the-ai-are-two-processors The human and
the AI are two processors with radically different architectures
working one task: @spec/done

- ##THE-HUMAN-OWNS-MEANING-MEMORY-AND-COHERENCE the human owns meaning, memory between sessions,
  and coherence; @spec/done
- ##THE-AI-OWNS-THROUGHPUT-CONSISTENCY-AND-STRUCTURE the AI owns throughput, mechanical consistency, and
  formal structure. @spec/done

##FILES-ARE-THE-ONLY-MEMORY-THE-TWO-SHARE Files are the only memory the two share. @spec/done

##both-default-metaphors-collapse-the-same-way The two default metaphors — "boss and subordinate", "human and
tool" — both collapse on real projects, and both for the same
reason: the human ends up carrying one hundred percent of the
cognitive load. @spec/done

##PACKAGE-INSTALLS-THE-ALTERNATIVE-AS-STANDING-INSTRUCTIONS This package installs the alternative as standing
session instructions plus three reference documents. @impl/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-MODEL `spec/flows/two-process-model/TWO-PROCESS-MODEL.md` — the model:
  why the common metaphors fail, the complementary strengths table,
  the productive cycle, and the one assignment that never moves
  (the human owns coherence). @impl/done
- ##CONTENT-THE-COGNITIVE-LOAD-SPLIT `spec/flows/two-process-model/cognitive-load-split.md` — the
  operational responsibility table: human-only work, AI-only work,
  shared work split by nature; and the four consequences of the
  AI's zero cross-session memory. @impl/done
- ##CONTENT-FILES-AS-IPC `spec/flows/two-process-model/files-as-ipc.md` — the reframe of
  spec files from "documentation" to the inter-process channel:
  three planes, their budgets, and the four IPC requirements. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/05-flow-two-process-model.md` — boot snippet loaded at
  session start: the architecture in brief and the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:two-process-model
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:two-process-model
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the
boot snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Composition {#composition}

##this-flow-is-the-root-of-the-collection This flow is the root of the redbook collection — the other members
are its consequences: @spec/done

- ##COMPOSES-THE-FOUR-IPC-REQUIREMENTS The four IPC requirements map to `flow:addressable-specs`
  (addressability), `flow:git-atomic-commits` (atomicity),
  `flow:conflict-protocol` (conflict rules), and `flow:wal` plus
  `flow:sync-from-code` (visibility). @impl/done
- ##COMPOSES-THE-MEMORY-ASYMMETRY The memory asymmetry is operationalized by
  `flow:decision-records` (record decisions, not facts) and
  `flow:wal` (the checkpoint that survives the session). @impl/done
- ##COMPOSES-CAMPAIGN-PLANS Coherence at multi-session scale is `flow:campaign-plans`. @impl/done
- ##COMPOSES-DISCOVERY-PROMPT Programming the AI process's reasoning posture for research is
  `flow:discovery-prompt`. @impl/done

## Philosophical background {#background}

##distilled-from-chapters-one-and-two Distilled from *AI-native development*, chapter 1 («Два процесса,
одна задача») and chapter 2 («Shared state: файлы как IPC»). @spec/done

##collections-spirit-is-the-redbook The
book ships in Russian inside `flow:redbook` at `spec/book/ru/`; the
collection takes the general spirit of the process from it. @spec/done

## License {#license}

##license-line UPL-1.0. See `LICENSE.md`. @impl/done
