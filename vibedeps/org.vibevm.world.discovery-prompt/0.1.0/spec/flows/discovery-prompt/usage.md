# Using the DISCOVERY prompt {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file explains *what* the DISCOVERY
prompt is, *when* deploying it pays off, *how* to deploy it, what each
structural piece does, how to read the output it produces, and how to
adapt it to your own domain. @status:impl/done

@fact:ARTIFACT-SITS-NEXT-TO-THIS-FILE-AND-IS-READ-ONLY The artifact itself sits next to this
file as [`DISCOVERY-PROMPT.md`](DISCOVERY-PROMPT.md), copied verbatim
from its origin — treat it as read-only. @status:impl/done

## What it is {#what}

@fact:PROMPT-IS-A-DISTRIBUTABLE-USER-LEVEL-PROMPT The DISCOVERY prompt is a **distributable user-level prompt**: a full
user-side alternative to a system prompt. @status:impl/done

@fact:NEEDS-NO-API-ACCESS-NO-TOOLING It needs no API access, no
custom deployment, no tooling — just a user talking to a capable LLM. @status:impl/done

@fact:PASTED-FIRST-IT-RESHAPES-THE-SESSION Pasted as the first message of a conversation, it reshapes that
session from "helpful assistant answers questions" into **structured
co-inquiry**: the human contributes domain knowledge, the model
contributes reasoning and pattern-recognition, and neither side
pretends to hold complete answers. @status:impl/done

@fact:CORE-STANCE-IS-EPISTEMIC Its core stance is epistemic — *"Uncertainty is data, not failure."* @status:impl/done

@fact:EVERYTHING-ELSE-PROTECTS-THAT-STANCE Everything else in the artifact (the response grammar, the adversarial
self-checks, the intensity knobs) exists to keep that stance from
collapsing back into confident-sounding helper mode. @status:impl/done

## When to deploy {#when}

@fact:DEPLOY-WHERE-THE-ANSWER-SPACE-IS-OPEN Deploy it where the answer space is genuinely open: @status:impl/done

- @fact:OCCASION-RESEARCH-CONVERSATIONS **research
  conversations** (mapping an unfamiliar field, weighing competing
  theories), @status:impl/done
- @fact:OCCASION-DESIGN-EXPLORATION **design exploration** (architecture options before any
  code exists), @status:impl/done
- @fact:OCCASION-REQUIREMENTS-DISCOVERY **requirements discovery** (working out what the problem
  even is), @status:impl/done
- @fact:OCCASION-COMPARATIVE-ANALYSIS and **comparative analysis** (trade-off studies whose
  conclusion is not known in advance). @status:impl/done

@fact:DO-NOT-DEPLOY-FOR-ROUTINE-CODING Do **not** deploy it for routine coding sessions. @status:impl/done

@fact:GRAMMAR-TAXES-EVERY-RESPONSE The grammar taxes
every response: each answer must carry a primary hypothesis with a
confidence number, two or three alternative interpretations, and a
meta-reflection. @status:impl/done

@fact:overhead-buys-nothing-on-a-known-answer On a task with a known right answer — rename this
function, fix this failing test — that overhead buys nothing: the
alternatives are padding and the confidence estimate is theatre. @status:spec/done

@fact:STRUCTURE-PAYS-ONLY-WHEN-THE-ALTERNATIVES-ARE-LIVE The
structure pays for itself only when the alternatives are live options
and the confidence number tracks something genuinely unresolved. @status:spec/done

## How to deploy {#deploy}

