# D1 schema authoring candidate

Writer stopped; source-independent review/root acceptance pending. No Cargo, generator, generated file, behavior/test Rust, existing C2 schema/vocabulary, boot/instruction, Git, provider, apply or public action changed.

## Exact inputs authored

- schemas/migration/e1/operation_request.jtd.json: discriminator operation apply/inspect/recover, schema:uint32 on each strict branch, operation_id and local authority. Apply carries approved_request_json string plus before_content_id/after_content_id/diff_id; recover carries expected_intent_id and complete/restore action. No unchecked after_text, C2 payload extraction or second planning codec.
- schemas/migration/e1/operation_report.jtd.json: schema, required-nullable operation/operation_id, closed outcome/target_state vocabularies; effects/warnings/residue arrays with x-empty=emit; closed local guarantees and application_error. Evidence IDs/error and barrier-binding are required+nullable: explicit null is unavailable and omitted fields must refuse, unlike C2 optional nonnullable choices. Explicit wrong primitive values remain structural errors. Error codes and warning actions match the source exactly.
- schemas/migration/e1/application_journal.jtd.json: exact source-law immutable fields, required nullable uint32 unix_mode, canonical-decimal string lengths and the five closed phases. Whole checkpoint shape only, no append parser or embedded unchecked plan. Raw hashes remain distinct from three domain identities.
- formats/REGISTRY.toml: append only migration-operation-request/report/application-journal epoch1 rows, exact schema/corpus paths, many parsers, explicit deny, sunsetnone; request/report recoverabletrue and journalfalse. Prior registry bytes remain an unchanged prefix with SHA256 ddf880d3271fd55f65258f70a44983e4a720427f9dbe7749187a367b413460e1.

Root metadata in each schema implements PROP044#MANIFEST-MIGRATION-APPLICATION-WIRE r3, with vibe_wire::migration::e1::<root> rustOptions package. All enums are declared closed locally; no generator requirement discovered that needs another vocabulary extraction. Existing C2 inputs/types remain untouched. Root will inspect actual generation before accepting emitted APIs; no generated type names are asserted as observed here.

## Finite fixtures and proof limits

Thirty synthetic/golden fixture files are authored under exactly the three new corpus roots. Request9:3valid apply/inspect/recover;4structural-invalid root/authority/operation/schema-type;2semantic-invalid schema2 and replay-ID mismatch. Report10:2valid completed/preflight-refused with explicit-null operation identity/evidence;6structural-invalid missing-required-operation/guarantee/effect/error/warning/primitive;2semantic-invalid schema2 and false protocol claim of excluded external writers. Journal11:3valid prepared/restored/explicit-null-mode;6structural-invalid root/authority/phase/mode/length primitives and missing-unix-mode;2semantic-invalid protocol2/noncanonical decimal.

The retained planning request supplies the fixed before text and independently fixed C1 framed planner IDs. Synthetic project/store spellings and opaque bindings are not live roots or grants. Raw SHA256, byte lengths and intent IDs are calculated with standard Python hashlib using the exact source-defined byte-length framing; no production hashing helper ran. Explicit-null-mode intent uses literal absent in the mode frame. Fixtures do not establish physical identity, successful publication, crash recovery, durability, platform behavior or real historical-file rehearsal.

Correction from additional exact owning source: xtask/src/codegen/optional_shapes/tests/required_nullable.rs demonstrates the existing required-nullable postpass. a_required_nullable_member_lifts_the_box_and_becomes_strict expects deserialize_with=crate::behaviour::required_nullable::deserialize; flat_some_null_none_and_missing_refusal_are_serde_exact demonstrates explicit null accepted and missing rejected. I previously reasoned from unprocessed serde Option and missed this existing pass. missing-unix-mode is now structural-invalid and proposed reader-refusal coverage, not assumed acceptance/owner-presence debt. This is a source-backed expectation pending actual application-journal generated output/attribute and selected reader proof; no test or generation ran here. Schema/protocol equality1, lowercase hashes, authority/path/binding/identity semantics, canonical decimal spelling, replay identities, phase transitions and effect/guarantee coherence remain operation-owner validation. Invalid cases are separated accordingly. Source law remains spec/plan; these files do not implement it.

## Proposed selected reader binding, unrun

Existing exact owner crates/vibe-wire/tests/migration_wire_corpus.rs was read only for naming. It already uses generated roots, wire_support round_trip/read_json and explicit structural rejects; its existing seven-plus cases remain unchanged. After root's canonical generated checkpoint, propose same integration target migration_wire_corpus with new exact cases:

1. operation_request_valid_roundtrips_preserve_explicit_intent_and_authority.
2. operation_report_valid_roundtrips_preserve_nullable_identity_evidence_effects_and_guarantees.
3. application_journal_valid_roundtrips_preserve_checkpoint_fields_and_mode.
4. operation_request_structural_invalids_refuse.
5. operation_report_structural_invalids_refuse_nested_guarantees_effects_codes.
6. application_journal_structural_invalids_refuse.
7. application_domain_invalids_remain_owner_validation_inputs (structural acceptance observation only, no fake semantic refusal).

Bind --locked --offline -p vibe-wire --test migration_wire_corpus with those exact selectors or root-reviewed new small family after generated API confirmation; do not repeat old C2 cases merely to aggregate results. No case was written or executed in this atom. Required D1 owner tests later must actually refuse semantic inputs before writes; new journal reader case6 must prove missing required nullable mode refusal after emission.

## Authored corpus paths

