# M14A1 export namespace review closure

The sole export-namespace blocker is closed in the reviewed bytes. EnvironmentDefinition::new now validates tool_exports and service_exports separately, with distinct error fields, while retaining the common declared package-root/runtime-reference checks. This removes the unbound cross-kind namespace restriction without admitting same-kind duplicates. PROP-076#ENVIRONMENT-DEFINITION remains the governing desired-composition contract; no new runtime or physical-authority claim follows.

The existing definition_and_lock_reject_duplicate_or_foreign_bindings case calls export_assertions::check_export_namespaces. That helper accepts a tool and a service with identical names and valid references, then separately rejects duplicate tool and duplicate service names with the respective field diagnosis. It adds assertions to an existing case, not a ninth distinct case. The actual exact invocation exited 0: 1 passed, 0 failed, 540 filtered. I did not rerun it.

All eight current source SHA256 values match the new receipt. roles.rs and tests.rs changed, export_assertions.rs is new; lib.rs, compatibility.rs, identity.rs, mod.rs and platform.rs match both the old and new receipts. The new raw log hash and both preserved original receipt hashes also match. Thus the original eight-case evidence remains retained, with a focused exact rerun covering the changed constructor and assertions.

Changed/new SHA256:

- roles.rs: 0ebeca4ff938556db689552c375b56a8f82209835c9aa1358b62d65cacbe5792
- tests.rs: 9c01e27e60ccb68686ae1897fb0fc2b314058b7f564bd951aff35b1946a1d3a5
- export_assertions.rs: 81d9339ee1b706460bceed48cec093490addfdc03f989273a05faa8aa3860cb6

No remaining finding in this correction. No Cargo, tests, Git, product edits, boot or local-state operations were performed. This closes only the reviewed constructor policy gap; root retains task acceptance and later consumer/native/physical evidence obligations.
