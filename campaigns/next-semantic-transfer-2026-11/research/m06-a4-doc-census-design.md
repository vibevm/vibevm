# A4 declared Doc page census/build/server decision

Read-only design snapshot, 2026-11-07. No implementation/tests/Cargo/Git mutation. Active point-reader draft is a prerequisite, not frozen or accepted evidence. Full A4 remains open.

## Recommended order

1. **Repair generated rendering as an explicit producer input.** The baseline-debt report identifies two real failed_manifest defects and level0/card::synthesise's fake `pack` kind, not merely malformed tests. Keep authored package admission strict. Introduce a narrow generated-render input at the existing build composition boundary: generator-owned PageSet/SpecDoc plus actual subject coordinate/version/title/language and optional original kind. Unknown original kind remains unknown. Feed the same render/card/projection functions, without writing a pretend authored package manifest then invoking ordinary package admission. Do not add `pack` to PackageKind or camouflage the original subject as `kind=doc`. This can be a preceding coherent product patch in site/render.rs, site/level0/card.rs and the narrow build entry; a4_tutorials owns the62 baseline fixture inventory and is not duplicated here.
2. **Add a captured selected Doc inventory and actual build/server callers.** Point capture supplies per-document semantics and guards, but does not establish complete page membership. Keep census separate from guessed repeated point addresses.
3. **Join self citations and borrowed examples to that same selection.** Exact borrowed_by can reuse captured point reads; broad borrowed and translation mirror validation use the selected page inventory. No guard deletion without an actual bound read path.

## Actual source gaps

- pages::read_package currently returns empty for ExplicitEmpty, refuses Explicit and otherwise walks `.xml` under SPEC_ROOT. pages::documents bypasses policy and still returns that suffix inventory, so declared raw XML can incorrectly appear as a documentation page even while actual parsing refuses.
- build::load calls read_package(package_dir), then resolves citations and borrowed examples. A bare installed slot path carries no independent hash/format binding.
- doc-server Reader::open stores package_dir plus injected SpecSources. Actual routes.rs page handler calls read_package(path), reconstructs every URL as `{stem}.xml`, then selects Page.rel. Arbitrary original suffix cannot work by parser replacement alone.
- Self citation resolves `{doc_path}.xml` and checks anchors from a path. borrowed_by reads adapted root + page path and forces Doc XML; borrowed reads all source pages through read_package. They still use explicit guards and path rereads.

## Inventory seam and scope

Extend the existing SpecSources selected-root machinery after point acceptance with a fallible package-page inventory operation. It needs exact source instance identity, not just coordinate-based lookup that might select a nearer instance than the explicit build package_dir. Confirm requested package_dir equals the selected candidate root; retain current checkout/in-tree/lock/store precedence only when resolving a coordinate, not when a caller explicitly names a different package root.

For authored roots, reuse the held policy/finite inventory owners and shared descriptor selection. Inventory only the operation's selected VibeDoc pages, preserving declared exact/directory scope and original names; ordinary/non-Doc bytes do not become pages merely because they are XML. Do not call broad whole-host inventory then parse/filter if a missing irrelevant ordinary resource would poison this page operation. Factor a narrow native-page selection from the existing inventory owner rather than add a second recursive walker or global census. Exact selected page absence is an availability failure; selected ambiguous native identity refuses. Generated outputs remain separate.

For installed roots, reuse the point-reader's full independently selected lock kind/group/name/version/hash and trusted input SpecFormat, existing prepared slot mapping and held reads. Inventory comes from validated retained output rows/original descriptors, not physical suffix enumeration. No new record parser or fake package metadata. Doc Markdown remains unsupported under the established one-way representation contract. Without independent installed selection, a bare path API must refuse declared materialized roots rather than manufacture expectation from the slot record.

