# Hermes profile-aware builtin skill projection — candidate

2026-11-19; base `2d78ca2c2`. Bounded builtin skill lifecycle, not SYS/NST completion.

## Existing owners

Pure `Agent::skills_root_with_dirs` / `skill_path_with_dirs` consume explicit operation directories. Hermes derives its skills root from the already resolved native profile home used by config projection, never from generic HOME or a guessed default. Unsupported skill agents return None without requiring HOME; project Hermes skills remain unsupported. Existing public ambient APIs remain compatibility wrappers.

Builtin MCP-command skill install/adoption, read-only status, upgrade and removal now receive the CLI's selected directory context. Native default profile lookup remains the existing admitted-runtime selector introduced with YAML MCP; explicit native profile input remains the existing composition-root capture. Shared snapshot, standalone adjacent receipt, drift checks, no-follow path checks and removal retain their existing owners. No new receipt format, writer, environment override, YAML operation or personal-layer solver.

Scope is explicitly builtin `vibevm` skill registration and maintenance. General package-skill selection/deploy and standalone activation APIs are not represented as fully profile-aware by this atom.

## Evidence

Private retained root: `/data/git/vibevm-native-carrier-20261119/control/`.

- `hermes-skills-native-explicit.log` / `hermes-skills-native-default.log`: actual Vibe install/repeat/status/upgrade/remove and pinned Hermes `tools.skills_tool.skills_list` discovery before installation, after installation and after removal. Selected explicit profile and installed-native default lookup both pass. Receipt provider asserted; repeated file bytes/mtime and receipt unchanged; foreign neighboring file preserved; config.yaml unchanged. First observed binary SHA256 `58f2b70ad00327876df9aa5175824a20c5295d6bcecccedad4713891f45cb645`; final-source binary rerun recorded separately below when complete.
- New library regression exercises injected profile lifecycle, ownership observation, foreign neighbor preservation, missing/relative profile refusal and unsupported-agent no-HOME skip.
- Initial affected library run passed: projection89 unit/19 doctests; MCP96 unit and all integration/doctest targets (27/18/5/1/1/7/25/53).
- Corrected CLI gate:31 tests pass. Initial CLI command mistakenly used `--lib` on a binary-only crate and failed before tests; retained `hermes-skills-cli-tests.log`. Correct binary-target retry passed; no failed command is counted as green.
- Exact-source libraries (`hermes-skills-exact-libraries.log`): projection89 unit/20 doctests; MCP96 unit plus27/18/5/1/1/7/25 integration and53 doctests PASS. Terminal CLI31 PASS, rebuilt terminal native explicit/default journeys PASS on binary SHA256 `fe6abbe955cc69ab0776076273805a126ea34f035f8fd7de1953a1a93b4b596b`; logs `hermes-skills-terminal-{cli,build,native-explicit,native-default}.log`.
- Initial/exact coherent conform checks:0 new findings/141 existing. Terminal conformance receipt accompanies the source manifest.
- Terminal native fixture additionally removes the launcher/profile input: automatic upgrade/uninstall skip unresolved Hermes while exact Hermes selection refuses. Default Vibe lookup is tested with HERMES_HOME removed; the subsequent independent skill listing explicitly receives the selected profile, proving discovery there rather than independently retesting native default selection.

All native execution uses the existing unshare-all credential-free sandbox, synthetic HOME/settings/profile, pinned Hermes source `4ae451d4126327c871fd084e60c045dec0c85911`, Python `-I`, no operator auth/config and zero provider requests. No measured cache savings, broader platform qualification, all failure cuts or Hermes ordinary session-selector completion claimed.
