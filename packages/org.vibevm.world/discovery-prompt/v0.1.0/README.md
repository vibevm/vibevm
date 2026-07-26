# `flow:discovery-prompt` — structured co-inquiry on demand {#root}

<status stage="doc" state="done" audience="user"/>

##PACKAGE-SHIPS-THE-DISCOVERY-PROMPT A vibevm `flow` package that ships the **DISCOVERY
collaborative-research prompt**: a distributable user-level prompt —
a full user-side alternative to a system prompt — that turns an LLM
session into structured co-inquiry. @impl/done

- ##EVERY-RESPONSE-FOLLOWS-AN-EXPLICIT-GRAMMAR Every response follows an
  explicit grammar (PrimaryHypothesis with a 0.0–1.0 confidence
  estimate, AlternativeInterpretations, MetaReflection), @impl/done
- ##SELF-OBJECTION-PRECEDES-EVERY-FINALIZED-ANSWER a mandatory
  adversarial self-objection precedes every finalized answer, @impl/done
- ##HASHTAG-KNOBS-STEER-THE-SESSION-INTENSITY and
  hashtag knobs (`#hot`, `#cold`, `#reboil`, `#superthink`) steer the
  session's intensity. @impl/done

##STANCE-UNCERTAINTY-IS-DATA-NOT-FAILURE Its stance in one line: uncertainty is data,
not failure. @impl/done

##package-contents-lead This package ships the artifact plus two pieces of guidance: @impl/done

- ##CONTENT-THE-PROMPT-VERBATIM `spec/flows/discovery-prompt/DISCOVERY-PROMPT.md` — the prompt
  itself, **verbatim**. Fill its `<VARIABLES>` block, paste it as
  the first message of a fresh session, and the session re-frames. @impl/done
- ##CONTENT-THE-USAGE-GUIDE `spec/flows/discovery-prompt/usage.md` — when the grammar's
  overhead pays off (and when it does not), a piece-by-piece map of
  the structure, how to read the output, the intensity knobs, the
  failure modes, and a re-derive prompt for adapting the artifact to
  your own domain. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/50-flow-discovery-prompt.md` — boot snippet loaded at
  session start: deploy the artifact on request, never mix its
  grammar into coding sessions uninvited. @impl/done

## Install {#install}

```bash
vibe install flow:discovery-prompt
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:discovery-prompt
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the
boot snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Provenance {#provenance}

##prompt-authored-in-the-origin-project The prompt was authored as a standalone, UPL-licensed artifact in
the origin project's research tree, where it reached version 3
through live research sessions. @spec/done

##PACKAGED-UNMODIFIED-BYTE-FOR-BYTE It is packaged **unmodified**: the
shipped file is a byte-for-byte copy, self-describing down to its
own license block. @spec/done

##ADAPT-YOUR-COPY-NEVER-THE-SHIPPED-FILE Adaptation happens on your copy — via the
re-derive prompt in `usage.md` — never on the shipped file. @impl/done

## Composition {#composition}

- ##COMPOSES-TWO-PROCESS-MODEL `flow:two-process-model` — this package is a tool for the
  human→AI control channel: it programs the AI process's reasoning
  posture for one session. @spec/done
- ##COMPOSES-CAMPAIGN-PLANS `flow:campaign-plans` — a discovery session often precedes a
  campaign plan: research first, arithmetic second. @spec/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — a discovery session's converged answer
  should land as a recorded decision, not evaporate with the chat. @spec/done

## License {#license}

##license-line UPL-1.0. @impl/done

##ARTIFACT-CARRIES-ITS-OWN-LICENSE-BLOCK The artifact carries its own `<LICENSE>` block (removable
in private use, not in redistribution); the package license matches
it. @impl/done
