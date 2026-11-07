# A4 host activation guard review

Read-only source review, 2026-11-07. No tests, edits, Cargo, Git mutation or full-A4 acceptance. Host sources are changing under the writer; sibling JSON records observed file hashes/time. This reviews admission boundaries, not the host-sync algorithm.

## Concrete uncovered path — progress seal

`crates/vibe-cli/src/commands/progress.rs` dispatches Seal directly to `seal::seal_cmd` (line66 at observation). `progress/seal.rs` explicitly skips ground, reads each requested file with fs::read_to_string at line110 and calls parse_document at line114. No PackageSourceMode/host policy admission precedes this read or parse. The shared policy update therefore does not protect this verb.

Effect from source inspection: a file explicitly declared ordinary CommonMark/opaque in project.sources can still have literal Vibe markers interpreted by progress seal. If the existing campaign cache contains a file entry and matching verdicts, progress-core::seal::seal updates content_hash, processed_hash and verified_at; seal_cmd writes the cache when recorded>0. Otherwise the observed outcome can be Unjudged/Unobserved rather than precise classified-source refusal. This is a concrete code path, not an executed reproduction.

Minimal discriminator: valid host manifest with project.sources selecting `vibevm/vibespecs/common/RAW.md` as opaque/resource/explicit; RAW contains `@fact:X ... @status:impl/done` as literal text. Seed a normal campaign cache entry/verdict for X with an older processed hash, invoke the real `vibe progress seal <path>` with valid campaign selection, and require unsupported-policy refusal with cache and source bytes unchanged. A simpler unjudged fixture distinguishes parsing from admission but does not prove no cache write. Include ExplicitEmpty and a nested package/project target under a legacy outer root: a guard only on --path root misses the nearest selected file owner. Precise per-file pre-parse refusal is sufficient for this host slice; implementing classified seal is not required. Preserve legacy per-file results rather than introducing a global transaction.

## Covered boundaries observed in current source

| Consumer | Admission observed | Qualification |
|---|---|---|
| Shared policy | PolicyState::mode checks project.sources Some(empty)/Some(rows); source_rows exposes package or project rows | Grammar does not silently map explicit host intent to Legacy. Existing package accessors remain package-only. |
| Facts authored observation | observe_authored_source routes both explicit modes to declared_capture; host ExplicitEmpty returns zero facts/documents, nonempty returns unsupported host facts census | Honest intermediate refusal; no suffix scan. No-citation sync no-op remains separate and approved. |
| Resolver | observation::locate_in rejects Explicit and returns missing for ExplicitEmpty before legacy layout/suffix lookup | Installed bound reader remains separate accepted route; no host native lookup support claimed. |
| Documentation pages | read_package checks mode, empty yields no pages, explicit rejects before parsing | Doc host functionality is not newly enabled. |
| Documentation self citations | resolve_self rejects both explicit modes | No implicit XML page lookup. |
| Documentation external citations/agent | SpecSources::locate uses guarded FileResolver before path-based parse | Current explicit host reaches refusal; accepted captured-read migration remains future A4 adapter. |
| Borrowed examples | borrowed calls guarded read_package; borrowed_by checks mode of adapted root before read | ExplicitEmpty yields empty examples; explicit refuses. |
| Grounded progress commands | ground checks root Explicit and VocabularyByPackage checks nearest manifest for each admitted source; empty skips | Pair checks/parser/cache reuse occur after classification admission. Enumeration/config/cache reads are not source interpretation and do not imply a product-write promise. |
| Tree in-place directive collection | checks nearest selected owner policy before load_spec_text/Directives::parse | Empty skips; explicit refuses. Generated artifact entries are skipped separately. |
| Boot/WAL checks | boot_directory suppresses authored dialect/pair checks under empty, reports explicit unsupported; WAL check returns empty/reports explicit before reading WAL | Generated STATIC/INDEX structural checks retained. Absence of a boot directory can legitimately require no authored read. |
| Show effective | collect_sections checks both explicit modes before lock/boot/WAL/written-file reading | Honest unsupported boundary, even empty; no new source renderer claimed. |
| node_own_boot | explicit project rows return unsupported before boot dir read; empty rechecks and returns no own boot | Approved zero authored semantics. Package-node preexisting behavior is separately protected by install admission. |
| Direct boot input compilation | unbound_guards walks held ancestor manifests and rejects Explicit/ExplicitEmpty; Normal invokes refusal guard before fallback; Simple checks before legacy load_spec_text | New host mode reaches this shared guard, including standalone legacy entry construction. Generated inputs retain separate admitted path. |
| Early install admission | validate_declared_source_operation rejects nonempty project rows; validate_source_admission visits every workspace node; vibe-install planned admission invokes it before metadata/apply | Empty allowed for zero own boot. This does not claim zero coordination bootstrap activity or universal atomicity. |
| Source conversion | existing root-aware conversion policy switches on shared modes before semantic conversion | Host mode therefore refuses unsupported direct conversion rather than using suffix dispatch. |

No additional concrete silent suffix fallback was established in these inspected consumers. This is a bounded inventory, not proof that every command in the repository is classified. `verify_boot_graph`/check's best-effort suppression remains separately tracked A4 work; it does not parse declared host sources through a fallback and is not duplicated here. Skill raw projection is explicit byte copying, not a normative parser; its classification matrix remains separate A4 work.

## Acceptance implication

Close the progress-seal hole before claiming host grammar activation safely rejects all named unsupported progress paths. Add the small real command discriminator and retain ordinary legacy seal tests. Reconcile final hashes with the writer after its edits; this review provides no source freeze or new runtime receipt. Full A4 host/query/check/docs/skill completion remains root-owned.