- formats/corpora/migration-operation-request/e1/semantic-invalid/unsupported-schema.json
- formats/corpora/migration-operation-request/e1/semantic-invalid/wrong-before-content-id.json
- formats/corpora/migration-operation-request/e1/structural-invalid/schema-string.json
- formats/corpora/migration-operation-request/e1/structural-invalid/unknown-authority.json
- formats/corpora/migration-operation-request/e1/structural-invalid/unknown-operation.json
- formats/corpora/migration-operation-request/e1/structural-invalid/unknown-root.json
- formats/corpora/migration-operation-request/e1/valid/apply.json
- formats/corpora/migration-operation-request/e1/valid/inspect.json
- formats/corpora/migration-operation-request/e1/valid/recover-complete.json
- formats/corpora/migration-operation-report/e1/semantic-invalid/claims-external-writers-excluded.json
- formats/corpora/migration-operation-report/e1/semantic-invalid/wrong-schema.json
- formats/corpora/migration-operation-report/e1/structural-invalid/barrier-binding-number.json
- formats/corpora/migration-operation-report/e1/structural-invalid/unknown-effect.json
- formats/corpora/migration-operation-report/e1/structural-invalid/unknown-error-code.json
- formats/corpora/migration-operation-report/e1/structural-invalid/unknown-guarantee.json
- formats/corpora/migration-operation-report/e1/structural-invalid/unknown-root.json
- formats/corpora/migration-operation-report/e1/structural-invalid/unknown-warning.json
- formats/corpora/migration-operation-report/e1/valid/completed.json
- formats/corpora/migration-operation-report/e1/valid/refused-unavailable.json
- formats/corpora/migration-application-journal/e1/structural-invalid/missing-unix-mode.json
- formats/corpora/migration-application-journal/e1/semantic-invalid/noncanonical-length.json
- formats/corpora/migration-application-journal/e1/semantic-invalid/unsupported-protocol.json
- formats/corpora/migration-application-journal/e1/structural-invalid/before-length-number.json
- formats/corpora/migration-application-journal/e1/structural-invalid/unix-mode-string.json
- formats/corpora/migration-application-journal/e1/structural-invalid/unknown-authority.json
- formats/corpora/migration-application-journal/e1/structural-invalid/unknown-phase.json
- formats/corpora/migration-application-journal/e1/structural-invalid/unknown-root.json
- formats/corpora/migration-application-journal/e1/valid/prepared-null-mode.json
- formats/corpora/migration-application-journal/e1/valid/prepared.json
- formats/corpora/migration-application-journal/e1/valid/restored.json

Local source-data review: all3schema/30fixture JSON files parse, local definition references resolve, and3registry rows match exact source-bound properties. This is syntax/input review only, not canonical schema validation, generated-reader execution or domain acceptance. Valid/ labels denote structural examples where physical bindings/effects are synthetic; native/domain fixtures remain separately bound by root. No further fixtures were added. Writer stopped.

Bounded correction only: journal metadata and the moved missing-mode fixture classification/report changed. Corpus count remains30. Existing report evidence semantics, C2 omission/null semantics, generator and behavior sources remain unchanged. Historical r2 journal schema SHA256 71af0b4f11b26bf38505561c85188f4d998f3f21d8a76c683e2614cc6477fc29. Writer stopped.

Root-directed source correction: report operation and operation_id are REQUIRED NULLABLE, distinct from optional nullable evidence. The refused-unavailable valid fixture now represents preflight with both null, unestablished guarantees and no effects. Missing-required-operation replaces redundant report unknown-root; the existing required-nullable generation pass is expected to refuse omission, pending actual output/reader proof. Request/journal operation_id remain nonnullable. Corpus stays30; no emission/tests. Historical r2 report schema SHA256 5582e2937df975e0b934d7f84c2e67d47d840fc357dcf4912ceea7832d9a4410. Writer stopped.

Historical pre-emission source revision binding: all3schema metadata.spec.r values were2 for the required-nullable application report identity/evidence amendment before first emission. Product1.1.0, wire epoch1 and journal protocol revision1 are unchanged. No further field/fixture/generator changes. Historical r2 schema SHA256 values: operation_request=12714b6c84151b7f1dff5f890c4420d5d8723de9ee3cef05dbe33c444c4934be, operation_report=523ffcb17e5173f909d98020eb5432594b5e683516376d505c7d6c048d3d1cd9, application_journal=8ba8b4a7a17b0ece9248aa4fad75350d86a2e39ef0a629a6d86b952c970e81c6. Writer stopped.

## Actual emission failure and supported correction

Root's actual canonical codegen attempt failed exit1, retained in /tmp/vibevm-next-M13-D1-codegen.log: optional_shapes refused optionalProperties.intent_id plus nullable because its pinned pass has no absent-and-null rule. This was an actual generator refusal, not a tested reader result. Failed staging was retained by root at /tmp/vibevm-next-M13-D1-failed-generated.new-595796; I did not inspect or edit that tree.

Source reqr3 now requires explicit-null unavailable evidence. All five report evidence fields and guarantees.barrier_binding are moved to required properties with nullable:true; optional x-default metadata is dropped. Operation/operation_id remain required nullable. All base-derived report fixtures have explicit null when evidence is unavailable, so structural negatives retain their intended faults; missing-required-operation deliberately stays missing. Corpus remains30. The existing required-nullable pass supplies the source-backed emission expectation; root must rerun actual generation and inspect attributes/readers. No generator/C2/behavior edits or execution occurred here. All3schema spec.r=3; product1.1, wire1 and journal1 unchanged. Final hashes: operation_request=29179c9c465176b56026a1fda9aa3efb678bcbc7dafa2b91b65420d5cf990096, operation_report=63a3b2371fb69849fa11dde792b88de147b63fa9df4bf75ffac74571eb71c9de, application_journal=1723b455c7b8e7dd7fd7c081a5c1e8e1e11b00b46744a59473ec04d41706c007. Writer stopped.
