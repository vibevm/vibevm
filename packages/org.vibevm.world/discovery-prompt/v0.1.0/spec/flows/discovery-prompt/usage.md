# Using the DISCOVERY prompt {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file explains *what* the DISCOVERY
prompt is, *when* deploying it pays off, *how* to deploy it, what each
structural piece does, how to read the output it produces, and how to
adapt it to your own domain. @impl/done

##ARTIFACT-SITS-NEXT-TO-THIS-FILE-AND-IS-READ-ONLY The artifact itself sits next to this
file as [`DISCOVERY-PROMPT.md`](DISCOVERY-PROMPT.md), copied verbatim
from its origin — treat it as read-only. @impl/done

## What it is {#what}

##PROMPT-IS-A-DISTRIBUTABLE-USER-LEVEL-PROMPT The DISCOVERY prompt is a **distributable user-level prompt**: a full
user-side alternative to a system prompt. @impl/done

##NEEDS-NO-API-ACCESS-NO-TOOLING It needs no API access, no
custom deployment, no tooling — just a user talking to a capable LLM. @impl/done

##PASTED-FIRST-IT-RESHAPES-THE-SESSION Pasted as the first message of a conversation, it reshapes that
session from "helpful assistant answers questions" into **structured
co-inquiry**: the human contributes domain knowledge, the model
contributes reasoning and pattern-recognition, and neither side
pretends to hold complete answers. @impl/done

##CORE-STANCE-IS-EPISTEMIC Its core stance is epistemic — *"Uncertainty is data, not failure."* @impl/done

##EVERYTHING-ELSE-PROTECTS-THAT-STANCE Everything else in the artifact (the response grammar, the adversarial
self-checks, the intensity knobs) exists to keep that stance from
collapsing back into confident-sounding helper mode. @impl/done

## When to deploy {#when}

##DEPLOY-WHERE-THE-ANSWER-SPACE-IS-OPEN Deploy it where the answer space is genuinely open: @impl/done

- ##OCCASION-RESEARCH-CONVERSATIONS **research
  conversations** (mapping an unfamiliar field, weighing competing
  theories), @impl/done
- ##OCCASION-DESIGN-EXPLORATION **design exploration** (architecture options before any
  code exists), @impl/done
- ##OCCASION-REQUIREMENTS-DISCOVERY **requirements discovery** (working out what the problem
  even is), @impl/done
- ##OCCASION-COMPARATIVE-ANALYSIS and **comparative analysis** (trade-off studies whose
  conclusion is not known in advance). @impl/done

##DO-NOT-DEPLOY-FOR-ROUTINE-CODING Do **not** deploy it for routine coding sessions. @impl/done

##GRAMMAR-TAXES-EVERY-RESPONSE The grammar taxes
every response: each answer must carry a primary hypothesis with a
confidence number, two or three alternative interpretations, and a
meta-reflection. @impl/done

##overhead-buys-nothing-on-a-known-answer On a task with a known right answer — rename this
function, fix this failing test — that overhead buys nothing: the
alternatives are padding and the confidence estimate is theatre. @spec/done

##STRUCTURE-PAYS-ONLY-WHEN-THE-ALTERNATIVES-ARE-LIVE The
structure pays for itself only when the alternatives are live options
and the confidence number tracks something genuinely unresolved. @spec/done

## How to deploy {#deploy}

