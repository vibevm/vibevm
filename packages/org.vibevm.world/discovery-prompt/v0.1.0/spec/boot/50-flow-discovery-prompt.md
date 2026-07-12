# Flow: Discovery Prompt {#root}

This project ships the **DISCOVERY collaborative-research prompt**, a
distributable user-level prompt that reshapes an LLM session into
structured co-inquiry. The artifact lives at
[`spec/flows/discovery-prompt/DISCOVERY-PROMPT.md`](../flows/discovery-prompt/DISCOVERY-PROMPT.md).
It is a **payload for a fresh session**, not standing instructions —
do not load it into context outside an explicit deployment request.

## When to deploy {#when}

When the user asks for a research, discovery, or exploration session
— comparing designs, mapping an unknown problem space, stress-testing
a hypothesis — reach for the artifact: fill its `<VARIABLES>` block
and paste the whole text as the first message of a fresh
conversation. Deployment steps and a piece-by-piece walkthrough:
[`spec/flows/discovery-prompt/usage.md`](../flows/discovery-prompt/usage.md).

## What it does {#what}

Once deployed, the prompt replaces the default helpful-assistant
frame with a response grammar (PrimaryHypothesis carrying an explicit
0.0–1.0 confidence, AlternativeInterpretations, MetaReflection), a
mandatory adversarial self-objection before any answer is finalized,
and hashtag intensity knobs (`#hot`, `#cold`, `#reboil`,
`#superthink`). Its stance in one line: uncertainty is data, not
failure.

## Never {#never}

- Never mix the discovery grammar into a coding session uninvited —
  its overhead pays off only when the answer space is genuinely open.
- Never edit the artifact in place; adapt a copy via the re-derive
  prompt in [`usage.md` §re-derive](../flows/discovery-prompt/usage.md#re-derive).
- Never treat the confidence numbers as guarantees — they are
  calibration aids, not measurements.
