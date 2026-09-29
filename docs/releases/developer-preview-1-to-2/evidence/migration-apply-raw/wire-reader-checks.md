# D1 generated reader candidate proof

Writer stopped; root reviews/accepts/commits. Only new integration target crates/vibe-wire/tests/migration_application_wire_corpus.rs retains candidate changes (297lines). Existing migration_wire_corpus.rs original321-line body is restored; its C2 cases and shared wire_support helpers are unchanged. No generated/schema/corpus/generator/core/adapter changes.

The initial append reached609lines, so the allowed split was needed. I first chose application_wire_corpus.rs without checking its existing ownership; a split assertion failed and command sequencing still ran rustfmt on that pre-existing90-line file. Root's narrow comparison confirmed NO byte delta. Root then explicitly authorized migration_application_wire_corpus.rs, and I moved only the seven new cases/helpers. No Cargo/test failure occurred in this atom; the failed assertion/formatter mistake is recorded rather than hidden.

## Bound target and actual execution

Before compilation the selector binding was updated to the new actual target. Each of the following ran separately with CARGO_TARGET_DIR=/home/olegchir/git/v/vibevm-next/target, CARGO_BUILD_JOBS=4:

cargo test --locked --offline -p vibe-wire --test migration_application_wire_corpus <case> -- --exact

- operation_request_valid_roundtrips_preserve_explicit_intent_and_authority: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/1-operation_request_valid_roundtrips_preserve_explicit_intent_and_authority.log.
- operation_report_valid_roundtrips_preserve_nullable_identity_evidence_effects_and_guarantees: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/2-operation_report_valid_roundtrips_preserve_nullable_identity_evidence_effects_and_guarantees.log.
- application_journal_valid_roundtrips_preserve_checkpoint_fields_and_mode: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/3-application_journal_valid_roundtrips_preserve_checkpoint_fields_and_mode.log.
- operation_request_structural_invalids_refuse: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/4-operation_request_structural_invalids_refuse.log.
- operation_report_structural_invalids_refuse_nested_guarantees_effects_codes: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/5-operation_report_structural_invalids_refuse_nested_guarantees_effects_codes.log.
- application_journal_structural_invalids_refuse: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/6-application_journal_structural_invalids_refuse.log.
- application_domain_invalids_remain_owner_validation_inputs: exit0,1passed/0failed/6filtered; raw stdout+stderr in /tmp/vibevm-next-M13-D1-wire-checks/7-application_domain_invalids_remain_owner_validation_inputs.log.

All seven intended nonzero cases passed; no old C2 cases, test listing, broad target, wire-diff, mutation testing, provider/user apply, Git operation or publication ran. Scoped rustfmt affected only the final new owner after the documented no-delta accident.

## Observed structural obligations

The actual canonical emitted report and journal carry required_nullable::deserialize attributes. Exact readers reject missing report operation/operation_id, all five nullable evidence fields, error, nested barrier binding and journal unix_mode, while explicit null values roundtrip. Request apply/inspect/recover payloads preserve exact approved_request_json and independent fixed C1 domain IDs; journal preserves all immutable fields, raw versus framed identities, decimal strings, mode/null and five closed phases. These synthetic/golden fixtures are not physical ownership/durability evidence.

Structural refusals cover unknown roots/nested authorities, guarantee/effect/error/warning vocabularies, wrong primitives and missing nullable properties. Additional in-memory rows exercise strict nested guarantees, effects, application error, warning and residue objects. No generic parser/codec/domain engine was added: all decoding uses emitted roots and existing reader support.

Six semantic-invalid fixture values deliberately decode/roundtrip as shape-valid: request schema2/wrong-before-content-id, report schema2/external-writer exclusion true, journal protocol2/noncanonical length. The seventh case observes that boundary only. Every such input MUST later be refused by the operation owner before mutation/emission; this does not accept the value or prove D1 apply, inspection, recovery, hash recipes, paths, phase transitions, concurrency/durability or native Linux/macOS/Windows behavior. Source reqr3 is bound by each new verifier; root retains whole-task/platform/account acceptance.
