# Claude Code 2.1.329 native Messages bearer settings

## Explicit supported authentication

The public native setup route admits `--agent claude` only with the explicitly typed connection API/auth kind `anthropic-messages-bearer`. The credential type is never inferred from the endpoint URL. OpenAI Chat Completions and Responses remain incompatible and refuse without native writes. This does not create protocol translation or admit an untested x-api-key alternative.

The fixed first-slice destination is `HOME/.claude/settings.json` under an existing private Unix directory; explicit `CLAUDE_CONFIG_DIR` refuses instead of silently writing an unused default. The native settings `env` object persists `ANTHROPIC_BASE_URL` and `ANTHROPIC_AUTH_TOKEN`, plus the selected native model. This is an agent-supported settings mechanism, not Vibe launcher credential injection. Other settings/env entries survive. `apiKeyHelper`, incompatible explicit login settings, API-key/custom-header/cloud-provider/OAuth-token env auth overrides refuse rather than execute or create ambiguous authorization. Existing OAuth files are not read or rewritten by the sink. Native credential scope is not expanded beyond this supported bearer case.

Official Claude Code documentation, `llm-gateway-connect`, sections “Set in a settings file” and “How the credential variable maps to a header”, describes user settings persistence and the bearer-vs-x-api-key distinction. That documentation alone is not pinned 2.1.329 behavior proof; actual native requests below establish the selected implementation.

## Executed evidence

Private evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `claude21329-native-config-r1/receipt.json`: direct unchanged official native 2.1.329 reads synthetic user settings and sends one `/v1/messages?beta=true` with matching model/credential. Exit 0; no real account, provider/model/key arguments or credential launch environment. Architect independently confirmed this native reader result.
- `claude21329-public-native-r1/receipt.json`: actual public keyring create/connection add/native preview/apply/no-op followed by direct unchanged native Claude. One matching Messages request; adjacent native settings/env and synthetic OAuth file preserved, both OpenAI API kinds refused unchanged, helper refused without execution. Exit 0. Architect independently accepted this public behavior.
- Immutable CLI snapshot `claude21329-public-native-r1/vibe` SHA-256 `744ffa0bdae390e49b0a8d553d35b110a62effb12519f655d4d0f3dc48ef574b`; native receipt SHA-256 `a7d0590fad22d895359df7439bc9b455b921e9eaf10e7d6532b8ff76a925f6bb`.
- `claude-native-clean-build-r1/vibe_secrets-runtime.log`: complete storage library **27/27 PASS**, no ignored/filtered.
- `claude-native-clean-final-build-r2/runtime.log`: complete public AI connections target **20/20 PASS**, no ignored/filtered, including the durable Claude bearer/OAuth/OpenAI-refusal regression and all previous native adapter cases.
- `claude-native-public-pty-r1/receipt.json`: Escape, default Keep and explicit Apply **3/3 PASS**, terminal state restored, no provider calls or secret output.

All runtime gates use enforced filesystem/PID/network isolation, synthetic HOME, selected read-only supplier/CLI artifacts and a local synthetic server. No ordinary harness settings/auth stores, private corporate profile, real account or credential is mounted. The clean compile excludes stopped owner-file preparation through explicit accepted-source projections.

`claude-native-clean-strict-r1` completed all targets of the three affected crates with warnings denied, **exit 0**. Exact final source/blob acceptance remains pending. No adapter commit or six-agent delivery is claimed yet.

## Remaining and limitations

Existing private Unix directory only; fresh-home genesis, native Windows ACL/security/runtime qualification, full multi-agent onboarding and the complete inspect/capabilities UI remain unfinished. Snapshot rechecks and atomic native replacement coordinate Vibe writers but are not a compare-and-swap against an independent editor. Preserved synthetic OAuth bytes do not establish every real-account coexistence scenario. Accepted source/native setup behavior is not a rebuilt distribution, corporate kit update or complete CORP/NST acceptance. No external push/publication occurred.
