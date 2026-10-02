# Registered fixed package action values

Three closed epoch-one formats are registered for exactly
`rust:vibe-lifecycle:package-static-file` under owner `rust:vibe-lifecycle`:
`machine-fixed-package-action-request`, `-reply` and `-error`. The request
permits only `submit {key}` and `inspect {key}`. It has no root, namespace,
scope, provider, target, destination or timestamp field; the trusted host and
lifecycle package owner supply those. Result variants retain Completed,
Uncertain with exact partial output/A2 observations, RecoveryRequired,
IntentReady, Retired and Absent. Error values retain nullable scope/run/view
and a closed effect-observation vocabulary.

The pinned `jtd-codegen 0.4.1` generated exact Rust readers and format IDs.
Two worker codegen runs produced the same 185-path content/hash set. Generated
`ByteCount` is `u64`; the selected wire tests roundtrip `u64::MAX` and reject
fractional, negative, overflowing, null and string byte counts. Structural
unknown/wrong/missing field probes are closed. Six semantic-gap corpus values
remain shape-valid intentionally: schema epoch 2, empty/oversized key and
oversized diagnostic must be rejected by the later owning adapter.

The focused `vibe-wire --test machine_fixed_package_action_contract` family
passed 7/7 in both the private worker target and the primary target. Its
corpora include 29 valid and 57 structural-invalid roots and 486 nested
unknown/missing-field probes. This is format and transport proof only. No
package descriptor, CLI/MCP/local callable adapter, second C1 reservation or
result journal is claimed; no full panel or mutation campaign was run.
