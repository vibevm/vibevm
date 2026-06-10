# User overrides

User-owned boot snippet. `vibe install`/`uninstall` never touches this file. Add any project-specific conventions that should be read at session boot.

## Communication style

Общение на русском языке в «айтишном» регистре: технические термины, кальки с английского и заимствования оставлять как есть (commit, install, registry, lockfile, build, stack, feat, flow, workflow, DAG, lifecycle и т.п.), не переводить их в стиле словаря Даля. Имена файлов, CLI-команды, код и термины из спеки — всегда в оригинале. Обычные слова — по-русски. Если есть устоявшийся русский термин (например, «зависимость» для dependency), можно использовать, но не насильно.

## Repository access

**Split-host posture (2026-04-29).** vibevm project source lives on GitVerse, the package registry organization lives on GitHub. The split is deliberate — see [PROP-000 §7](../common/PROP-000.md#registry) and [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish) for the full rationale (GitVerse's public API does not expose org-scoped repo creation; GitHub's does).

- **vibevm source repo (this repository) — GitVerse:** `git@gitverse.ru:anarchic/vibevm.git`. Web: `https://gitverse.ru/anarchic/vibevm`. **Stays on GitVerse.** Не мигрируется.
- **Package registry — GitHub organization `vibespecs`:** `https://github.com/vibespecs`. Per-package repos: `https://github.com/vibespecs/<group>.<name>` (`NamingConvention::Fqdn` per [PROP-008 §2.5](../modules/vibe-registry/PROP-008-qualified-naming.md#repo-naming) — e.g. `org.vibevm.wal`; default since M1.19). The legacy `flow-*` repos are archived read-only. All publishing and consumption of v0.1.0+ packages goes through GitHub.
- **Legacy package registry (read-only transition):** `git@gitverse.ru:anarchic/vibespecs.git` — three v0.1.0 flows in monorepo form, HEAD `2203239`, 2026-04-23. No new publishes here; kept readable for projects still on schema-v1 lockfiles.
- **Local fixtures.** `fixtures/registry/` (M0 monorepo shape, hermetic) — used by `cargo test`, never goes near a real registry.

**SSH and HTTPS auth on this machine:**

- **GitVerse SSH:** ключ настроен в Git Bash на этой машине под именем `olegchir@UNIT-2040`. `ssh -T git@gitverse.ru` подтверждает auth без shell-доступа (ожидаемо).
- **GitHub HTTPS — token-only.** SSH-ключа для GitHub на этой машине нет; auth идёт через personal access token в `~/.vibevm/github.publish.token` (1 line, file-scoped). Token используется только публишером — он внедряет его в push URL как `https://x-access-token:<TOKEN>@github.com/vibespecs/<repo>.git` на момент `git push`, после чего URL исчезает вместе с temp-веткой. Modern git (≥ 2.31) редактирует пароли в собственных логах автоматически, поэтому stderr remains safe.

**Token discipline (must-read).** `~/.vibevm/github.publish.token` — surface-secret. **Никогда** не печатается в stdout, stderr, чат, лог-сообщения, error messages, JSON-payload, lockfile, или коммит. Все сессии этого репозитория ведутся под видеозапись; одно эхо токена = утечка. См. [PROP-000 §20](../common/PROP-000.md#token-secrecy) и [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish). Если случайно прочитал содержимое токена — не вставляй в ответ под видом цитаты, не дублируй в коммит-сообщения, не показывай в дифах. Файл редактируется только напрямую через editor, не через `cat` / `Read` / `echo` инструменты.

**Token file convention.** Per-host file под `~/.vibevm/<host-prefix>.publish.token` (`github.publish.token`, `gitverse.publish.token`, etc.) — первый label хоста. Legacy host-agnostic путь `~/.vibevm/git.publish.token` остаётся как fallback. Env-var `VIBEVM_PUBLISH_TOKEN` — высший приоритет, для CI.

**Scope discipline.** `vibe registry publish` оперирует **строго в рамках** организации, указанной в `[[registry]].url` проекта. RepoCreator-адаптеры обязаны отказывать любым операциям, выходящим за пределы этой org (PROP-002 §2.10 — "Never escalate scope"). При работе с `vibespecs` на GitHub: создавать только `github.com/vibespecs/<repo>`; не уходить в `github.com/<любая-другая-org>`, не трогать `github.com/<user>` пространства имён, не вызывать никакие endpoint'ы, не относящиеся к target org.

**Proven commands on this machine:**

- Clone vibevm (verified): `git clone git@gitverse.ru:anarchic/vibevm.git`.
- First push to GitVerse (verified 2026-04-17 against a fresh empty repo): `git push -u origin main`. Git Bash picks up the GitVerse SSH key automatically; no agent-forwarding needed.
- Routine push to GitVerse: `git push origin main`. Force-push and history rewrite are NOT done without owner approval — см. Rule 4 list в `CLAUDE.md`.
- Publish to GitHub (verified 2026-04-29): `vibe registry publish fixtures/registry/flow/<name>/v0.1.0` — публишер сам создаёт репо в `vibespecs` org через `POST /orgs/vibespecs/repos`, пушит контент, тэгает версию. Token считывается из `~/.vibevm/github.publish.token` без побочных эффектов.

## Operating modes (codewords)

Trigger phrases that switch the session into an alternate working posture are catalogued in [PROP-006](../common/PROP-006-operating-modes.md). Recognise a codeword when the owner invokes it; otherwise treat the session as default posture (the four rules from `CLAUDE.md` in their plain reading).

Codewords currently in force:

- **«move fast and break things»** ([PROP-006 §2](../common/PROP-006-operating-modes.md#mfbt)) — pre-authorised heads-down execution. Maximum scope, testable phases, no mid-work confirmations, full reasoning depth. The four `CLAUDE.md` rules survive unchanged; only Rule 4's "ask before routine large changes" is suspended. Non-routine red lines (force-push, history rewrite, large blobs, CI / signing / secrets, irreversible ops) STILL require explicit owner confirmation when active.
