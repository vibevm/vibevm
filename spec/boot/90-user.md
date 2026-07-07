# User overrides

User-owned boot snippet. `vibe install`/`uninstall` never touches this file. Add any project-specific conventions that should be read at session boot.

## Communication style

Общение на русском языке в «айтишном» регистре: технические термины, кальки с английского и заимствования оставлять как есть (commit, install, registry, lockfile, build, stack, feat, flow, workflow, DAG, lifecycle и т.п.), не переводить их в стиле словаря Даля. Имена файлов, CLI-команды, код и термины из спеки — всегда в оригинале. Обычные слова — по-русски. Если есть устоявшийся русский термин (например, «зависимость» для dependency), можно использовать, но не насильно.

## Repository access

**Source mirrors + registry split-host (updated 2026-06-14).** The vibevm *source* is multi-homed — mirrored across GitVerse (`anarchic/vibevm`) and GitHub (`anarchic-pro/vibevm`), both public and canonical for reading (US↔GitHub, RU↔GitVerse), kept in sync by `cargo xtask mirror` under the benevolent-dictator / hub-and-spoke model ([PROP-016](../common/PROP-016-source-mirrors.md): mainline is the maintainer's single-writer local `main`; each host is a downstream read-replica). Separately, the *package registry* org lives on GitHub (`vibespecs`) — the deliberate split there is for publishing ([PROP-000 §7](../common/PROP-000.md#registry), [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish): GitVerse's public API does not expose org-scoped repo creation; GitHub's does). **Source mirroring and the package registry are orthogonal** — different GitHub orgs (`anarchic-pro` vs `vibespecs`), different auth (SSH keys vs the publish token).

- **vibevm source repo (this repository) — multi-homed:** GitVerse `git@gitverse.ru:anarchic/vibevm.git` (web `https://gitverse.ru/anarchic/vibevm`) and GitHub `git@github.com:anarchic-pro/vibevm.git` (web `https://github.com/anarchic-pro/vibevm`), both public and canonical for reading. Roll a change out to both with `cargo xtask mirror` (the target list is `mirrors.toml`), NOT `git push origin` (which only hits GitVerse). See [PROP-016](../common/PROP-016-source-mirrors.md).
- **Package registry — GitHub organization `vibespecs`:** `https://github.com/vibespecs`. Per-package repos: `https://github.com/vibespecs/<group>.<name>` (`NamingConvention::Fqdn` per [PROP-008 §2.5](../modules/vibe-registry/PROP-008-qualified-naming.md#repo-naming) — e.g. `org.vibevm.wal`; default since M1.19). The legacy `flow-*` repos are archived read-only. All publishing and consumption of v0.1.0+ packages goes through GitHub.
- **Legacy package registry (read-only transition):** `git@gitverse.ru:anarchic/vibespecs.git` — three v0.1.0 flows in monorepo form, HEAD `2203239`, 2026-04-23. No new publishes here; kept readable for projects still on schema-v1 lockfiles.
- **Local fixtures.** `fixtures/registry/` (M0 monorepo shape, hermetic) — used by `cargo test`, never goes near a real registry.

**SSH and HTTPS auth on this machine:**

- **GitVerse SSH:** ключ настроен в Git Bash на этой машине под именем `olegchir@UNIT-2040`. `ssh -T git@gitverse.ru` подтверждает auth без shell-доступа (ожидаемо).
- **GitHub SSH (source / dev):** на этой машине есть SSH-ключ с полным доступом к GitHub от имени `olegchir` (`ssh -T git@github.com` → "Hi olegchir! … successfully authenticated"). Он используется для **dev-операций с исходниками** — `cargo xtask mirror` / push / fetch в `anarchic-pro/vibevm` по **SSH-урлам**, не HTTPS.
- **GitHub token (publish only):** `~/.vibevm/github.publish.token` (1 line, file-scoped) используется **исключительно публишером** (`vibe registry publish`) — он внедряет его в push URL как `https://x-access-token:<TOKEN>@github.com/vibespecs/<repo>.git` на момент `git push`, после чего URL исчезает вместе с temp-веткой. Token **никогда** не используется для push исходников; SSH-ключ никогда не используется для publish пакетов. Modern git (≥ 2.31) редактирует пароли в собственных логах автоматически, поэтому stderr remains safe.

**Token discipline (must-read).** `~/.vibevm/github.publish.token` — surface-secret. **Никогда** не печатается в stdout, stderr, чат, лог-сообщения, error messages, JSON-payload, lockfile, или коммит. Все сессии этого репозитория ведутся под видеозапись; одно эхо токена = утечка. См. [PROP-000 §20](../common/PROP-000.md#token-secrecy) и [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish). Если случайно прочитал содержимое токена — не вставляй в ответ под видом цитаты, не дублируй в коммит-сообщения, не показывай в дифах. Файл редактируется только напрямую через editor, не через `cat` / `Read` / `echo` инструменты.

**Token file convention.** Per-host file под `~/.vibevm/<host-prefix>.publish.token` (`github.publish.token`, `gitverse.publish.token`, etc.) — первый label хоста. Legacy host-agnostic путь `~/.vibevm/git.publish.token` остаётся как fallback. Env-var `VIBEVM_PUBLISH_TOKEN` — высший приоритет, для CI.

**Scope discipline.** `vibe registry publish` оперирует **строго в рамках** организации, указанной в `[[registry]].url` проекта. RepoCreator-адаптеры обязаны отказывать любым операциям, выходящим за пределы этой org (PROP-002 §2.10 — "Never escalate scope"). При работе с `vibespecs` на GitHub: создавать только `github.com/vibespecs/<repo>`; не уходить в `github.com/<любая-другая-org>`, не трогать `github.com/<user>` пространства имён, не вызывать никакие endpoint'ы, не относящиеся к target org.

**Proven commands on this machine:**

- Clone vibevm (verified): `git clone git@gitverse.ru:anarchic/vibevm.git`.
- First push to GitVerse (verified 2026-04-17 against a fresh empty repo): `git push -u origin main`. Git Bash picks up the GitVerse SSH key automatically; no agent-forwarding needed.
- Routine push to GitVerse: `git push origin main`. Force-push and history rewrite are NOT done without owner approval — см. Rule 4 list в `CLAUDE.md`.
- Roll a change out to ALL source mirrors (GitVerse + GitHub), verified 2026-06-14: `cargo xtask mirror` — reads `mirrors.toml`, pushes `main` + tags to every target fast-forward-only, never `--force`. `cargo xtask mirror --check` verifies sync; `cargo xtask mirror --from <name>` pulls a host's accepted-PR merge into mainline first. This is the standard rollout, preferred over a bare `git push origin`. See [PROP-016](../common/PROP-016-source-mirrors.md).
- Publish to GitHub (verified 2026-04-29): `vibe registry publish fixtures/registry/flow/<name>/v0.1.0` — публишер сам создаёт репо в `vibespecs` org через `POST /orgs/vibespecs/repos`, пушит контент, тэгает версию. Token считывается из `~/.vibevm/github.publish.token` без побочных эффектов.

## Third-party research code — clean-room rule (owner directive, 2026-07-07)

**`eth-sri/type-constrained-code-generation` (PLDI'25, https://github.com/eth-sri/type-constrained-code-generation) is inspiration-only — NEVER a code source.** Its code may be READ to understand the approach; no code from that repository may be copied, adapted line-by-line, or ported into this project — license and patent exposure. The working method is clean-room: study what their code achieves, then write STRUCTURALLY DIFFERENT code that reaches the same behavior (identical results are fine; borrowed expression is not). This binds any future `vibe-tcg` / `vibe-tcg-ts` work. Apply the same posture to any other research repository until the owner explicitly clears its license/patent status.

## TypeScript toolchain — quality bar (owner directive, 2026-07-07)

The TypeScript discipline toolchain (extractor, bridge, conform/specmap frontends, `typescript-ai-native`, the demo) is **production-grade work, not a sketch**: it is the foundation for a much larger effort. No "MVP" framing, no stub subcommands left as the shipped surface, no skipped edge cases justified by scope. Full implementations at a level fit to show in production; spend whatever tokens/time that takes. (Recorded from the owner's words during the deferrals-closeout campaign; survives it.)

## Machine quirks (this box)

Boot-resident since the deferrals-closeout campaign (owner-sanctioned; the
sweep manual's §3 keeps a pointer here). These are machine facts, not
project policy:

- Edits through editor tools only — PowerShell 5.1 corrupts UTF-8-no-BOM
  round-trips; recover with `git restore`.
- `self-check.sh` through Git Bash, not WSL; check the REAL exit code
  (`; echo "EXIT=$?"`), never a `| tail`'d pipe.
- Commits via `git commit -F - <<'MSG'` heredoc only.
- Windows UAC blocks test executables named `*install*` (os-740).
- `bash … > "$VAR/file" 2>&1` with an unset `$VAR` writes to `/file` and
  silently never runs the command — inline the path or set the var on the
  same line.

## Operating modes (codewords)

Trigger phrases that switch the session into an alternate working posture are catalogued in [PROP-006](../common/PROP-006-operating-modes.md). Recognise a codeword when the owner invokes it; otherwise treat the session as default posture (the four rules from `CLAUDE.md` in their plain reading).

Codewords currently in force:

- **«move fast and break things»** ([PROP-006 §2](../common/PROP-006-operating-modes.md#mfbt)) — pre-authorised heads-down execution. Maximum scope, testable phases, no mid-work confirmations, full reasoning depth. The four `CLAUDE.md` rules survive unchanged; only Rule 4's "ask before routine large changes" is suspended. Non-routine red lines (force-push, history rewrite, large blobs, CI / signing / secrets, irreversible ops) STILL require explicit owner confirmation when active.
