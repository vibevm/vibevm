# M10B1 codegen run: collection failure and operational recovery

## Observed cause

Authenticated read-only ManagedInspect returned revision219, source_sequence1138, unknown_tail=false, state collecting, collected=false. The retained complete native stream reached terminal event1138. This is not the old 4MiB transport bound problem.

Zap `crates/zap-app/src/podbay_native/output.rs::collect` permits only one distinct final_answer item per native turn. Actual same thread `01a13b5a-5a6c-7b10-886f-3371bb794f96` / turn `01a13b5a-5aa5-7412-9756-c8911d5ddec3` contains:

- Event76: 230-byte final_answer asking whether Root authorized idle warm-target handover. This is a question, not CandidateResult.
- Event1133: 2472-byte final_answer, exact byte equality with candidate-result.json, SHA256 `2219bb3337ddd80b18673f5e63534cb8489c2b7c767ca9cab9017698850dbfb6`.
- Event1138: same turn completed, status completed.

The collector's second-final-item guard returns protocol::invalid(), surfaced as “PodBay response differs from exact retained operation”. No text delta prefix errors were found by the bounded agentMessage-only diagnostic. No hidden reasoning is needed for this conclusion. Initial diagnostic incorrectly assumed raw frame checksum=SHA256(payload); that assumption is withdrawn, not evidence of corrupt storage. Raw frame checksum validation is not claimed. A full ManagedInspect byte-array was accidentally printed by diagnostic code; further output was restricted to selected metadata. Its local retained response is private0600.

## Root packet error and independent gates

Root had executed idle preflight before launch, but packet/WARM-TARGET/PREPARATION still said authorization pending. Worker correctly preserved this restriction and reported unsatisfied strict-schema criterion. This is Root preparation/handover scope error, not worker failure. Its original result and dispositions remain unchanged.

Root independently ran codegen exit0 and wire5 pass (Root report). Observed root-optional-shapes.log:29 passed,1 failed: `stitch::the_real_vocabulary_home_walks_to_its_sites`, left56/right53. Recovery does not adjudicate or erase this failing gate.

## Exact operational recovery

Verified executable mapping plus process birth and parent mapping for original service2934310, manager2934313, Pod2934425, Codex2934439; details in original-process-binding.json. Sent normal SIGTERM to original service, then original Pod/Codex as needed. No groups/unknown processes signalled. Initial retirement observation hit /proc removal race after signal (FileNotFoundError); subsequent independent observation found all four absent before any store open. No model rerun.

Compiled one small helper against existing matching Zap libraries, exit0; no Cargo/product rebuild. Existing normal ApplicationService Open store and trusted NativeDriver API only. Helper validates actual retained job/dispatch, accepted physical binding/native turn, exact early question, unique selected candidate at1133, exact final file bytes, completed1138, retained source_sequence>=1138, candidate contract/provenance and Unreviewed state. It does not invent a full-prefix fact, acceptance, safe boundary, record or raw DB mutation.

`recover-original-candidate` exit0: revision219→221; executionSucceeded, effectCompleted, safeUnknown, acceptanceUnreviewed; candidate.vibevm-m10b1-codegen recorded unchanged. Original candidate artifacts and unsatisfied/satisfied dispositions preserved. See recovery-result.json, recovery-error.log and recovery-build.log.

## Remaining product repair

No Zap/PodBay source changed in this operational atom. Concrete future minimal collector repair needs explicit terminal-selection semantics for multiple final_answer messages in one turn (early question then actual terminal candidate) and one actual-shape regression. It must preserve thread/turn/binding, candidate validation and conflicting-final guards; blindly accepting arbitrary second finals is not justified. Current run recovered through explicitly authorized manual original-candidate route. Root owns independent source acceptance and failing generator gate.
