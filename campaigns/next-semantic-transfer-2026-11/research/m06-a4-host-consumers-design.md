# A4 declared host consumer seam — implementation decision candidate

Read-only, 2026-11-07. No implementation/tests/Cargo/Git mutation. Host grammar/cited-sync/guards are the foundation; nonempty unsupported adapters are temporary, not final A4 support. Root chooses the implementation contract before activation.

## Next slice: actual grouped host native lookup through the existing resolver

Add a declared HOST SourceLocation reader beside the existing Legacy and installed Declared readers. Its data is the existing held PackagePolicyObservation plus one captured original source (relative path, declared media/interpretation, SourceObservation identity/bytes and relevant parent/member observations). Reuse/factor source_policy/host.rs candidate classification and recheck; do not synthesize SlotRecord, ResolvedDep, PackageMeta, selected package hash or a new source world. The host manifest and caller-selected root are the independent intent; there is no materialized representation transformation to infer.

`FileResolver::locate_source` currently tries installed declared_location, then observed_root and locate_in. observed_root already knows when an address is the actual self coordinate, preserves versioned-self refusal and distinguishes derived installed self slots from caller-supplied roots. Introduce the host arm only for a real project root selected as self, after installed binding dispatch; never reinterpret installed slot as host merely because it has files. Validate the captured manifest consumer coordinate matches the resolver SelfCoordinate. Use capture_cited_native for grouped native lookup and return builtin markdown metadata with original physical stem/path, declared_media and held bytes. `FsSectionSource::read_location` already projects selected native media before anchor resolution and compares metadata/path on re-resolution; extend its LocationReader dispatch, not its compiler semantics.

For this first slice an ordinary resource remains non-native, so spec:// lookup refuses it even if it contains a matching anchor or #use. Host raw reading can use the same observation internally by exact relative path for later consumer joins; no public resource URI or command is introduced. Unknown/unsupported custom frontend must not receive the declared host file via extension fallback. Exact overlays, if used, must be tied to captured bytes/metadata or refused; no bad-overlay fallback to disk.

## Minimal reusable observation shape

Prefer a small extension of existing source_policy owner, not an exported service:

- An exact relative-path operation selects shared rows, captures original bytes through the held root, retains selected descriptor/provenance, and rechecks root/manifest/parents/source. It can represent ordinary bytes without creating native identity.
- A native-address operation selects the exact cited canonical candidate set via the existing capture_cited_native law, then uses that exact read operation. It requires a real group/name and native interpretation.
- A finite inventory operation is a later caller-owned input for facts/boot: exact rows plus only relevant declared directories, with selected path/identity observations; it does not claim the entire repository or package universe was scanned.

Keep the current CitedNativeSource guarantees and its shared-inode source reader. Its coordinate requirement is appropriate for addressed reads; do not remove it merely to support ungrouped Simple boot. An exact original resource read can lack a coordinate, while native fact/address minting must require one. Preserve source identity, bytes and interpretation as separate concepts. No global quotas, permanent FD per file or source-authentication claim.

## What this unlocks next

1. **Host lookup:** actual FileResolver/FsSectionSource path above. Tests: project.sources + native XML under .payload, spec:// original address and anchor success; raw same-stem excludes native collision; duplicate native canonical candidates refuse; wrong self coordinate/version and ordinary/custom-frontend cases refuse. Directory-default-only slug lookup and held drift use existing host tests. This is the smallest complete useful next slice.
2. **Host facts/query:** replace the temporary declared_capture nonempty-host refusal with finite selected-source inventory supplied by the same host observation. Reuse direct Spec AST facts extraction (not XML→Markdown→parse), complete captured witnesses and existing Available/Invalid/Unavailable semantics. Do not cast a project into PackageMeta or borrow installed output witnesses. Group is required before minting host fact addresses; explicit empty remains zero facts. Query is read-only and shares the same source owner; preserve grammar-before-I/O and source digest/report rules.
3. **Own boot:** select only relevant authored files in the immediate node boot directory, not every source under specs root. Native media chooses projection even with arbitrary suffix. Exclude generated STATIC/INDEX by existing ownership predicate. Retain original node-relative paths, categories/order and current provider types. Add the host captured source branch to BootSectionSource/input assembly; node_own_boot must carry the same selected read intent rather than discover a file and reread it by suffix later. Remove early nonempty-host admission refusal only when this actual pipeline supports the selected operation before effects.

## SourceLoading: separate scanning, explicit selection and interpretation

The existing descriptor law says scan may enumerate any interpretation, while only VibeSpec/VibeDoc may mint native meaning; ordinary text never becomes a managed directive because it is enumerated. The task's “explicit loading for ordinary resources” means an actual resource/skill/user selection is necessary to consume it as data; it is not permission to reinterpret data as boot. Merely placing a resource under spec/boot is not selection.

Recommended host contract for root approval: automatic own-boot inventory selects native VibeSpec + loading=scan in the boot scope; loading=explicit native can be read by an exact native-address request but is not auto-enrolled. Ordinary scan rows may be inventoried/structured as data, never added to native boot. Host facts whole-scope inventory must state its enrollment rule explicitly before implementation (scan-only by default versus explicit requested inventory). Do not silently change already accepted package facts/query behavior: those exact declared rows include loading=explicit in existing fixtures and currently yield requested facts. A reusable capture seam must carry loading, not independently impose a global rule that breaks those consumers. The first exact-host-lookup slice is unambiguous and can start while root pins the automatic inventory rule. No new transport or source_exports is necessary.

