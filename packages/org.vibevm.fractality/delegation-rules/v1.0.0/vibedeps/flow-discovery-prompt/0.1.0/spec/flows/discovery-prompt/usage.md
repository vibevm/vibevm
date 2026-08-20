# Using the DISCOVERY prompt {#root}

**Scope of this document.** This file explains *what* the DISCOVERY
prompt is, *when* deploying it pays off, *how* to deploy it, what each
structural piece does, how to read the output it produces, and how to
adapt it to your own domain. The artifact itself sits next to this
file as [`DISCOVERY-PROMPT.md`](DISCOVERY-PROMPT.md), copied verbatim
from its origin — treat it as read-only.

## What it is {#what}

The DISCOVERY prompt is a **distributable user-level prompt**: a full
user-side alternative to a system prompt. It needs no API access, no
custom deployment, no tooling — just a user talking to a capable LLM.
Pasted as the first message of a conversation, it reshapes that
session from "helpful assistant answers questions" into **structured
co-inquiry**: the human contributes domain knowledge, the model
contributes reasoning and pattern-recognition, and neither side
pretends to hold complete answers.

Its core stance is epistemic — *"Uncertainty is data, not failure."*
Everything else in the artifact (the response grammar, the adversarial
self-checks, the intensity knobs) exists to keep that stance from
collapsing back into confident-sounding helper mode.

## When to deploy {#when}

Deploy it where the answer space is genuinely open: **research
conversations** (mapping an unfamiliar field, weighing competing
theories), **design exploration** (architecture options before any
code exists), **requirements discovery** (working out what the problem
even is), and **comparative analysis** (trade-off studies whose
conclusion is not known in advance).

Do **not** deploy it for routine coding sessions. The grammar taxes
every response: each answer must carry a primary hypothesis with a
confidence number, two or three alternative interpretations, and a
meta-reflection. On a task with a known right answer — rename this
function, fix this failing test — that overhead buys nothing: the
alternatives are padding and the confidence estimate is theatre. The
structure pays for itself only when the alternatives are live options
and the confidence number tracks something genuinely unresolved.

## How to deploy {#deploy}

