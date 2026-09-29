# C2 generated migration reader corpus

Writer stopped; only three migration corpus roots and crates/vibe-wire/tests/migration_wire_corpus.rs were written. The owner is321 lines after exact-file rustfmt. No generated/schema/behavior/adapter/export/dependency/Git/provider/public/root-state changes were made. No whole-wire/full-epoch/codegen/wire-diff gate ran.

## Actual selected evidence

Both commands used CARGO_BUILD_JOBS=4 and CARGO_TARGET_DIR=/home/olegchir/git/v/vibevm-next/target.

* cargo test --locked --offline -p vibe-wire --test migration_wire_corpus: **exit0;11passed,0failed,0filtered**. Raw redirected log /tmp/vibevm-next-M13-C2-wire-corpus-tests.log. No failed compile/test or rerun occurred.
* cargo test --locked --offline -p vibe-wire --test format_registry_completeness format_id_completeness -- --exact: **exit0;1passed,0failed,1filtered**. Raw redirected log /tmp/vibevm-next-M13-C2-formatid-test.log. This proves registry↔generated FormatId membership, including the three new IDs, not semantic migration behavior.

The pre-existing vibe-cli unused manifest build-key warning appeared in both logs. The full new target was bound before compile as11 cases; no list/global inventory or production mutation was used.

## Authored frames and observations

**46 fixtures:**10valid (3request/4report/3preflight error),26structural-invalid (4request/17report/5error),3presence-invalid optional-null requests,7semantic-invalid frames for later adapter checks. Folders keep these distinctions explicit.

Typed round_trip uses the existing wire_support helper: generated read/serialize preserves every authored JSON value and exact embedded source string, including UTF8 café/CRLF, missing selections, named choice order, source/target profiles, epoch markers and identities. It does not claim raw JSON formatting byte identity. Requests cover explicit legacy, absent choices and current epoch1. Reports cover planned/no_op/needs_input/refused. Standalone error frames use only the three proper preflight codes. The shared error is the same actual Rust type in both roots, and all nine closed code spellings roundtrip.

The supplied independent C1 golden vector is retained exactly for the planned synthetic package input: offsets86/96 and all three fixed identities are asserted. No-op uses one fixed legitimate synthetic epoch1 source; its content/diff IDs were authored independently with standard Python SHA256 and the existing source-defined u64be frame recipe, without production helpers. This reader target proves carriage, not a new core semantic oracle or real-file historical corpus. The emitted actual epoch1 constructor is Epoch::Epoch(Box<EpochEpoch>); no guessed API name was used.

All26 structural negatives actually refuse with the intended diagnostic category:8 wrong primitive types,11 unknown members and7 unknown tags/closed enum codes. They cover request/report/error roots, outcome branches, Plan/TokenEdit/Question, both empty epoch payload markers, shared error in both roots, and numeric offsets rather than strings. Wrong-branch payload and unknown outcome tag are distinct cases. Empty markers and shared errors demonstrably retain strict reader policy.

The three explicit null cases **decode directly to Option::None and lose presence on serialization**. The test names/documents this limitation; it is not generated strict rejection and does not permit null in JTD. P044's separately owned request-presence guard must later refuse them. The seven schema2/stage-code/canonical-offset/source-bound/profile-epoch invalid frames are deliberately accepted structurally and retained for the pure adapter. They do not pass semantic validation here. Thus successful reader cases establish no callable CLI/MCP path, implicit state-free startup, semantic result acceptance or task/DP2 closure.

## Byte/provenance references

Test owner SHA256:3175128e513b77157efe7e50bdce5a7d5684707606a85b284ec17835379406cb.
Independent supplied golden JSON SHA256:4eb723e5497bb7c5cfd3bd2a1be9b23c84d0940e13c0de7d6f208db49e75145e.
The sorted46-row fixture(path,SHA256,byte-length) compact JSON inventory identity is7e60eec19fe0c5b711821f495c03b80067c0c0993b12ab62449e78a1c381d49d.

Read-scope deviation was disclosed promptly: my initial shared-error selection printed about40following unrelated lines from shared/mod.rs; they were not used. No additional reads of those types, writes or gates resulted. Root retains type/fixture review, commits, pure-adapter/presence/CLI/MCP conformance and all acceptance.