1. ##STEP-COPY-THE-FULL-TEXT Copy the full text of [`DISCOVERY-PROMPT.md`](DISCOVERY-PROMPT.md). @impl/done
2. ##STEP-FILL-VARIABLES-AND-USER-INFO Fill the `<VARIABLES>` block (which model you are talking to) and
   rewrite `<EXTENSION_USER_INFO>` so it describes *you*, not the
   original author — [§Re-derive](#re-derive) below automates this. @impl/done
3. ##STEP-PASTE-AS-THE-FIRST-MESSAGE Paste the result as the **first message of a fresh session** — not
   into an ongoing conversation. The prompt sets a frame; a session
   that already has momentum in helper mode will not fully re-frame. @impl/done
4. ##STEP-SEND-A-THROWAWAY-GREETING Send a throwaway greeting if your interface needs a second message
   to get going — the artifact's closing section hands the first real
   move to the model. @impl/done

##WORKS-IN-CHAT-PRODUCTS-AND-AGENT-CLIS This works in chat products and agent CLIs alike: anywhere a user can
send a long first message to an LLM. @spec/done

## The structural pieces {#pieces}

| Piece | Kind | Effect |
|---|---|---|
| ##ROW-PIECE-PRIMARY-HYPOTHESIS `PrimaryHypothesis` @impl/done | grammar field @impl/done | The main response, with an explicit 0.0–1.0 confidence estimate. @impl/done |
| ##ROW-PIECE-ALTERNATIVE-INTERPRETATIONS `AlternativeInterpretations` @impl/done | grammar field @impl/done | Two or three genuinely different readings of the problem, every turn. @impl/done |
| ##ROW-PIECE-META-REFLECTION `MetaReflection` @impl/done | grammar field @impl/done | A note on the computational approach ("Am I pattern-matching or reasoning?") plus conversation dynamics and forward directions. @impl/done |
| ##ROW-PIECE-ADVERSARIAL-AMPLIFICATION ADVERSARIAL AMPLIFICATION @impl/done | standing rule @impl/done | At least one substantive self-objection before finalizing, from a different level of analysis; a missing objection is itself a diagnostic (see below). @impl/done |
| ##ROW-PIECE-VARIABLES `<VARIABLES>` @impl/done | macro block @impl/done | Names the target model; macro-replaced through the `MAIN` and `EXTENSION` blocks. @impl/done |
| ##ROW-PIECE-EXTENSION-USER-INFO `<EXTENSION_USER_INFO>` @impl/done | extension @impl/done | Who the human is — background, stack, current focus. The model calibrates depth against it. Always on. @impl/done |
| ##ROW-PIECE-EXTENSION-PREFERRED-LANGUAGE `<EXTENSION_PREFERRED_LANGUAGE>` @impl/done | extension @impl/done | Mirror the language of the last prompt, natural and programming alike; translate protocol terms into established equivalents, not calques. @impl/done |
| ##ROW-PIECE-EXTENSION-SUPERTHINK `<EXTENSION_SUPERTHINK>` @impl/done | extension @impl/done | `#superthink` — maximum-effort thinking for one prompt. Off unless the hashtag appears. @impl/done |
| ##ROW-PIECE-EXTENSION-OPEN-SOURCE-LICENSES `<EXTENSION_OPEN_SOURCE_LICENSES>` @impl/done | extension @impl/done | Licensing policy for recommendations: permissive-first, viral licenses flagged and given workarounds. @impl/done |
| ##ROW-PIECE-EXTENSION-CRITICALITY `<EXTENSION_CRITICALITY>` @impl/done | extension @impl/done | The intensity machinery: `#hot` / `#cold` / `#reboil` knobs, an always-on annealing triage, and the anti-costume test. @impl/done |
| ##ROW-PIECE-LICENSE `<LICENSE>` @impl/done | terms @impl/done | UPL-1.0. Removable when you use the prompt privately; **not** removable when you distribute the text. @impl/done |

## How to read the output {#read}

- ##READ-CONFIDENCE-NUMBERS-AS-CALIBRATION-AIDS **Confidence numbers are calibration aids, not decoration.** Watch
  them move: a claim whose confidence climbs under challenge is
  crystallizing; one that never moves is not being re-examined. @spec/done
- ##READ-ALTERNATIVES-AS-THE-ANTI-ANCHORING-DEVICE **AlternativeInterpretations are the anti-anchoring device.** They
  keep the session from committing to the first plausible reading of
  your question. When one alternative keeps resurfacing turn after
  turn, promote it — that is the conversation telling you where the
  live fork is. @spec/done
- ##READ-META-REFLECTION-AS-THE-DRIFT-DETECTOR **MetaReflection is where drift shows first.** Before the answers
  themselves degrade, the reflections go generic — "depth increased,
  novelty 7/10" with nothing behind it. The artifact contrasts a good
  and a bad meta-reflection explicitly; when yours start resembling
  the bad one, intervene (see [§Failure modes](#failures)). @spec/done
- ##READ-THE-MANDATORY-OBJECTION **A mandatory objection precedes every finalized answer.** The
  ADVERSARIAL AMPLIFICATION rule requires a counter-argument strong
  enough that, were it correct, the primary hypothesis would need
  significant revision — and if no substantive counter-argument
  emerges, the model must flag that it is pattern-matching and
  restart. An output missing its objection paragraph is off-protocol. @impl/done

## The intensity knobs {#knobs}

##EVERY-CONTROL-IS-A-LITERAL-HASHTAG Every criticality control — `#hot`, `#cold`, `#reboil` and their
modifiers — is a literal hashtag, `#` plus the exact token, case-insensitive.
`#superthink` is the documented exception: it ships as its own extension block
above the criticality one and fires on the bare word too. @impl/done

##BARE-WORDS-NEVER-FIRE-THE-MACHINERY Bare `hot` and `cold` in ordinary prose never fire the criticality
machinery — the words are generic, and only the `#`-prefixed token triggers. @impl/done

##DEFAULT-WITH-NO-HASHTAG With
no hashtag the session runs the default: robust on established facts,
exploratory on the open frontier. @impl/done

- ##KNOB-HOT `#hot` — force exploration this turn. The model names the modal
  (template) answer, then nucleates at least one discrete candidate
  from the tail of the distribution. Turn it when answers converge too
  fast or smell like the textbook. Modifiers: `#adaptive` (default —
  push where the current answer is weakest) or `#fixed` (run the four
  standard destabilizing operators as listed, more reproducible). @impl/done
- ##KNOB-COLD `#cold` — force consolidation this turn. Stop generating candidates,
  freeze the best survivor, verify it hard, lock it in. Turn it when
  the table is full of options and you need a decision. @impl/done
- ##KNOB-REBOIL `#reboil`, or `#reboil <target>` — one self-correction cycle aimed
  backwards at the previous output: find the weakest load-bearing
  claim, re-open it, try to replace it, report the verdict. Use it
  when you have no specific challenge but suspect weak spots. Honest
  stall is part of the contract: the artifact obliges the model to
  say "this reboil found no improvement that survives" rather than
  invent a marginal change to look productive — believe that report. @impl/done
- ##KNOB-SUPERTHINK `#superthink` — maximum-effort thinking for one hard prompt. Save it
  for the questions that deserve it. @impl/done

## Failure modes {#failures}

- ##FAILURE-MODEL-IGNORES-THE-GRAMMAR **The model ignores the grammar.** Responses arrive as ordinary
  prose with no hypothesis, alternatives, or reflection. Re-paste the
  STRUCTURAL REQUIREMENTS block from the artifact as your next
  message and ask for the previous answer in protocol form. @spec/done
- ##FAILURE-SYCOPHANCY-RELAPSE **Sycophancy relapse.** The model starts agreeing with your framing
  instead of testing it — objections go soft, confidence numbers drift
  up in lockstep with your enthusiasm. Invoke the artifact's
  anti-costume test by name and demand a real adversarial
  amplification against the current favorite. @spec/done
- ##FAILURE-COSTUME-MODE **Costume mode.** The vocabulary changed but the behavior did not:
  the output is dressed in protocol lexicon while every answer is
  still the modal one. The artifact's own test is the question "did
  behavior change measurably, or only vocabulary?" — if the honest
  answer is "only vocabulary" and a flag-and-rewrite does not fix it,
  restart the session; re-framing mid-stream costs more than a fresh
  deployment. @spec/done

## Re-derive for your project {#re-derive}

##ARTIFACT-IS-READ-ONLY-BUT-BUILT-TO-BE-ADAPTED The artifact is read-only, but it is built to be adapted: the
`<VARIABLES>` block and the EXTENSION blocks that describe *you* are
configuration; `<MAIN>` and `<EXTENSION_CRITICALITY>` are mechanism. The
re-derive prompt below draws the same line at its step 4. @impl/done

##re-derive-prompt-lead Hand your assistant this prompt to produce a
personalized copy: @impl/done

```
Read spec/flows/discovery-prompt/DISCOVERY-PROMPT.md in full. Produce
an adapted copy for me — do not edit the original file.
1. In <VARIABLES>, set LLM to the model I actually talk to.
2. Rewrite <EXTENSION_USER_INFO> for me: name, background, current
   stack, what I am researching. Ask me for anything you cannot infer.
3. Keep, drop, or rewrite <EXTENSION_OPEN_SOURCE_LICENSES> and
   <EXTENSION_PREFERRED_LANGUAGE> to match my policies and languages.
4. Leave <MAIN>, <EXTENSION_CRITICALITY>, and the response grammar
   untouched — they are the mechanism, not the configuration.
5. Keep the <LICENSE> block intact if the copy will ever leave my
   machine; it is removable only for private personal use.
Output the adapted prompt as one paste-ready block, then list every
change you made against the original.
```

## Summary {#summary}

- ##SUM-USER-LEVEL-PROMPT-NO-TOOLING A user-level prompt: paste it as the first message of a fresh
  session; no tooling required. @impl/done
- ##SUM-DEPLOY-FOR-OPEN-ENDED-WORK Deploy for open-ended research, design, requirements, and
  comparative work — never for routine coding. @impl/done
- ##SUM-READ-THE-STRUCTURE-NOT-THE-PROSE Read the structure, not just the prose: confidence movement,
  recurring alternatives, and the quality of meta-reflections carry
  the signal. @spec/done
- ##SUM-STEER-INTENSITY-WITH-HASHTAGS Steer intensity with hashtags: `#hot` to explore, `#cold` to decide,
  `#reboil` to self-audit, `#superthink` for the hardest prompts. @impl/done
- ##SUM-WHEN-THE-PROTOCOL-SLIPS When the protocol slips: re-paste the requirements; when it turns
  into costume: restart. @impl/done
- ##SUM-ADAPT-NEVER-EDIT-THE-SHIPPED-ARTIFACT Adapt via the re-derive prompt; never edit the shipped artifact, and
  keep its license block when the text travels. @impl/done
