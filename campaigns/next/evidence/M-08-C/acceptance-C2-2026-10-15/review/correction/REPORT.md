# M-08-C.2 correction review

**Bounded clear. Previous P2 resolved; no new P1/P2 found in the correction.** This supplements the immutable original full review. Root alone accepts/commits/counts. No source edits, build/tests, process actions or acceptance were performed by this reviewer. Root reports normal candidate collection1343 with unknown_tail=false/failed=false; source stability is independently checked and does not imply that all managed processes have exited.

## Correction

Only three of the original13 candidate files differ: generic/plan.rs, generic/apply.rs and generic/tests.rs. The other10 match the prior reviewed hashes. The entire correction delta was read and compared against the original patch, retaining the original full-contract review for unchanged bodies.

GenericResource now contributes a component-normalized absolute destination lock independently of symbolic scope/root partition, plus the original incumbent User home:relative or Project absolute spelling. GenericPlan expands these through the same DeployState lock owner. Its existing path identity normalization, sorted hashed lock names and deduplication provide one total acquisition order. No second lease engine, reversed ordering or receipt-check substitute was introduced. Retaining compatibility keys preserves participation with incumbent providers; the common absolute key fixes the two-generic-host scope mismatch. Normalized root spelling in structured ownership removes the spurious collision for accepted trailing-dot roots without folding literal JSON/TOML member segments.

The new case uses two valid hosts, shared settings, different deployment IDs, opposite scopes in both directions and outer/nested root partitioning including a trailing-dot spelling. It takes the incumbent key alone, observes host A immediately before acquisition and verifies that A cannot enter; after releasing it, A holds its actual critical section while B reports pre-acquisition and cannot enter. The document remains unchanged while A is held. A publishes; B subsequently enters and returns an Infrastructure stale-ownership refusal. Explicit replan/reapproval succeeds and preserves both new members plus the foreign value. This exercises actual locking and stale rejection, not merely equality of derived strings. apply.rs changes only cfg(test) synchronization hooks.

These checks resolve the exact original counterexample under the supported retained, no-follow root model. They do not claim arbitrary mount aliases, hostile ABA/content CAS or cross-platform runtime commissioning. Case/Unicode comparison still belongs to the unchanged incumbent owner. No additional frontend/C3/platform requirement is introduced.

## Preserved full C2 boundary

Selected/enabled source admission, complete protected categories, immutable exact grants, current principal/policy checks, four closed shapes, literal members, foreign preservation, whole-selection conflict refusal and truthful publication boundaries are unchanged. Production apply and shared state/ownership/transaction owners are byte-preserved. Original grant-free refusal, scalar/duplicate JSON, aliases, locked drift, landed-failure and launch-content cases all run again. The two updated lock-count assertions reflect compatibility keys; their ownership/publication assertions remain intact. C3 migration/inverse/recovery and upper CLI/MCP remain separate.

## Evidence

- All13 candidate hashes match before/after review; exactly3 changed versus the prior candidate.
- Final V1 log records13 passed,0failed,0ignored. All52 before/after consumed-subject hashes agree and match current files. This roster is explicit, not a claim to hash every transitive compiler input.
- V3/V4/V5 reuse4 prior distinct cases with matching selected-owner hashes; they are not fresh runs. Current evidence is13 freshly executed distinct cases +4 reused cases, not one17-case run. V2 remains not applicable.
- Initial correction compile failure exit101/zero cases and subsequent11pass/2fail attempt are retained. Third and final runs each pass13; repeats are not additional cases.
- V6 scoped format and whitespace commands exit0 separately. V7 XML/preservation succeeds. The spec itself is unchanged from the original candidate; removing its original C2 addition reconstructs prior SHA256 cd594468e7b3cad5a5edf92855e01fcabe613426182f6fe7a60ecc03f47db2e5.
- All57 final census records match their hashes and contain equal before/after maps. They prove their finite observed file/membership assertions, not global no-syscall guarantees.

No COMMAND-LEDGER.json exists in this correction directory; exact group results, logs and command/subject records supply the requested evidence. Original reports, rejected self-report and P2 review remain retained. See sourcehashes.json for observed bindings. Full C2 is clear within this injected-root contract; this review performs no campaign acceptance.
