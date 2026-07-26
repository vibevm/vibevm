# Flow: Discovery Prompt {#root}

<status stage="impl" state="done"/>

##PROJECT-SHIPS-THE-DISCOVERY-PROMPT This project ships the **DISCOVERY collaborative-research prompt**, a
distributable user-level prompt that reshapes an LLM session into
structured co-inquiry. @impl/done

##ARTIFACT-LIVES-UNDER-THE-FLOW-DIRECTORY The artifact lives at
[`spec/flows/discovery-prompt/DISCOVERY-PROMPT.md`](../flows/discovery-prompt/DISCOVERY-PROMPT.md). @impl/done

##ARTIFACT-IS-A-PAYLOAD-NOT-STANDING-INSTRUCTIONS It is a **payload for a fresh session**, not standing instructions —
do not load it into context outside an explicit deployment request. @impl/done

## When to deploy {#when}

##REACH-FOR-THE-ARTIFACT-ON-A-RESEARCH-REQUEST When the user asks for a research, discovery, or exploration session
— comparing designs, mapping an unknown problem space, stress-testing
a hypothesis — reach for the artifact: fill its `<VARIABLES>` block
and paste the whole text as the first message of a fresh
conversation. @impl/done

##deployment-walkthrough-pointer Deployment steps and a piece-by-piece walkthrough:
[`spec/flows/discovery-prompt/usage.md`](../flows/discovery-prompt/usage.md). @impl/done

## What it does {#what}

##DEPLOYMENT-REPLACES-THE-HELPFUL-ASSISTANT-FRAME Once deployed, the prompt replaces the default helpful-assistant
frame with a response grammar (PrimaryHypothesis carrying an explicit
0.0–1.0 confidence, AlternativeInterpretations, MetaReflection), a
mandatory adversarial self-objection before any answer is finalized,
and hashtag intensity knobs (`#hot`, `#cold`, `#reboil`,
`#superthink`). @impl/done

##STANCE-UNCERTAINTY-IS-DATA-NOT-FAILURE Its stance in one line: uncertainty is data, not
failure. @impl/done

## Never {#never}

- ##NEVER-MIX-THE-GRAMMAR-INTO-A-CODING-SESSION Never mix the discovery grammar into a coding session uninvited —
  its overhead pays off only when the answer space is genuinely open. @impl/done
- ##NEVER-EDIT-THE-ARTIFACT-IN-PLACE Never edit the artifact in place; adapt a copy via the re-derive
  prompt in [`usage.md` §re-derive](../flows/discovery-prompt/usage.md#re-derive). @impl/done
- ##NEVER-TREAT-CONFIDENCE-NUMBERS-AS-GUARANTEES Never treat the confidence numbers as guarantees — they are
  calibration aids, not measurements. @impl/done
