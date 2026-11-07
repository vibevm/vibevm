# Residual after Doc-only census — level-zero native Spec projection

Read-only source audit, 2026-11-07. No runtime tests, Cargo, edits or Git mutation. Active Doc census writer retains its perimeter; this is a next-slice recommendation, not a failing-runtime claim or A4 promotion.

## Concrete residual

`site/level0.rs::compose_prepared` calls pages::read_package(source) whenever source/SPEC_ROOT is a directory. That wrapper now calls SpecSources::new().capture_pages. For explicit nonempty policy, census::capture_original_pages obtains actual package metadata and invokes capture_native_inventory(VibeDoc). The lower inventory correctly requires actual kind=doc for that interpretation.

Therefore a valid flow/lang/tool package with declared VibeSpec XML under SPEC_ROOT takes a Doc-only route and refuses. The equivalent absent-sources legacy package follows read_legacy and renders its XML. This is established from the caller/body chain; no execution was performed here. Existing site/level0/tests.rs::plain is a concrete valid flow fixture with common/PROP-001.xml and demonstrates the existing level-zero intent. Add a sources declaration selecting that same native Spec file to isolate the missing join.

The correct Doc-kind restriction must remain. The missing behavior is an explicit **native Spec→level-zero render adapter**, not permission for non-Doc authored content to parse as Doc. Do not add kind=doc, fake a manifest, restore suffix fallback or widen capture_pages(VibeDoc).

## Recommended next coherent slice

Do one bounded level-zero source projection slice before tree→show→progress. It completes the actual build-site “any package” path now affected by declared source admission and reuses the already added Generated render input rather than duplicating a renderer.

At composition select by actual source role/policy:
- actual Doc authored pages continue through the accepted bound Doc census;
- non-Doc/native Spec or grouped host Spec uses existing capture_native_inventory(VibeSpec) and direct Spec parser, then enters the existing generated projection/card/manifest pipeline as source-backed native content;
- ordinary resources are not promoted to pages by suffix; README/manifest reference generators remain explicit independent render producers;
- absent/empty/legacy contracts remain explicit, with no invented group or source identity.

Use the original logical resource/address mapping and selected media. Keep original subject kind in generated card metadata, including unknown kind when the producer legitimately lacks it. Parsing VibeSpec and rendering its existing SpecDoc blocks through the common page renderer does not require pretending the source is a Doc package. Any generated wrapper/augmentation must remain distinguishable from captured original native content.

## Two adjacent caller issues to include in that slice

1. **Physical SPEC_ROOT gate:** compose_prepared currently skips read_package entirely if SPEC_ROOT is absent. A declared exact native page outside that directory (e.g. guide.payload, supported by point/census original identity) can therefore vanish into card-only rendering. Select the actual declared operation scope rather than use directory existence as proof there are no authored sources. Preserve legacy directory-empty behavior separately.
2. **Capture lifetime/copy:** pages::read_package returns a cloned PageSet and drops CapturedPages; compose_prepared subsequently copy_specs uses fs::copy on page.path, then returns Generated {pages,card} without retained source guard. This is source evidence of an unbound composition edge, not a demonstrated race. The new source-backed projection should carry retained observation and original captured bytes through copy/render/publication, or finish the relevant recheck before each effect under the existing finite publication law. Do not label copied authored pages as guard-free producer data simply because Generated also carries newly composed pages. No global transaction/CAS claim is required.

`manifest::build` also routes all explicit sources through capture_pages; its documented primary role is a documentation manifest. Do not broaden that API blindly. Level-zero already has build_for_card/build_generated assembly for explicit generated/source-backed content; reuse that route and test its machine manifest. Treat any direct non-Doc manifest command promise as a separate explicitly demonstrated caller requirement, not an inferred reason to weaken Doc census.

## Exact useful test/perimeter

Primary source owners: site/level0.rs (source role selection/retained composition), narrow source projection child if useful, build::Generated/build_generated carrier only for retained source-backed input, and existing native inventory read API. Do not edit Doc-only guards to accept Spec. Coordinate any shared lower accessor with root after current writer freezes. Existing site/render.rs::render and level0 tests provide actual composition call sites; add one build-site render fixture for the declared flow/host case rather than only parser construction.

Discriminators:
- Legacy flow fixture versus the identical source declared VibeSpec XML: level0 compose/render must carry common/PROP-001 address/content and correct flow card in both; raw same-stem XML remains data.
- Declared native Markdown/arbitrary suffix renders through Spec parser with original address; literal ordinary #use/#embed never gains compiler/page meaning.
- Declared exact source with no physical SPEC_ROOT is not silently dropped; use the operation's admitted logical path and existing page address rules.
- Actual Doc package remains on Doc census and supports Doc-only blocks; the same Doc-only syntax under non-Doc VibeSpec refuses or appears as an unreadable native source, not accepted via vocabulary widening.
- Source-backed capture drift before composition copy/output refuses with the existing finite guard; copied diagnostics and rendered AST must not describe unrelated bytes. Keep generated error/card producer tests distinct.

This is a real remaining A4 consumer, ahead of the independent CLI views. Tree/show/progress designs remain valid afterward. No further broad parser inventory, new source world or current runtime/whole-A4 claim is made.
