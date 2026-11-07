# A4 host cited-sync final source review

Read-only review, 2026-11-07. No code/spec edits, tests, Cargo or Git writes. No concrete blocking defect found in the inspected host grammar/policy/cited-sync boundary. This is qualified evidence for that slice, not full A4 acceptance. Writer is active; JSON hashes are observed snapshots to reconcile with final seal.

## Source result

ProjectSection.sources is Option<Vec<SourceDescriptor>> with default/skip-none and a Vec-deserializing nonnull helper. Absent, empty and nonempty remain distinct; source_exports stays unsupported. Existing descriptor selector/validation owns path/media/interpretation semantics. A group is not artificially required merely to store declarations; capture_cited_native requires a matching consumer coordinate when an address operation occurs. PolicyState::mode now recognizes project presence, source_rows shares package/project rows, and package-specific accessors are unchanged.

Both explicit and legacy load_spec_snapshot return a SpecSnapshot containing a mandatory PackagePolicyObservation. check rechecks before comparing; reconcile_snapshot computes mismatches, rechecks all observations and policy, then starts registry mutation. The legacy route's source enumeration/parsing law remains legacy, but a legacy→explicit manifest transition cannot publish an old snapshot. Empty needed-doc set returns before policy/root access, preserving the approved read-free no-op.

The declared loader verifies all origin=spec records name the captured host coordinate, captures every needed document before parsing, and builds all statuses before any write. A later malformed/raw/missing/ambiguous/directory/unsupported-Doc citation therefore cannot follow an earlier registry update. Capture and recheck use the same held source law, not per-file canonicalized authority. Source reads support shared regular inodes; symlink/special/directory failures remain typed refusals. This is finite cooperative validation, not whole-registry transaction/CAS or arbitrary-writer atomicity.

## Targeted canonical resolution

source_policy/host.rs first gathers explicit file rows whose incumbent canonical_doc_path matches the citation. For directory-default policy it inventories only the cited parent in the original/new-spec/legacy-spec spellings that the same canonical function recognizes; there is no recursive source census. It descends original UTF-8 path components with checked child opens, and child filenames pass declarant_path_component before joining. Literal backslashes are skipped as unrepresentable portable names, never rewritten into separators.

Each candidate is classified through shared select_source_rows before it can claim a native address. Raw same-stem neighbors are excluded from native alternatives; multiple selected native claimants refuse. Conflicting interpretation rows refuse rather than guessed precedence. Actual directory targets eventually fail the regular-source read; a directory-default-selected regular file succeeds. The chosen source retains parent identities, file identity/bytes, media, original relative spelling and document/coordinate. Recheck repeats relevant membership selection and capture, so newly appearing native aliases, path/parent replacements, bytes or manifest drift refuse. Unrelated raw additions do not poison a citation.

## Meaning and absence

Declared native media drives the existing projection/parser; original canonical address remains independent of suffix. collect_statuses preserves the incumbent marked-fact/status extraction. A successful native parse containing spec/void remains an explicit status, not absence. A genuinely missing or now-unmarked anchor removes the existing host overlay under the old tie-break. A missing/unreadable native file, raw-only/no native classification or unsupported Doc vocabulary refuses instead of guessing semantic deletion. Explicit empty with citations therefore refuses unresolved native sources; no citations remains no-op.

## Executed receipts inspected (not rerun)

- `/tmp/nst-a4-host-facts-acceptance.log`: 45 PASS. The source bodies include an isolated directory-default-only PROP-slug positive with unrelated malformed source, followed by a separate newly added native-alias negative. Exact native override over raw default/raw same-stem has its own positive.
- Native arbitrary-suffix XML and Markdown cases assert equivalent LAW status, retained VOID, removal of unmarked anchor and unchanged native source bytes. Missing-file versus missing-anchor case separately pins the deletion distinction.
- Later bad citation matrix checks both facts-directory disk bytes and in-memory registry entries before/after refusal. This helper snapshots the registry directory, not the entire source tree; no wider byte-identical-tree claim is inferred.
- Prepared reconciliation cases isolate manifest, byte, native membership, parent/root identity drift; shared-inode positive and symlink/FIFO refusal are exercised. Legacy→declared drift has a separate retained-snapshot test. Unrelated raw addition succeeds.
- `/tmp/nst-a4-host-core-descriptors.log`: 9 PASS. Project presence/null/unknown-field test body inspected; its exact receipt should be retained by writer/root alongside core suite logs.
- `/tmp/nst-a4-host-package-query.log`: 53 PASS; `/tmp/nst-a4-host-boot-carriers.log`: 14 PASS. These cover regression families across shared grammar/policy changes, not complete host adapters.
- `/tmp/nst-a4-host-cli-complete.log`: 3 PASS, including early nonempty-host install refusal, empty dependency boot, ordinary-content consumer refusal and the real nested progress-seal/cache regression.

Counts overlap and are not coverage percentages. Existing HOST-SEAL-01 admission gap now has ancestor policy admission in seal.rs before parse plus the passing actual command case. Root-identified show-effective blanket guard is being narrowed by writer to authored reads so generated tape/empty semantics survive; this review does not duplicate that work or treat its earlier CLI receipt as proof of the later edit.

## Remaining root gates

Reconcile live hashes and rerun the writer-selected gates after the show adjustment; verify final fmt/Clippy/Conform and specification-map/format-ledger coherence. Honest unsupported host adapter refusals remain intermediate A4 boundaries, not completion of host/query/check/docs/skill inventory. No further architecture or broader platform/CAS guarantee is requested by this review.