1. @fact:STEP-COPY-THE-FULL-TEXT Copy the full text of [`DISCOVERY-PROMPT.md`](DISCOVERY-PROMPT.md). @status:impl/done
2. @fact:STEP-FILL-VARIABLES-AND-USER-INFO Fill the `<VARIABLES>` block (which model you are talking to) and
   rewrite `<EXTENSION_USER_INFO>` so it describes *you*, not the
   original author — [§Re-derive](#re-derive) below automates this. @status:impl/done
3. @fact:STEP-PASTE-AS-THE-FIRST-MESSAGE Paste the result as the **first message of a fresh session** — not
   into an ongoing conversation. The prompt sets a frame; a session
   that already has momentum in helper mode will not fully re-frame. @status:impl/done
4. @fact:STEP-SEND-A-THROWAWAY-GREETING Send a throwaway greeting if your interface needs a second message
   to get going — the artifact's closing section hands the first real
   move to the model. @status:impl/done

@fact:WORKS-IN-CHAT-PRODUCTS-AND-AGENT-CLIS This works in chat products and agent CLIs alike: anywhere a user can
send a long first message to an LLM. @status:spec/done

## The structural pieces {#pieces}

| Piece | Kind | Effect |
|---|---|---|
| @fact:ROW-PIECE-PRIMARY-HYPOTHESIS `PrimaryHypothesis` @status:impl/done | grammar field @status:impl/done | The main response, with an explicit 0.0–1.0 confidence estimate. @status:impl/done |
| @fact:ROW-PIECE-ALTERNATIVE-INTERPRETATIONS `AlternativeInterpretations` @status:impl/done | grammar field @status:impl/done | Two or three genuinely different readings of the problem, every turn. @status:impl/done |
| @fact:ROW-PIECE-META-REFLECTION `MetaReflection` @status:impl/done | grammar field @status:impl/done | A note on the computational approach ("Am I pattern-matching or reasoning?") plus conversation dynamics and forward directions. @status:impl/done |
| @fact:ROW-PIECE-ADVERSARIAL-AMPLIFICATION ADVERSARIAL AMPLIFICATION @status:impl/done | standing rule @status:impl/done | At least one substantive self-objection before finalizing, from a different level of analysis; a missing objection is itself a diagnostic (see below). @status:impl/done |
| @fact:ROW-PIECE-VARIABLES `<VARIABLES>` @status:impl/done | macro block @status:impl/done | Names the target model; macro-replaced through the `MAIN` and `EXTENSION` blocks. @status:impl/done |
| @fact:ROW-PIECE-EXTENSION-USER-INFO `<EXTENSION_USER_INFO>` @status:impl/done | extension @status:impl/done | Who the human is — background, stack, current focus. The model calibrates depth against it. Always on. @status:impl/done |
| @fact:ROW-PIECE-EXTENSION-PREFERRED-LANGUAGE `<EXTENSION_PREFERRED_LANGUAGE>` @status:impl/done | extension @status:impl/done | Mirror the language of the last prompt, natural and programming alike; translate protocol terms into established equivalents, not calques. @status:impl/done |
| @fact:ROW-PIECE-EXTENSION-SUPERTHINK `<EXTENSION_SUPERTHINK>` @status:impl/done | extension @status:impl/done | `#superthink` — maximum-effort thinking for one prompt. Off unless the hashtag appears. @status:impl/done |
| @fact:ROW-PIECE-EXTENSION-OPEN-SOURCE-LICENSES `<EXTENSION_OPEN_SOURCE_LICENSES>` @status:impl/done | extension @status:impl/done | Licensing policy for recommendations: permissive-first, viral licenses flagged and given workarounds. @status:impl/done |
| @fact:ROW-PIECE-EXTENSION-CRITICALITY `<EXTENSION_CRITICALITY>` @status:impl/done | extension @status:impl/done | The intensity machinery: `#hot` / `#cold` / `#reboil` knobs, an always-on annealing triage, and the anti-costume test. @status:impl/done |
| @fact:ROW-PIECE-LICENSE `<LICENSE>` @status:impl/done | terms @status:impl/done | UPL-1.0. Removable when you use the prompt privately; **not** removable when you distribute the text. @status:impl/done |

## How to read the output {#read}

- @fact:READ-CONFIDENCE-NUMBERS-AS-CALIBRATION-AIDS **Confidence numbers are calibration aids, not decoration.** Watch
  them move: a claim whose confidence climbs under challenge is
  crystallizing; one that never moves is not being re-examined. @status:spec/done
- @fact:READ-ALTERNATIVES-AS-THE-ANTI-ANCHORING-DEVICE **AlternativeInterpretations are the anti-anchoring device.** They
  keep the session from committing to the first plausible reading of
  your question. When one alternative keeps resurfacing turn after
  turn, promote it — that is the conversation telling you where the
  live fork is. @status:spec/done
- @fact:READ-META-REFLECTION-AS-THE-DRIFT-DETECTOR **MetaReflection is where drift shows first.** Before the answers
  themselves degrade, the reflections go generic — "depth increased,
  novelty 7/10" with nothing behind it. The artifact contrasts a good
  and a bad meta-reflection explicitly; when yours start resembling
  the bad one, intervene (see [§Failure modes](#failures)). @status:spec/done
- @fact:READ-THE-MANDATORY-OBJECTION **A mandatory objection precedes every finalized answer.** The
  ADVERSARIAL AMPLIFICATION rule requires a counter-argument strong
  enough that, were it correct, the primary hypothesis would need
  significant revision — and if no substantive counter-argument
  emerges, the model must flag that it is pattern-matching and
  restart. An output missing its objection paragraph is off-protocol. @status:impl/done

## The intensity knobs {#knobs}

@fact:EVERY-CONTROL-IS-A-LITERAL-HASHTAG Every criticality control — `#hot`, `#cold`, `#reboil` and their
modifiers — is a literal hashtag, `#` plus the exact token, case-insensitive.
`#superthink` is the documented exception: it ships as its own extension block
above the criticality one and fires on the bare word too. @status:impl/done

@fact:BARE-WORDS-NEVER-FIRE-THE-MACHINERY Bare `hot` and `cold` in ordinary prose never fire the criticality
machinery — the words are generic, and only the `#`-prefixed token triggers. @status:impl/done

@fact:DEFAULT-WITH-NO-HASHTAG With
no hashtag the session runs the default: robust on established facts,
exploratory on the open frontier. @status:impl/done

- @fact:KNOB-HOT `#hot` — force exploration this turn. The model names the modal
  (template) answer, then nucleates at least one discrete candidate
  from the tail of the distribution. Turn it when answers converge too
  fast or smell like the textbook. Modifiers: `#adaptive` (default —
  push where the current answer is weakest) or `#fixed` (run the four
  standard destabilizing operators as listed, more reproducible). @status:impl/done
- @fact:KNOB-COLD `#cold` — force consolidation this turn. Stop generating candidates,
  freeze the best survivor, verify it hard, lock it in. Turn it when
  the table is full of options and you need a decision. @status:impl/done
- @fact:KNOB-REBOIL `#reboil`, or `#reboil <target>` — one self-correction cycle aimed
  backwards at the previous output: find the weakest load-bearing
  claim, re-open it, try to replace it, report the verdict. Use it
  when you have no specific challenge but suspect weak spots. Honest
  stall is part of the contract: the artifact obliges the model to
  say "this reboil found no improvement that survives" rather than
  invent a marginal change to look productive — believe that report. @status:impl/done
- @fact:KNOB-SUPERTHINK `#superthink` — maximum-effort thinking for one hard prompt. Save it
  for the questions that deserve it. @status:impl/done

## Failure modes {#failures}

- @fact:FAILURE-MODEL-IGNORES-THE-GRAMMAR **The model ignores the grammar.** Responses arrive as ordinary
  prose with no hypothesis, alternatives, or reflection. Re-paste the
  STRUCTURAL REQUIREMENTS block from the artifact as your next
  message and ask for the previous answer in protocol form. @status:spec/done
- @fact:FAILURE-SYCOPHANCY-RELAPSE **Sycophancy relapse.** The model starts agreeing with your framing
  instead of testing it — objections go soft, confidence numbers drift
  up in lockstep with your enthusiasm. Invoke the artifact's
  anti-costume test by name and demand a real adversarial
  amplification against the current favorite. @status:spec/done
- @fact:FAILURE-COSTUME-MODE **Costume mode.** The vocabulary changed but the behavior did not:
  the output is dressed in protocol lexicon while every answer is
  still the modal one. The artifact's own test is the question "did
  behavior change measurably, or only vocabulary?" — if the honest
  answer is "only vocabulary" and a flag-and-rewrite does not fix it,
  restart the session; re-framing mid-stream costs more than a fresh
  deployment. @status:spec/done

## Re-derive for your project {#re-derive}

@fact:ARTIFACT-IS-READ-ONLY-BUT-BUILT-TO-BE-ADAPTED The artifact is read-only, but it is built to be adapted: the
`<VARIABLES>` block and the EXTENSION blocks that describe *you* are
configuration; `<MAIN>` and `<EXTENSION_CRITICALITY>` are mechanism. The
re-derive prompt below draws the same line at its step 4. @status:impl/done

@fact:re-derive-prompt-lead Hand your assistant this prompt to produce a
personalized copy: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-discovery-prompt/<version>/spec/flows/discovery-prompt/`, check `vibe.lock`) DISCOVERY-PROMPT.md in full. Produce
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

- @fact:SUM-USER-LEVEL-PROMPT-NO-TOOLING A user-level prompt: paste it as the first message of a fresh
  session; no tooling required. @status:impl/done
- @fact:SUM-DEPLOY-FOR-OPEN-ENDED-WORK Deploy for open-ended research, design, requirements, and
  comparative work — never for routine coding. @status:impl/done
- @fact:SUM-READ-THE-STRUCTURE-NOT-THE-PROSE Read the structure, not just the prose: confidence movement,
  recurring alternatives, and the quality of meta-reflections carry
  the signal. @status:spec/done
- @fact:SUM-STEER-INTENSITY-WITH-HASHTAGS Steer intensity with hashtags: `#hot` to explore, `#cold` to decide,
  `#reboil` to self-audit, `#superthink` for the hardest prompts. @status:impl/done
- @fact:SUM-WHEN-THE-PROTOCOL-SLIPS When the protocol slips: re-paste the requirements; when it turns
  into costume: restart. @status:impl/done
- @fact:SUM-ADAPT-NEVER-EDIT-THE-SHIPPED-ARTIFACT Adapt via the re-derive prompt; never edit the shipped artifact, and
  keep its license block when the text travels. @status:impl/done