## Simple/Normal and ungrouped hosts

Main boot/tests.rs explicitly asserts own authored boot uses PackageFormat::Simple; mid-band own boot is dynamic by the existing boot composition law, while inherited/foundation/override categories retain their incumbent rules. Source interpretation is not package format: XML VibeSpec may project into a Simple input without opening a Normal compiler path. Do not add `[project].format`, infer Normal from XML, or grant ordinary literal text directive authority.

Grouped host identity uses actual SelfCoordinate. Ungrouped hosts already have DocumentProvider::HostUngrouped(name), and virtual workspace has its own provider; neither is a fabricated spec:// coordinate. Such a node may retain explicitly admitted original resources/Simple boot using its selected root/name and provider, but cannot mint fact addresses or satisfy self-coordinate spec:// lookup. A native Simple input requiring an addressed reference still needs the incumbent explicit refusal; do not invent group aliases. Named consumer behavior should be tested independently of address-bearing facts.

## Immutable prototype evidence and rejected copying

At pin `7fe6376ccef7f321302a8f18610fcb0c9cc2686f`:

- PROP-072 SOURCE-DESCRIPTOR-GRAMMAR documents scan/explicit and non-native ordinary semantics; SOURCE-HOST-OPT-IN was spec/plan, and its absent/empty legacy statement is superseded by accepted main presence semantics.
- `vibe-facts/src/scan.rs::observe_authored_source` still enumerates suffix documents directly; its pure declared walker is package-typed and does not activate project policy. It is not a completed host query join.
- `vibe-spec/src/resolver.rs` uses descriptor maps only when explicitly bound to a specs root; otherwise falls through to suffix lookup. `source_map.rs` is host-validated output-map machinery, not an independent original host manifest reader.
- `install/bootgen.rs::node_own_boot` enumerates files then calls captured::authored_boot_from_inventory. That helper filters `is_spec_source`, assigns categories and checks suffix pairs without project descriptor/loading classification. It is not evidence that declared host boot was correctly implemented.
- Pinned host sync is the concrete opt-in implementation already audited; its blanket directory refusal and fixed caps were not adopted.

One mistaken pinned path probe for `boot/captured.rs` failed; the actual module is `install/bootgen/captured.rs`, subsequently read and hashed. No live prototype worktree was read or changed.

## Perimeter and end-to-end acceptance

Next exact-lookup write perimeter: source_policy/host.rs (+ focused tests), resolver/observation.rs and narrow new host reader module, FileResolver registration only if required, embed/read.rs only if its existing media dispatch needs enum support, actual resolver/SectionSource tests, permanent PROP-072 law. Avoid package-query/facts installed-owner churn. Later inventory joins add facts declared_capture/host child, direct parser extraction if needed, workspace node boot/source admission/input carrier and actual CLI tests.

Real discriminators after the lookup slice: (a) grouped project native arbitrary-suffix + ordinary same-stem → actual host requirements query shows only native address/status/void and complete witnesses; (b) install/regenerate with scanned native own boot plus literal ordinary resource → compiled output has selected native content only and no ordinary #use/#embed execution; (c) explicit-only native file not auto-enrolled but addressable on exact request; (d) ungrouped Simple boot works without fake coordinate while addressed facts refuse; (e) changed policy/candidate/file before consumption refuses before any publication; (f) legacy absent and explicit-empty behavior remain pinned.

Full A4 still includes docs/citations, show effective, skills, bound graph check, adoption/remaining installed consumers, help/tutorial corrections and the final mixed-source matrix. Precise refusal is safe intermediate behavior, not a substitute for that completed functionality. This design supplies a concrete foundation order, not a claim those remaining features exist.

## Root verdict for sequential implementation

First implement the existing resolver/SectionSource's real held host read path, without fake package metadata or installed records; preserve source policy and selected root/coordinate binding through consumption. Then reuse it for broad host facts/query and own Simple boot. This is an unfinished A4 consumer obligation, not justification to keep temporary Unsupported guards forever. Existing HostUngrouped semantics may carry raw/Simple input without minting a spec address; grouped native address operations need the actual selected coordinate. Do not add a new host format or reinterpret own Simple input as Normal.

Loading is operation selection, not a universal byte-read ban. Root independently checked pinned PROP-072 SOURCE-DESCRIPTOR-GRAMMAR (`scan may enumerate a declared source of any interpretation`) and the pinned facts `complete_reader_keeps_large_spec_only_set` fixture: its package helper declares native loading=explicit and the test requires320facts. Therefore requested facts/adopt/query observations retain native explicit and classified raw witnesses without making ordinary data normative. Named native lookup/citation accepts either loading value; a named ordinary file/export or skill.resource declaration is explicit selection and remains literal.

For automatic own-boot discovery, root chooses native+scan within the existing boot scope. Native explicit is not automatically enrolled, and ordinary scan is never a compiler input. This is a clarified target rule consistent with the selector vocabulary and A4 negative, not a claim that the prototype proved it. Explicit package boot_snippet/source requests retain their separate existing explicit-selection semantics. No new export/URI/MCP transport is introduced.
