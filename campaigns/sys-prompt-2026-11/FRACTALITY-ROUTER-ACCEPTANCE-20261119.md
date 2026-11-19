# Canonical fractality router — bounded reconstruction acceptance

Date2026-11-19. Scope: `vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0`, not host-derived refresh or native Hermes/Pi parity.

## Accepted declared-world reconstruction

The original slot tree was preserved before mutation:614 regular files/5542464 bytes, with independently checked backup hashes. Seven root inputs, including the original schema6 lock, were also preserved. Ordinary initial install skipped stale slots and was not accepted as recovery. Exact nested-root pinned offline refetch `bigf30swu` subsequently completed exit0 through the existing Vibe owner; no whole-host force or manual slot edits were used.

Independent destination verification found31 packages, exact package-array/source/hash equality with the accepted fresh candidate,33 declared dependency edges resolved,31 slot source hashes and595 recorded payload hashes correct,32 INDEX/static inputs present and authored bytes outside all three routers preserved. The fresh candidate's six-output source-free idempotence evidence remains separate and reusable within its original scope.

Actual canonical refetch selected only three changed eligible targets: core-ai-native, rust-ai-native-lang and rust-ai-native-mcp. Three mandatory build callbacks succeeded; three optional MCP callbacks refused without explicit scope and registration remains unperformed. This differs legitimately from the fresh world's31 successful build callbacks/31 optional refusals. No31-callback canonical execution is claimed. The architect independently checked the destination outputs, graph and callback log and accepted this bounded outcome.

Six frozen output SHA256 values:

| Output | SHA256 |
|---|---|
| AGENTS.md / CLAUDE.md / GEMINI.md | `cae076d10e986cbb4b546713195528ceb61253969885b02500c4d1922bfe8baf` |
| vibe.lock | `ba2e4bbef44d64fb7c3618d4ce6107a3580b4179af88e9c0ea1e651c67dfd175` |
| INDEX.md | `ea24a298e14daf6b5bf15e76a28e44794cd79d84cf28360fdd5aa3d7012fc895` |
| STATIC.md | `8fe4921399bd700dd2431a5f761ad17b3cfb8c6bce48a4940cf233324f22e69d` |

Evidence root `/data/git/vibevm-router-reconstruction-20261119/`: `CANONICAL-FRACTALITY-RESULT.json`, `FRACTALITY-CANDIDATE.json`, `FRACTALITY-IDEMPOTENCE.json`, `FRACTALITY-OLD-SLOTS-PRESERVATION.json`, `canonical-fractality-root-before/` and `canonical-fractality-refetch.log`.

## Concrete semantic dependency defect remains open

Correct reconstruction of declared dependencies does not prove that the declarations are complete. Following the owner's explicit correction, a concrete missing direct dependency was found:

- `vibevm/vibespecs/boot/75-tool-fractality.xml:11–14` requires consulting the delegation-rules matrix/boot snippet77.
- `vibevm/vibespecs/skills/fractality-delegate/SKILL.md:8–10` also relies on that matrix.
- Canonical fractality `vibe.toml` declares redbook, rust-ai-native and wal-specspaces only. The31-row graph therefore does not include delegation-rules.

The architect confirmed the bounded remedy: declare `flow:org.vibevm.fractality/delegation-rules = { version = "^1.0.0", access = "private" }` under the consumer's `[requires.packages]`, then resolve and materialize through Vibe. Inspected dependencies show no cycle. Do not add a manual slot or change the general Rust aggregator. Expected31→32 is only a prediction until the actual solve verifies it. Receipt: `FRACTALITY-MISSING-DECLARED-DEPENDENCY.json`.

This manifest repair is NOT implemented. The designated isolated coding lane timed out even on a single no-tools90s availability probe, without a final response or provider error diagnostic. No route switch, direct-coding exception or further blind retry was inferred.

## Remaining scope

The four original canonical router reconstructions now have bounded acceptance. The new fractality semantic dependency repair, host-derived refresh and universal consumer-boot ownership generator change remain open. Native Hermes/Pi full parity, subsequent agent audit and later adoption remain separate obligations. This commit preserves four changed root outputs and this report only; no bulk dependency payload, unrelated dirty files, external push or publication.
