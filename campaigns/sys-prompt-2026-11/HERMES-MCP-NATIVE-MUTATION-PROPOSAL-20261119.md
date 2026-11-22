# Hermes MCP native mutation — bounded ownership proposal

2026-11-19. Existing native projection obligation. CURRENT SELECTED DECISION: native FileBackend renders a captured private stage; existing Rust PublicationBatch alone publishes final config with recheck and OwnedFileReceipt. The external-final-mutation proposal below is historical/rejected and MUST NOT be implemented. No operator config mutation is authorized by feasibility tests.

## Reuse

The selected installed Hermes runtime already owns config.yaml comment-preserving writes: `hermes_cli.config.save_config` → native atomic/comment-preserving writer, unreadable guard, config lock and native secret-template preservation. `hermes_cli.mcp_config._save_mcp_server` validates a server then updates only the named mcp_servers member through that writer; `_remove_mcp_server` is its native inverse. Do not implement a Rust YAML whole-document serializer or call the native CLI with key values on argv.

Existing Vibe package_registration owns per-config adjacent receipt, pending old/new payload, foreign-entry refusal, activation intent and cooperative PublicationBatch before/after authority. Its current publication authority is a Vibe-created safefs stage receipt; an external native writer cannot be described as such a stage merely by reading its after-image. Therefore native writer adoption needs an explicit owner contract, not an unreviewed special case in payload serialization.

## Proposed operation

1. Caller resolves selected Hermes installation and exact profile home (explicit injected path, no ambient guessed fallback). Admit runtime module identity and config parent no-follow; acquire existing Vibe config/receipt operation lock. Helper uses private request file containing only operation, server name, expected owned payload/fingerprint and desired public MCP payload; never full config or credentials.
2. Native helper takes native config lock and reads the owned server only. Return a canonical JSON representation of that key plus complete-config byte fingerprint, not the config body. Parent compares receipt-owned old/pending key and refuses unreceipted/foreign changes before retaining mutation intent.
3. Same native helper transaction validates unchanged input fingerprint under native lock, invokes native validated key writer/remove, captures after fingerprint and owned-key result before releasing its lock. Existing receipt records desired intent/pending and observed result; verification never turns arbitrary after-state into rollback authority.
4. Failure after native write reports pending/recovery-required with retained fingerprints. Do not replay a complete preimage over a changed file. Inverse removes/reinstates only exact receipt-owned key through the same native compare/key mutation operation; any foreign owned-key drift refuses and preserves operator edits. Preserve unknown fields/comments through native writer. No config values in diagnostics or reports.

## Exact remaining decision

The parent PublicationBatch currently models complete file publication and guarded file restore. A native-key transaction has different ownership: content fingerprint + native locked key compare/mutation, not Vibe stage receipt. Decide whether the existing registration receipt gets one explicit native-key operation branch (same receipt/state, not a new journal), with batch failures retaining pending instead of whole-file inverse; or whether native writer can render safely to a Vibe-owned stage while preserving comments, then existing safefs publisher remains sole writer. Rendering via existing native YAML document owner into an admitted stage would be simpler for rollback authority if the native API exposes that exact operation without copying its serializer.

## Synthetic stage feasibility result

Task `b4ii3icjw` exit0 uses the actual pinned native `FileBackend.read_path/write_path(stage, Changes(document=...))` against an invocation-owned stage inside synthetic Hermes home. Original config is unchanged; rendered stage retains operator/model/foreign-server comments, foreign values and YAML anchor/alias spelling, and adds only the desired MCP key semantically. Private `control/hermes-yaml-stage-proof.py` / `.log` under `/data/git/vibevm-native-carrier-20261119/`. No operator config, key, model/network call or product publication. Stage output SHA256 `18949162abde0defa15d0d6106a2ed7b35cc407a1656359f025aa76988468357`. This establishes a native stage-render path for this synthetic document, not arbitrary YAML losslessness or existing Vibe transaction integration. Prefer this path over external final-file native mutation if the architect admits its exact stage/home/receipt contract.

## Finite proof

Synthetic profile only: create/repeat/remove owned entry; foreign server/config values and comments retained; native invalid entry refuses; malformed/unreadable YAML and anchors/tags preserve/refuse per native owner; wrong profile/unsafe path refusal; before/after drift no overwrite; failure after write stays pending; recovery touches owned key only. No operator auth/config read, provider call, hooks, service stop or persistent profile rewrite beyond explicitly selected owned MCP key.
