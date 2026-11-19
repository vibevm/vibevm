# Hermes builtin skills — bounded architectural acceptance

Date: 2026-11-19. Base: `2d78ca2c22b348e9f6b1ceb17d5150d796787201`.

Accepted candidate manifest: `/data/git/vibevm-native-carrier-20261119/HERMES-SKILLS-CANDIDATE.json`, SHA256 `e312ba2821ece3864c8fa1395b87ca4569e4c5643e887a444e3b9d8d62fa5491`. Independently verified the manifest digest and all 11 current file hashes: nine product files, producer result and finite capability disposition. Commit binding and source publication remain separate follow-up steps.

## Design review

Reviewed final changes to injected skill root/path resolution, snapshot install/adoption/status/removal and CLI builtin install/status/upgrade/uninstall. Hermes derives the destination from the explicitly selected native profile; unsupported agents return None before requiring HOME. Existing snapshot, adjacent receipt, drift and no-follow owners remain. No new receipt grammar, writer, YAML serializer, global environment mutation or alternate personal-layer solver. Automatic maintenance skips unresolved Hermes; explicit selection refuses rather than guessing a profile.

## Independently inspected evidence

- Actual fixture `control/hermes-vibe-skills-proof.py`: install/repeat/status/upgrade/remove; receipt provider; repeat bytes/mtime/receipt stability; foreign neighbor preservation; unchanged config; actual native skill listing before/after/removal; unresolved automatic versus explicit maintenance assertions.
- Terminal explicit/default runtime logs bind binary SHA256 `fe6abbe955cc69ab0776076273805a126ea34f035f8fd7de1953a1a93b4b596b` and PASS. Vibe default lookup removes HERMES_HOME; subsequent native listing explicitly receives the selected profile, so it is not a second independent native default-resolution test.
- Exact library log: projection 89 unit + 20 doctests; MCP 96 unit, integration targets 27/18/5/1/1/7/25, 53 doctests PASS.
- Terminal CLI 31 PASS; build finished; conform 0 new / 141 existing. Independently ran read-only scoped diff check: clean. Formatting is producer evidence, not an architect build claim.

Native execution evidence is producer-run under the existing isolated synthetic setup; architect inspected code/logs and did not rerun a build in the peer checkout. No operator credentials/provider calls or measured cache savings are claimed.

## Boundary and continuation

Accept builtin `vibevm` skill lifecycle only. General package selection/deploy and standalone activation are not fully profile-aware by this atom; no platform or ordinary native session-selector completion follows. SYS-PROMPT/CORP-ENV/NST remain open.

Builder was instructed to commit exactly the accepted 11 files, preserve unrelated dirt, send committed OID, update authoritative plan before refreshing dashboard and proceed to the native session-selection/cwd-before-prefix atom in NATIVE-PRODUCT-IMPLEMENTATION-04.md. Source milestone publication to both mirrors follows committed-blob verification; known GitVerse tag drift remains outside this acceptance.
