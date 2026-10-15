# M-08-A.2 four-client contracts candidate

Source: `af61dd1aaf57ba4117e41f8d586b55a74bb26153`. Authorized documentation writes are completed; Root acceptance is pending. No runtime support is qualified.

The separate [contract fixture](../../../../fixtures/next/adapters/four-client-contracts.json) contains 33 rows, 26 fetched official sources and 396 future proofs (12 groups per row). All four clients cover project/user skill, MCP, config and plugin/extension surfaces. The additional OpenCode row distinguishes legacy skill/MCP projection from native JS/TS plugins. Unsupported native OpenCode and Codex project-plugin combinations remain explicit refusals.

| Client | Exact binary observation | Source-characterized skills, project / user | MCP path, project / user |
| --- | --- | --- | --- |
| Claude Code | Missing from worker PATH; no probe | `.claude/skills` / `.claude/skills` | `.mcp.json` / `.claude.json` |
| Codex | Missing from worker PATH; no probe | `.agents/skills` / `.agents/skills` | `.codex/config.toml` / `.codex/config.toml` |
| OpenCode | Missing from worker PATH; no probe | `.opencode/skills` / `.config/opencode/skills` | `opencode.json` / `.config/opencode/opencode.json` |
| Qwen Code | Missing from worker PATH; no probe | `.qwen/skills` / `.qwen/skills` | `.qwen/settings.json` / `.qwen/settings.json` |

Paths use distinct symbolic project and injected-user roots. They are source facts, not proof that a locally available CLI discovered them. The lookup was metadata-only and repeated in non-login shell; no client executable ran. No HOME override, installation or real-config access occurred.

OpenAI Docs was used from the exact authorized skill. Current [Codex skills](https://learn.chatgpt.com/docs/build-skills), [MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli), [plugin package](https://developers.openai.com/plugins/build/plugins) and [CLI commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli) pages were opened after official search. All are unversioned and cannot establish exact-release behavior. The [Claude plugin commands](https://code.claude.com/docs/en/plugins/cli-reference), [OpenCode native plugin documentation](https://docs.opencode.ai/docs/plugins/) and [Qwen extensions documentation](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/) support distinct vendor concepts; full direct citation/applicability records are in the fixture. Qwen project activation is not a project installation directory.

Actual selected verification:

```text
/usr/bin/python3 /fast/zap-managed/2026-10-15-m08a2/verify-m08a2.py /fast/git/v/vibevm-next/fixtures/next/adapters/four-client-contracts.json
exit 0; clients 4; rows 33; sources 26; proof_bindings 396; scenario_groups 12
old_PROP054_children_preserved 23; legacy_changes []; runtime_support_qualified false
```

Scoped `git diff --check` exited 0. The independent finite author check verified all 66 example digests and unchanged named source owners. See [report.json](report.json) for exact argv/exits/counts and [observations.json](observations.json) for read digests and source observations. The initial metadata run also passed; final verification followed semantic metadata edits. The verifier itself was not modified.

No Rust tests ran: legacy consumed fixtures are unchanged, so A1 WorkAccepted2029 and its 11 characterization cases are retained without replay. Rust formatting is not applicable. Whole-project and exhaustive-facts checks were not triggered by this additive non-fact documentation section; every existing XML subtree, attribute, direct text and status remains intact. No source, manifest, task, generated state or Git mutation occurred.

Every future proof records seeded roots/foreign neighbors, operation sequence, exact row-byte/value bindings, receipt/refusal, oracle, owner and environment. Tests cover write-free plan versus preparation, trust/grants, apply/repeat/update, digest/drift, checkpoint recovery, actual before-image inverse, cleanup after revocation/removal, foreign/cross-family equal bytes, malformed/overlap/escape/links and unsupported versions/platform/scopes/shapes. Actual client discovery/update/removal is a distinct unexecuted gate; where a no-provider route is not established, the case explicitly refuses to substitute presence evidence.

Unresolved scope remains visible: four missing exact versions; unversioned release applicability; exact client-managed plugin cache/activation paths; complete private user-root isolation and safe no-provider catalogue/effective-config observation. Existing plugin update still requires undeploy then deploy, and nonreversible skill updates do not gain rollback from these documents. No future case was executed or counted as passed. These debts must be resolved before runtime qualification, without installing software under this task.

M-06-C.2, the standalone interactive VibeVM boot inspector, remains mandatory future work. VibeVM owns projection/context, Zap owns tasks/review/acceptance, and PodBay owns physical process execution. Only Root accepts this candidate. Counts remain unchanged.
