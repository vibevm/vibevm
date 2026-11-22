# ADV-SDD-SPEC-SECTION-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The committed `specmap.json` traceability index lists the new spec_section module, the read_spec tool, the show section command and their test files, and `cargo xtask specmap --check` passes.

**Write.** `specmap.json`

**Check.** `cargo xtask specmap --check`

**Test.** No new test file. The gate is the existing byte-compare in `cargo xtask specmap --check` (xtask/src/specmap.rs), followed by its resolve gate. After regeneration, specmap.json contains the strings "crates/vibe-workspace/src/spec_section.rs", "vibe_mcp::tools::read_spec::ReadSpecMcpTool" and "crates/vibe-cli/src/commands/show/section.rs".

**Read first.**
- xtask/src/specmap.rs:1-60
- specmap.toml:40-60

**Steps.**
1. From /fast/git/vibevm-architect, with entries 1-4 applied, run `cargo xtask specmap` (regenerate, no --check).
2. Confirm with grep that specmap.json now contains crates/vibe-workspace/src/spec_section.rs, vibe_mcp::tools::read_spec::ReadSpecMcpTool and crates/vibe-cli/src/commands/show/section.rs.
3. Run the check `cargo xtask specmap --check`.

**Decisions already made.**
- This entry is the chosen option for the specmap freshness gate. Entries 1-4 do not touch specmap.json. The lead treats a red `specmap --check` before this entry as expected, not as a blocker.
- specmap.json is generated output and must never be hand-edited. The only allowed write is the output of `cargo xtask specmap`.
- If the regenerated diff contains changes unrelated to the four new files and three new tests, stop and report it as a pre-existing drift instead of committing them silently.
- If the resolve gate reports one of the new spec:// anchors as unresolved, report it as a defect of the entry that added that anchor. Do not change the anchor here. The anchors used are PROP-035#URI-OBSERVED-SOURCE-READ, PROP-015#tools and VIBEVM-SPEC#command-summary, and all three are already used by existing code.

**Not in this entry.** Editing any source, spec or specmap.toml to make the gate pass; regenerating other generated artefacts (vibe-wire schemas, conform maps).

**Depends on.** ADV-SDD-SPEC-SECTION-1, ADV-SDD-SPEC-SECTION-2, ADV-SDD-SPEC-SECTION-3, ADV-SDD-SPEC-SECTION-4

Entries that write the same file run one after another, never in parallel.
