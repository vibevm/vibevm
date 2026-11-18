# Hermes carrier — corrected native instance seam

Genre: architectural refinement after two rejected prototypes, not a third blind retry.
Date2026-11-19. Full parity remains mandatory. Keep Pi as an independently bounded native implementation/proof so it cannot disappear behind Hermes work.

## Observed blockers

The first candidate replaced a module-global native constructor and recursed through its own patched function. The second assigned a bound Carrier method expecting a live-agent parameter to an AIAgent instance; the actual native caller supplies system_message, not the agent again. Its alternative proof path replaced the CLASS method and redirected HERMES_HOME/SOUL through a fixture pointer. These violate the accepted contract. Neither timed-out run is successful carrier evidence; retain candidates and failure receipts, do not execute the known recursive candidate.

## Revised minimal mechanism

Pinned native AIAgent._build_system_prompt is a lazy class-function forwarder; native callers invoke `agent._build_system_prompt(system_message)`. The native `build_system_prompt` delegates to `build_system_prompt_parts`, which places caller `system_message` first in the context tier, after native stable identity and before workspace/volatile content.

Use this actual native seam rather than reproducing native assembly:

1. On the actual selected main-session AIAgent instance, capture its ORIGINAL bound `_build_system_prompt` before attaching the carrier. Record whether a preexisting instance attribute exists.
2. Bind a wrapper only on that instance through `types.MethodType`. Its native signature is `(live_agent, system_message=None)`; verify that live_agent is the admitted instance and the runtime/file/session binding is current.
3. Read/admit the full project prefix from the invocation-owned file. Combine it with the caller's real system_message in the declared order, preserving None/empty semantics where relevant. Pass that combined argument to the captured original bound method. The native owner performs its actual assembly, joins, diagnostics and cache fields.
4. Do not call the newly shadowed method recursively. Do not patch native modules/classes, duplicate build_system_prompt's tail, manufacture volatile suffixes, substitute ephemeral_system_prompt or redirect HOME/SOUL. Ephemeral API-time content is a separate native path; replacing a missing caller argument with it can double-inject or reorder content.
5. Detach restores precisely the original instance attribute or removes only the owned shadow. Other native agent instances and subprocess launches retain the original class function and receive no carrier through inherited environment or ambient config.

This design must be verified against the exact admitted native signature and instance construction path. If the pinned upstream path cannot reach the admitted instance without forbidden global interception, return that specific constraint rather than inventing a class patch. A whole replacement agent loop is not a solution.

## First finite proof before broad matrix

Use the real native call site and original assembly with synthetic isolated resources: caller None, caller text, main instance, unrelated/worker instance and detach. Assert correct signature/binding, exact once-only insertion before volatile/workspace, original native base and resource preservation, unchanged independent instances and no transport call. Bound actual source/runtime/module hashes. Do not claim the absence of a worker path merely by constructing a plain mock object.

Once this seam works, extend the same candidate through the existing finite full-size/stability/fresh personal/resume/quiet-child native proof packet. Real cache savings require actual provider measurements and are not claimed by deterministic prompt construction. Full capability parity requires actual native CLI/SDK launch integration, not only a helper that concatenates strings.

## Pi separation

Pi uses its actual per-launch native extension/file path and pinned installed runtime. Give it a separate ready scope and proof target; it does not depend on Hermes internals. Both results must eventually integrate through VibeVM's existing harness owners. Do not create a shared generic carrier framework just to combine two upstream-specific seams.
