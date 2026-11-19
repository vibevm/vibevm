# Hermes native YAML MCP projection — coherent candidate

2026-11-19; base `c25b060f8`. Existing native projection obligation, not complete Hermes/SYS parity.

## Owners / design disposition

Existing Agent registry gains Hermes, YAML ConfigFormat and canonical JSON-valued per-server YAML payload. Native profile directory is explicit injected context. CLI captures HERMES_HOME when given and otherwise queries the admitted installed runtime's native early profile selector/get_hermes_home. Pure library refuses missing or unsafe profile input rather than guessing `.hermes`; automatic unresolved discovery skips only that unresolved agent. User launcher override resolves through existing AgentsConfig/launchers, matching Hermes kind/direct admitted runtime; wrapper standing arguments refuse instead of silently using ambient plain Hermes.

Native Python helper is embedded in Rust and executed under the selected interpreter with `-I`. It reads only an invocation-owned captured copy, validates desired MCP entry through native owner, and calls native FileBackend's comment-preserving YAML writer on a private rendered stage. Native backend remote mode refuses. Only owned-key JSON crosses private result file; no full config in stdout/stderr or reports. Owned entries may be sensitive: request/result files are never logged. Scratch temporary home is not the actual agent process home; selected live config is never redirected or mutated by the helper.

Existing Rust PublicationBatch captures/rechecks real config, creates final publication stages, publishes through safefs OwnedFileReceipt and retains existing pending/inverse/foreign-drift behavior. No external native final write, new YAML serializer, new journal or after-image-derived rollback authority. Parent uses capability bounded reads, immutable embedded helper bytes and private scratch files. Windows ACL confidentiality is not claimed from Unix0600.

Helper process has owned-child RAII kill/reap on cancellation/error and bounded30s polling; existing invocation cancellation token is checked. MCP finite operations now attach the existing command cancellation source, not MCP serve. Native YAML status/preview uses temporary scratch and does not publish helper assets/stage directories into selected settings/profile. Public install/status/maintenance paths receive caller directory context; automatic missing-profile discovery does not poison all other agents.

## Actual proof

Private root `/data/git/vibevm-native-carrier-20261119/control/`:

- `hermes-vibe-mcp-scratch-fixed.log`: actual rebuilt Vibe install/repeat/upgrade/uninstall, synthetic Hermes profile and real pinned native FileBackend. Adjacent ownership receipt package/payload asserted, repeat receipt/config identical, uninstall drops owned receipt entry. Foreign server/model/operator values/comments and anchor/alias survive. Binary SHA256 `3c19c4195aa7999dce68c33a91619e5da9903831e233b20dfc4a1ec2cc84ad3b` frozen by driver. No servers/provider started.
- `hermes-vibe-mcp-default-profile.log`: same actual lifecycle with HERMES_HOME removed from Vibe child environment, native lookup selects expected synthetic default home; code0.
- `hermes-yaml-projection-final.log`:89 projection unit tests +18 doctests PASS.
- `hermes-yaml-helper-tests.log`:4 native helper/publisher/runtime tests PASS, including cancelled owned-child reaping. This is guard unit proof, not every process-tree/signal failure cut.
- `hermes-yaml-mcp-final-tests.log`: all MCP targets PASS:95 library,27 agent-profile integration,18 config,5 package-server,1 projection-transport,1 skill-template,7 skill,25 tool surface,53 doctests. Counts are per target, not disjoint feature claims.
- `hermes-yaml-cli-final-tests.log`:31 CLI registration/activation/discovery tests PASS.
- `hermes-yaml-coherent-conform.log`:0 new findings/141 existing. Formatting and diff check PASS.

All native execution unshare-all sandbox, pinned source Hermes4ae451d4126327c871fd084e60c045dec0c85911 and synthetic HOME/config, no operator auth/config read or provider request. Temp cleanup oracle in old driver checks no profile `.vibe-mcp-stage/render-*`; newer scratch cleanup relies on owned TempDir lifecycle and is not separately asserted as an exhaustive leftover proof.

## Rework / failures retained

Initial command returned targeted[] because detection omitted explicit native profile context; fixed at CLI detection. First fixture sent install-only `--what` to upgrade/uninstall; native grammar correctly refused, fixture corrected to --config-only. Scratch migration initially chose scratch-home/config.yaml, triggering helper live-config guard; moved rendered file below scratch render/ while keeping guard. Full projection88PASS1FAIL was old unknown-agent diagnostic; MCP profile26PASS1FAIL expected ambient default for every agent; updated for explicit Hermes injected home. Conform install-module length615 failed; split result rendering along existing owner seam, no budget waiver. Prior failed commands/logs remain, no shell aggregate0 relabelled as green.

## Explicit remaining / not claimed

Profile-aware Hermes skills APIs/callers remain incomplete; old ambient skill methods cannot replace injected native profile selection. Native default query and explicit profile MCP behavior are tested; sticky profile variants/native remote backend/Windows/macOS wrapper qualification not claimed. Cancellation guard test does not establish all CLI signal timing or process-tree descendants. No raw secret-safe general keyring, agent adapter refactor or corporate setup implementation is added here.

Remaining six-harness capability table is `SYS-NATIVE-CAPABILITY-DISPOSITION-20261119.md`: real workers, remaining ordinary Hermes session flags/shared gateway/platforms and final parity audit stay open; intentional Pi replacement-slot conflict is separate from unimplemented capabilities. No SYS/NST completion, provider cache-hit/cost savings or public release claim.
