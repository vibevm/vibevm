# Hermes/Pi — current upstream evidence and architecture decisions

Genre: bounded primary-source integration research. Access date2026-11-19. Status: architecture preparation, not implementation acceptance. URLs below are source pointers; moving main references must be pinned to exact commits before implementation packet acceptance. No agent installation, provider call or credential access was performed.

## Pinned primary-source snapshots

GitHub commits/main API observed2026-11-19: Hermes `4ae451d4126327c871fd084e60c045dec0c85911`; Pi `942842439e1f1d65f68a61857ddb7590b80c5a92`. Hermes pinned `_parser.py` and main.py inspected directly: native flags include query/query-file, resume/workspace binding and ignore-rules; no additive system-prompt option was established by that parser inspection. This is not proof that no native extension/config mechanism exists. Next narrow read is prompt-construction/config ownership, not guessing a CLI flag or adopting broad ignore-rules. Pi direct source and docs must be rechecked at the pinned commit before final packet.

## Pi: native additive delivery, not identity replacement

Primary sources:
- https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md
- https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/configuration.md
- https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/resource-loader.ts
- https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts
- https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts

Current CLI documentation inspected: `--append-system-prompt <text|path>` accepts file content additively and is repeatable; `--system-prompt` replaces the upstream base. `--no-context-files` disables AGENTS/CLAUDE discovery. `--print` is one-shot text; JSON and RPC reserve stdout for their protocol. CWD determines resource discovery and sessions. Do not use a user prompt @file as a substitute for native instruction delivery, and do not replace Pi's base prompt merely to add VibeVM instructions.

Architecture decision: use native additive file delivery through the existing VibeVM harness adapter/composer. Determine deduplication precisely against actual upstream loader, preserve unrelated project/user context and native agent resources, and validate the composed project prefix plus fresh USER-PROJECT late layers. A broad discovery-disable flag is not automatically correct: it could erase unrelated authored instructions. A provider-free request-capture or native prompt-construction proof must establish actual placement/order before live acceptance.

## Hermes: context discovery has meaningful precedence

Primary sources:
- https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/context-files.md
- https://github.com/NousResearch/hermes-agent/blob/main/hermes_cli/main.py

Context-file documentation inspected: one project context type wins in priority `.hermes.md` / HERMES.md, AGENTS.override.md, AGENTS.md, CLAUDE.md, then cursor rules. AGENTS.override replaces the neighboring AGENTS document; SOUL is independent instance-wide identity under HERMES_HOME. Git-root-to-CWD chains and progressive subdirectory discovery matter for root binding, deduplication and nested-project exclusion.

Actual main.py inspected: `--ignore-rules` skips project rules, SOUL, memory and preloaded skills; it is not a narrow VibeVM duplicate-prevention switch. Do not select it casually or claim native memory/skills preserved when they are disabled. Need the actual CLI argument owner and prompt-construction owner to settle the narrow additive carrier; do not invent a flag from analogy with Claude/Pi.

Architecture constraint: preserve Hermes's base identity and unrelated user/agent resources; keep VibeVM personal project layers separate from native Hermes memory rather than replacing one with the other. Higher-priority native project instructions must not silently bypass VibeVM boot/USER-PROJECT and still be advertised as supported.

## Existing VibeVM owners inspected

- `crates/vibe-workspace/src/system_prompt/harness.rs`: closed HarnessKind set currently Claude/Codex/OpenCode/Qwen, shared launch_flags and read/launch decision. Add first-class capabilities here rather than a second launcher/composer.
- `crates/vibe-core/src/manifest/package/system_prompt/harness.rs`: manifest-facing per-harness delivery selection and admission, distinct from executable overrides.
- `crates/vibe-agent-projection/src/agents.rs` and associated path/config adapters: separate MCP/skill projection registry; full agent support must evaluate these supported upstream capabilities instead of assuming launch registration covers them.
- `crates/vibe-app/src/agents/presets.rs`: unattended/interactive application agent-step presets; assess native supported modes and truthful unsupported behavior.

## Additional pinned capability evidence

Pi pinned docs/skills.md confirms Agent Skills layout and supported generic `~/.agents/skills/` plus project `.agents/skills/` discovery up to repository root. Use an existing supported injected-home projection owner rather than inventing a Pi-specific global directory from historical package naming. Name collisions keep first discovered with warning; malformed skill/description is not silently available.

Hermes pinned context-files.md adds a concrete read-delivery admission concern: startup files are security-scanned and head/tail truncated at a model/config-dependent cap (defaultfloor20000, ceiling500000 chars); progressive hints have a separate32000-char cap. File targets outside the project boundary may be skipped. Native read integration must verify the managed router survives actual admission and direct model tool reads can reach the required project/personal files within the authorized environment. Do not disable upstream security scans as a workaround; a blocked/truncated router is an actionable unsupported-context outcome, not successful USER-PROJECT integration.

## Finite next architecture output

Pin exact upstream versions/source snapshots; verify Hermes native additive mechanism; map first-class invocation + instruction/skill/MCP surfaces against actual capabilities, then issue one bounded implementation brief with existing owners and end-to-end USER-PROJECT proof. Do not expand into provider infrastructure, credential migration, upstream code copying or indefinite harness testing. SYS-TAILS owns historical tail reconciliation; it must not duplicate this new integration implementation.