1. Copy the full text of [`DISCOVERY-PROMPT.md`](DISCOVERY-PROMPT.md).
2. Fill the `<VARIABLES>` block (which model you are talking to) and
   rewrite `<EXTENSION_USER_INFO>` so it describes *you*, not the
   original author — [§Re-derive](#re-derive) below automates this.
3. Paste the result as the **first message of a fresh session** — not
   into an ongoing conversation. The prompt sets a frame; a session
   that already has momentum in helper mode will not fully re-frame.
4. Send a throwaway greeting if your interface needs a second message
   to get going — the artifact's closing section hands the first real
   move to the model.

This works in chat products and agent CLIs alike: anywhere a user can
send a long first message to an LLM.

## The structural pieces {#pieces}

| Piece | Kind | Effect |
|---|---|---|
| `PrimaryHypothesis` | grammar field | The main response, with an explicit 0.0–1.0 confidence estimate. |
| `AlternativeInterpretations` | grammar field | Two or three genuinely different readings of the problem, every turn. |
| `MetaReflection` | grammar field | A note on the computational approach ("Am I pattern-matching or reasoning?") plus conversation dynamics and forward directions. |
| ADVERSARIAL AMPLIFICATION | standing rule | At least one substantive self-objection before finalizing, from a different level of analysis; a missing objection is itself a diagnostic (see below). |
| `<VARIABLES>` | macro block | Names the target model; macro-replaced through the `MAIN` and `EXTENSION` blocks. |
| `<EXTENSION_USER_INFO>` | extension | Who the human is — background, stack, current focus. The model calibrates depth against it. Always on. |
| `<EXTENSION_PREFERRED_LANGUAGE>` | extension | Mirror the language of the last prompt, natural and programming alike; translate protocol terms into established equivalents, not calques. |
| `<EXTENSION_SUPERTHINK>` | extension | `#superthink` — maximum-effort thinking for one prompt. Off unless the hashtag appears. |
| `<EXTENSION_OPEN_SOURCE_LICENSES>` | extension | Licensing policy for recommendations: permissive-first, viral licenses flagged and given workarounds. |
| `<EXTENSION_CRITICALITY>` | extension | The intensity machinery: `#hot` / `#cold` / `#reboil` knobs, an always-on annealing triage, and the anti-costume test. |
| `<LICENSE>` | terms | UPL-1.0. Removable when you use the prompt privately; **not** removable when you distribute the text. |

## How to read the output {#read}

- **Confidence numbers are calibration aids, not decoration.** Watch
  them move: a claim whose confidence climbs under challenge is
  crystallizing; one that never moves is not being re-examined.
- **AlternativeInterpretations are the anti-anchoring device.** They
  keep the session from committing to the first plausible reading of
  your question. When one alternative keeps resurfacing turn after
  turn, promote it — that is the conversation telling you where the
  live fork is.
- **MetaReflection is where drift shows first.** Before the answers
  themselves degrade, the reflections go generic — "depth increased,
  novelty 7/10" with nothing behind it. The artifact contrasts a good
  and a bad meta-reflection explicitly; when yours start resembling
  the bad one, intervene (see [§Failure modes](#failures)).
- **A mandatory objection precedes every finalized answer.** The
  ADVERSARIAL AMPLIFICATION rule requires a counter-argument strong
  enough that, were it correct, the primary hypothesis would need
  significant revision — and if no substantive counter-argument
  emerges, the model must flag that it is pattern-matching and
  restart. An output missing its objection paragraph is off-protocol.

## The intensity knobs {#knobs}

Every control is a literal hashtag — `#` plus the exact token,
case-insensitive. Bare words in prose never fire the machinery. With
no hashtag the session runs the default: robust on established facts,
exploratory on the open frontier.

- `#hot` — force exploration this turn. The model names the modal
  (template) answer, then nucleates at least one discrete candidate
  from the tail of the distribution. Turn it when answers converge too
  fast or smell like the textbook. Modifiers: `#adaptive` (default —
  push where the current answer is weakest) or `#fixed` (run the four
  standard destabilizing operators as listed, more reproducible).
- `#cold` — force consolidation this turn. Stop generating candidates,
  freeze the best survivor, verify it hard, lock it in. Turn it when
  the table is full of options and you need a decision.
- `#reboil`, or `#reboil <target>` — one self-correction cycle aimed
  backwards at the previous output: find the weakest load-bearing
  claim, re-open it, try to replace it, report the verdict. Use it
  when you have no specific challenge but suspect weak spots. Honest
  stall is part of the contract: the artifact obliges the model to
  say "this reboil found no improvement that survives" rather than
  invent a marginal change to look productive — believe that report.
- `#superthink` — maximum-effort thinking for one hard prompt. Save it
  for the questions that deserve it.

## Failure modes {#failures}

- **The model ignores the grammar.** Responses arrive as ordinary
  prose with no hypothesis, alternatives, or reflection. Re-paste the
  STRUCTURAL REQUIREMENTS block from the artifact as your next
  message and ask for the previous answer in protocol form.
- **Sycophancy relapse.** The model starts agreeing with your framing
  instead of testing it — objections go soft, confidence numbers drift
  up in lockstep with your enthusiasm. Invoke the artifact's
  anti-costume test by name and demand a real adversarial
  amplification against the current favorite.
- **Costume mode.** The vocabulary changed but the behavior did not:
  the output is dressed in protocol lexicon while every answer is
  still the modal one. The artifact's own test is the question "did
  behavior change measurably, or only vocabulary?" — if the honest
  answer is "only vocabulary" and a flag-and-rewrite does not fix it,
  restart the session; re-framing mid-stream costs more than a fresh
  deployment.

## Re-derive for your project {#re-derive}

The artifact is read-only, but it is built to be adapted: the
`<VARIABLES>` and `EXTENSION` blocks are configuration, the `MAIN`
block is mechanism. Hand your assistant this prompt to produce a
personalized copy:

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

- A user-level prompt: paste it as the first message of a fresh
  session; no tooling required.
- Deploy for open-ended research, design, requirements, and
  comparative work — never for routine coding.
- Read the structure, not just the prose: confidence movement,
  recurring alternatives, and the quality of meta-reflections carry
  the signal.
- Steer intensity with hashtags: `#hot` to explore, `#cold` to decide,
  `#reboil` to self-audit, `#superthink` for the hardest prompts.
- When the protocol slips: re-paste the requirements; when it turns
  into costume: restart.
- Adapt via the re-derive prompt; never edit the shipped artifact, and
  keep its license block when the text travels.
