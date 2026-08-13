# Flow: Discovery Prompt {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-SHIPS-THE-DISCOVERY-PROMPT This project ships the **DISCOVERY collaborative-research prompt**, a
distributable user-level prompt that reshapes an LLM session into
structured co-inquiry. @status:impl/done

@fact:ARTIFACT-LIVES-UNDER-THE-FLOW-DIRECTORY The artifact lives at
@spec://org.vibevm.world/discovery-prompt/flows/discovery-prompt/DISCOVERY-PROMPT#root. @status:impl/done

@fact:ARTIFACT-IS-A-PAYLOAD-NOT-STANDING-INSTRUCTIONS It is a **payload for a fresh session**, not standing instructions —
do not load it into context outside an explicit deployment request. @status:impl/done

## When to deploy {#when}

@fact:REACH-FOR-THE-ARTIFACT-ON-A-RESEARCH-REQUEST When the user asks for a research, discovery, or exploration session
— comparing designs, mapping an unknown problem space, stress-testing
a hypothesis — reach for the artifact: fill its `<VARIABLES>` block
and paste the whole text as the first message of a fresh
conversation. @status:impl/done

@fact:deployment-walkthrough-pointer Deployment steps and a piece-by-piece walkthrough:
@spec://org.vibevm.world/discovery-prompt/flows/discovery-prompt/usage#root. @status:impl/done

## What it does {#what}

@fact:DEPLOYMENT-REPLACES-THE-HELPFUL-ASSISTANT-FRAME Once deployed, the prompt replaces the default helpful-assistant
frame with a response grammar (PrimaryHypothesis carrying an explicit
0.0–1.0 confidence, AlternativeInterpretations, MetaReflection), a
mandatory adversarial self-objection before any answer is finalized,
and hashtag intensity knobs (`#hot`, `#cold`, `#reboil`,
`#superthink`). @status:impl/done

@fact:STANCE-UNCERTAINTY-IS-DATA-NOT-FAILURE Its stance in one line: uncertainty is data, not
failure. @status:impl/done

## Never {#never}

- @fact:NEVER-MIX-THE-GRAMMAR-INTO-A-CODING-SESSION Never mix the discovery grammar into a coding session uninvited —
  its overhead pays off only when the answer space is genuinely open. @status:impl/done
- @fact:NEVER-EDIT-THE-ARTIFACT-IN-PLACE Never edit the artifact in place; adapt a copy via the re-derive
  prompt in @spec://org.vibevm.world/discovery-prompt/flows/discovery-prompt/usage#re-derive. @status:impl/done
- @fact:NEVER-TREAT-CONFIDENCE-NUMBERS-AS-GUARANTEES Never treat the confidence numbers as guarantees — they are
  calibration aids, not measurements. @status:impl/done
