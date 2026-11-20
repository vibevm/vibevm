# Ordinary package selected subskill — qualification candidate

Baseline accepted boolean implementation `174978ac5`; test-only architect contract `eab934791` integrated as `fca035662`. No production loader, store, lock or projection changes.

Three synthetic public-install worlds use the same inert uniquely marked `skills/audit/SKILL.md` source: a selected feature with a Subskill effect, a dormant feature with that effect, and a package with no Subskill effect. The file exists in all source worlds so full-copy presence cannot masquerade as activation. No embedded commands, agent launch or provider calls occur.

## Existing owners and first gap

- Declaration and path admission: `crates/vibe-core/src/manifest/configuration.rs` ConfigurationEffect::Subskill and configuration/validate.rs declarant-path validation.
- Original input normalization: configuration/intent.rs ConfigurationDecl::normalize_input; feature/group selection remains the existing shared evaluator.
- First runtime handoff: configuration/package.rs PackageConfiguration::evaluate_parts explicitly refuses a selected Subskill effect with `selected subskill loading is not connected` before slot/lock publication. Parser support is not runtime support.
- Historical installation record: `crates/vibe-install/src/record.rs` initializes LockedPackage.subskills_active as empty. No selected common-effect-to-delivery handoff is established by that field alone.
- Boot composition: `crates/vibe-workspace/src/boot/hybrid.rs` owns recursive static membership/boot compilation from existing package boot declarations and direct edges. A full source copy is not an additional boot contribution.
- Public inspection: `crates/vibe-cli/src/commands/show/subskills.rs` reads admitted lock subskills_active, not arbitrary source files. It cannot prove real agent loading.
- Nearest existing skill-delivery owner: `crates/vibe-agent-projection/src/pkgskill.rs` and pkgskill/projection.rs implement package-declared skill collection, projection and managed activation. Their existing [[skill]] contract is distinct from a selected ordinary ConfigurationEffect::Subskill; it is a reuse candidate for architect review, not an already wired consumer or permission to add a parallel loader.

## Runtime evidence boundary

The focused test checks exact selected refusal with absent lock/slot/hooks and unchanged authored manifest and launcher bytes. Dormant and absent-effect controls must install the same inert file into the full slot, keep its unique body out of generated static boot, report no active subskills through the actual public show command, and cold-replay byte-identical lock/boot/hooks and slot skill bytes. `subskill-qualification-test-r3.log` records focused PASS across all three worlds in the same credential-free filesystem/process/network sandbox; `subskill-qualification-clippy-r3.log` records strict scoped lint PASS. Formatting and diff checks passed. The private candidate manifest binds both authored files and the actual executable SHA-256 `c8488fe855a811cae8c8d2a9f3cc2323da8b16c7c5cbc39da8f8539ebcdfca6b` before independent review.

Selected installation, selected boot delivery and selected discovery are NOT REACHED after the existing evaluator refusal. Actual agent loading is UNRUN in all worlds by design. This qualification identifies the first missing handoff; it does not implement it or close broader common-configuration, nine-kind, USER-PROJECT or NST obligations. REMAINDER/AR/IDE unchanged.