Only actual Doc package metadata authorizes declared Doc vocabulary. A nested manifest is data under the selected outer scope, never an authority switch. A plain project or non-Doc package cannot obtain Doc parsing by adding rows. Missing/legacy compatibility remains explicitly the historical XML page route where it was already permitted; current absent-manifest fixtures and generated producer inputs are different cases. Explicit empty yields zero pages/documents. Malformed present manifest still refuses. Preserve original source bytes and capture every selected page before interpretation, then recheck membership/policy/source observations before returning a complete census.

## Result, diagnostics and identity

Keep PageSet's useful distinction: native syntax failures are UnreadablePage entries while other readable pages remain available; selected source admission/census/read failures are errors, not fabricated zero pages. documents() must list valid selected page identities without requiring successful syntax parse, so malformed selected native page remains in navigation comparison. It may need a bound sibling returning selected membership without forcing AST construction; no extra schema is needed.

Page.rel must describe the ORIGINAL logical path, not converted output path or appended `.xml`. Use the shared canonical identity/address mapping accepted by point reading; retain physical path only for diagnostics. Check page output/link/route mapping for arbitrary suffixes and alias collisions. Server lookup should match logical document identity (e.g. pages::document_of/accepted canonical mapping) to an observed Page, then choose Html/Md/Xml projection from URL; it must not reconstruct the source filename. Preserve trailing-slash redirects, missing page404 and unreadable selected page500 behavior. Existing `.xml` page URLs remain byte-compatible.

Retain a private captured census/guards through build preparation/rendering. Bare PageSet may remain a compatibility result, but actual declared build must recheck before output publication and server before response. Avoid making Page clone carry an opaque filesystem authority accidentally; a private prepared-read wrapper around existing PageSet is sufficient. Direct Doc AST parsing preserves note/figure/example metadata; do not round-trip through Markdown. Borrowed examples use original logical page identity and the same source instance, and don't fall back across a selected-source refusal.

## Actual integration perimeter

After point acceptance: pages.rs and bounded census child/tests; SpecSources package-selection/inventory method coordinated with the point owner; build.rs load/build path; citations self-check; translations/borrowed plus translation selection as necessary; doc-server routes.rs and Reader composition only for trusted context. CLI doc/env already supplies selected format in the point slice; reuse it, not ambient config in the library/server. Necessary lower inventory extraction is root-owned and narrow. Keep generated-render repair distinct from authored admission even if both land in one reviewed patch.

## Discriminating tests

- Public read_package/bound census on an actual declared Doc package: exact +directory-selected arbitrary-suffix native pages, raw same-stem/nonUTF8 resources excluded, nested manifest inert, selected malformed page listed unreadable, explicit empty zero, missing page unavailable; legacy equivalent unchanged.
- Actual local-registry installation in Mixed/Xml followed by selected bound page inventory/build. Original page identities survive renamed outputs; missing/wrong format/hash/record refuses before render writes. No claimed Doc Markdown support.
- Real `vibe doc build --path <doc> --out <temp> --format md` emits selected pages, substitutes real captured rule citations and borrowed example references, and preserves raw input bytes. Snapshot source/output on late census/read refusal.
- Existing server route harness (`vibe-doc-server/src/routes/tests.rs` and routes/tests/editions.rs) serves `/guide.payload/`, `/guide.payload.md` and `/guide.payload.xml` from the same original declared page, retains ordinary `.xml` routes/redirects and does not serve raw resources as pages. Reader::open alone is not request-path proof.
- Self citations honor real anchors and existing positional pNN exemption; declared empty cannot fabricate a page. Borrowed_by reads exactly one selected source page; broad borrowed validates the selected page set.
- Generated refused-version and unknown-kind level0 card tests render graceful error/card pages while preserving unknown original kind, without relaxing invalid authored manifest negatives. Consume a4_tutorials baseline classification and rerun corrected corpus; don't relabel all62 failures as fixture-only or skip them.

This is the coherent page-census closure after point reads, not proof of current runtime support or whole A4 completion.
