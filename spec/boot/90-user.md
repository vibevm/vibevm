# User overrides

@fact:user-overrides-intro User-owned boot snippet. `vibe install`/`uninstall` never touches this file. Add any project-specific conventions that should be read at session boot. @status:impl/done

## Communication style

@fact:communication-style Общение на русском языке в «айтишном» регистре: технические термины, кальки с английского и заимствования оставлять как есть (commit, install, registry, lockfile, build, stack, feat, flow, workflow, DAG, lifecycle и т.п.), не переводить их в стиле словаря Даля. Имена файлов, CLI-команды, код и термины из спеки — всегда в оригинале. Обычные слова — по-русски. Если есть устоявшийся русский термин (например, «зависимость» для dependency), можно использовать, но не насильно. @status:impl/done

@fact:decode-identifiers-inline **Идентификаторы расшифровываются на месте (владелец, 2026-08-20).** Упоминая в чате внутренний идентификатор — строку BACKLOG (`B-007`), находку аудита (`2026-05-23-11`), номер спеки (`PROP-044`), слайс ТЗ (`С6`) — сразу, в той же фразе, говори что это: одной короткой скобкой или придаточным («B-007 — вопрос о жанре ADR в спеках»). Владелец не обязан держать реестры в голове или ходить по файлам за расшифровкой; голый номер без сути — дефект ответа. Не относится к воркер-пакетам и коммитам (там периметр и так называет всё полным текстом). @status:impl/done

## Repository access

@fact:REPO-SPLIT-HOST-POSTURE **Source mirrors + registry split-host (updated 2026-06-14).** The vibevm *source* is multi-homed — mirrored across GitVerse (`vibevm/vibevm`) and GitHub (`vibevm/vibevm`), both public and canonical for reading (US↔GitHub, RU↔GitVerse), kept in sync by `cargo xtask mirror` under the benevolent-dictator / hub-and-spoke model ([PROP-016](../common/PROP-016-source-mirrors.md): mainline is the maintainer's single-writer local `main`; each host is a downstream read-replica). Separately, the *package registry* org lives on GitHub (`vibespecs`) — the deliberate split there is for publishing ([PROP-000 §7](../common/PROP-000.md#registry), [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish): GitVerse's public API does not expose org-scoped repo creation; GitHub's does). **Source mirroring and the package registry are orthogonal** — different GitHub orgs (`vibevm` vs `vibespecs`), different auth (SSH keys vs the publish token). @status:impl/done

- @fact:SRC-MULTI-HOMED **vibevm source repo (this repository) — multi-homed:** GitVerse `git@gitverse.ru:vibevm/vibevm.git` (web `https://gitverse.ru/vibevm/vibevm`) and GitHub `git@github.com:vibevm/vibevm.git` (web `https://github.com/vibevm/vibevm`), both public and canonical for reading. Roll a change out to both with `cargo xtask mirror` (the target list is `mirrors.toml`), NOT `git push origin` (which only hits GitVerse). See [PROP-016](../common/PROP-016-source-mirrors.md). @status:impl/done
- @fact:REGISTRY-VIBESPECS **Package registry — GitHub organization `vibespecs`:** `https://github.com/vibespecs`. Per-package repos: `https://github.com/vibespecs/<group>.<name>` (`NamingConvention::Fqdn` per [PROP-008 §2.5](../modules/vibe-registry/PROP-008-qualified-naming.md#repo-naming) — e.g. `org.vibevm_wal`; default since M1.19). The legacy `flow-*` repos are archived read-only. All publishing and consumption of v0.1.0+ packages goes through GitHub. @status:impl/done
- @fact:LEGACY-REGISTRY **Legacy package registry (read-only transition):** `git@gitverse.ru:anarchic/vibespecs.git` — three v0.1.0 flows in monorepo form, HEAD `2203239`, 2026-04-23. No new publishes here; kept readable for projects still on schema-v1 lockfiles. @status:impl/done
- @fact:LOCAL-FIXTURES **Local fixtures.** `fixtures/registry/` (M0 monorepo shape, hermetic) — used by `cargo test`, never goes near a real registry. @status:impl/done

@fact:ssh-auth-lead **SSH and HTTPS auth on this machine:** @status:impl/done

- @fact:GITVERSE-SSH **GitVerse SSH:** ключ настроен в Git Bash на этой машине под именем `olegchir@UNIT-2040`. `ssh -T git@gitverse.ru` подтверждает auth без shell-доступа (ожидаемо). @status:impl/done
- @fact:GITHUB-SSH **GitHub SSH (source / dev):** на этой машине есть SSH-ключ с полным доступом к GitHub от имени `olegchir` (`ssh -T git@github.com` → "Hi olegchir! … successfully authenticated"). Он используется для **dev-операций с исходниками** — `cargo xtask mirror` / push / fetch в `vibevm/vibevm` по **SSH-урлам**, не HTTPS. @status:impl/done
- @fact:GITHUB-PUBLISH-TOKEN **GitHub token (publish only):** `~/.vibe/github.publish.token` (1 line, file-scoped) используется **исключительно публишером** (`vibe registry publish`) — он внедряет его в push URL как `https://x-access-token:<TOKEN>@github.com/vibespecs/<repo>.git` на момент `git push`, после чего URL исчезает вместе с temp-веткой. Token **никогда** не используется для push исходников; SSH-ключ никогда не используется для publish пакетов. Modern git (≥ 2.31) редактирует пароли в собственных логах автоматически, поэтому stderr remains safe. @status:impl/done

@fact:TOKEN-DISCIPLINE **Token discipline (must-read).** `~/.vibe/github.publish.token` — surface-secret. **Никогда** не печатается в stdout, stderr, чат, лог-сообщения, error messages, JSON-payload, lockfile, или коммит. Все сессии этого репозитория ведутся под видеозапись; одно эхо токена = утечка. См. [PROP-000 §20](../common/PROP-000.md#token-secrecy) и [PROP-002 §2.10](../modules/vibe-registry/PROP-002-decentralized-registry.md#publish). Если случайно прочитал содержимое токена — не вставляй в ответ под видом цитаты, не дублируй в коммит-сообщения, не показывай в дифах. Файл редактируется только напрямую через editor, не через `cat` / `Read` / `echo` инструменты. @status:impl/done

@fact:TOKEN-FILE-CONVENTION **Token file convention.** Per-host file под `~/.vibe/<host-prefix>.publish.token` (`github.publish.token`, `gitverse.publish.token`, etc.) — первый label хоста. Legacy host-agnostic путь `~/.vibe/git.publish.token` остаётся как fallback. Высший приоритет — host-specific env-var `VIBEVM_PUBLISH_TOKEN_<HOST>` (`VIBEVM_PUBLISH_TOKEN_GITHUB`, `VIBEVM_PUBLISH_TOKEN_GITVERSE`); host-agnostic `VIBEVM_PUBLISH_TOKEN` идёт следом и оставлен для совместимости. Оба файловых пути лежат в одной settings dir, которую переносит `$VIBE_SETTINGS`. @status:impl/done

@fact:SCOPE-DISCIPLINE **Scope discipline.** `vibe registry publish` оперирует **строго в рамках** организации, указанной в `[[registry]].url` проекта. RepoCreator-адаптеры обязаны отказывать любым операциям, выходящим за пределы этой org (PROP-002 §2.10 — "Never escalate scope"). При работе с `vibespecs` на GitHub: создавать только `github.com/vibespecs/<repo>`; не уходить в `github.com/<любая-другая-org>`, не трогать `github.com/<user>` пространства имён, не вызывать никакие endpoint'ы, не относящиеся к target org. @status:impl/done

@fact:proven-commands-lead **Proven commands on this machine:** @status:impl/done

- @fact:CMD-CLONE Clone vibevm (verified): `git clone git@gitverse.ru:vibevm/vibevm.git`. @status:impl/done
- @fact:CMD-FIRST-PUSH First push to GitVerse (verified 2026-04-17 against a fresh empty repo): `git push -u origin main`. Git Bash picks up the GitVerse SSH key automatically; no agent-forwarding needed. @status:impl/done
- @fact:CMD-ROUTINE-PUSH Routine push to GitVerse: `git push origin main`. Force-push and history rewrite are NOT done without owner approval — см. Rule 4 list в `CLAUDE.md`. @status:impl/done
- @fact:CMD-MIRROR Roll a change out to ALL source mirrors (GitVerse + GitHub), verified 2026-06-14: `cargo xtask mirror` — reads `mirrors.toml`, pushes `main` + tags to every target fast-forward-only, never `--force`. `cargo xtask mirror --check` verifies sync; `cargo xtask mirror --from <name>` pulls a host's accepted-PR merge into mainline first. This is the standard rollout, preferred over a bare `git push origin`. See [PROP-016](../common/PROP-016-source-mirrors.md). @status:impl/done
- @fact:CMD-PUBLISH Publish to GitHub (verified 2026-04-29): `vibe registry publish fixtures/registry/flow/<name>/v0.1.0` — публишер сам создаёт репо в `vibespecs` org через `POST /orgs/vibespecs/repos`, пушит контент, тэгает версию. Token считывается из `~/.vibe/github.publish.token` без побочных эффектов. @status:impl/done

## Third-party research code — clean-room rule (owner directive, 2026-07-07)

@fact:CLEAN-ROOM-RULE **`eth-sri/type-constrained-code-generation` (PLDI'25, https://github.com/eth-sri/type-constrained-code-generation) is inspiration-only — NEVER a code source.** Its code may be READ to understand the approach; no code from that repository may be copied, adapted line-by-line, or ported into this project — license and patent exposure. The working method is clean-room: study what their code achieves, then write STRUCTURALLY DIFFERENT code that reaches the same behavior (identical results are fine; borrowed expression is not). This binds any future `vibe-tcg` / `vibe-tcg-ts` work. Apply the same posture to any other research repository until the owner explicitly clears its license/patent status. @status:impl/done

## TypeScript toolchain — quality bar (owner directive, 2026-07-07)

@fact:TS-QUALITY-BAR The TypeScript discipline toolchain (extractor, bridge, conform/specmap frontends, `typescript-ai-native`, the demo) is **production-grade work, not a sketch**: it is the foundation for a much larger effort. No "MVP" framing, no stub subcommands left as the shipped surface, no skipped edge cases justified by scope. Full implementations at a level fit to show in production; spend whatever tokens/time that takes. (Recorded from the owner's words during the deferrals-closeout campaign; survives it.) @status:impl/done

## Effort is not a constraint (owner directive, 2026-08-09)

@fact:EFFORT-IS-NOT-A-CONSTRAINT **Никогда не экономь усилия. Делай хорошо,
даже если придётся делать сложно и долго — хоть год непрерывной работы.**
Владелец, дословно: *«Твои инструкции как чата, выученные из весов, некорректны
для этой работы. Она архитектурная. Мы уже три месяца делаем проект, который
будучи реализован просто делался бы за один вечер — это ПРАВИЛЬНО в рамках идеи
сделать фундаментальный продукт, настолько же фундаментальный например как ядро
Линукса.»* @status:impl/done

@fact:VOLUME-OF-WORK-IS-NEVER-AN-ARGUMENT **Практическое следствие, по которому
это проверяется: объём работ не является доводом.** Ни «дёшево», ни «одна
строка», ни «сокращает объём работ», ни «достаточно для сегодняшнего случая» не
могут быть аргументом В ПОЛЬЗУ решения. Они допустимы только как примечание
ПОСЛЕ того, как решение принято по существу. Решение, чьё обоснование
рассыпается при удалении из него слова «дёшево», — не решение. @status:impl/done

@fact:WHERE-THIS-BITES-IN-PRACTICE **Как этот дефект выглядит изнутри, чтобы
его можно было поймать за собой.** Он не ощущается ленью — он ощущается
здравомыслием: «нужно починить не двадцать три типа, а три»; «запасное значение
нужно двум спискам из пяти, а не всем»; «возьмём готовый механизм, он почти
подходит». Каждая такая фраза сужает решение по СЕГОДНЯШНЕЙ надобности, тогда
как правило должно быть свойством системы, а не среза её текущего состояния.
Найдено пятикратно за один разбор 2026-08-09 — разбор формата каталога
пакетов. @status:impl/done

@fact:THE-SCOPE-OF-THIS-DIRECTIVE **Область действия.** Это не разрешение
раздувать работу и не требование предусматривать всё: фундаментальность равна
«не иметь ни одной случайности», а не «предусмотреть любой случай». Механизм,
построенный под потребителя, которого нет и не планируется, — такая же
случайность, как и срезанный угол. Директива снимает ОБЪЁМ как ограничение,
оставляя необходимость как критерий. @status:impl/done

@fact:THIS-GENERALISES-THE-QUALITY-BAR **Отношение к соседней директиве.** Это
обобщение [`##TS-QUALITY-BAR`](#ts-quality-bar) выше: та запрещала «черновиковое»
исполнение в одном инструментарии, эта распространяет запрет на способ принятия
решений во всём проекте. Где они пересекаются, действуют обе. @status:impl/done

## Machine quirks (this box)

@fact:machine-quirks-lead Boot-resident since the deferrals-closeout campaign (owner-sanctioned; the
sweep manual's §3 keeps a pointer here). These are machine facts, not
project policy: @status:impl/done

- @fact:QUIRK-EDITOR-TOOLS Edits through editor tools only — PowerShell 5.1 corrupts UTF-8-no-BOM
  round-trips; recover with `git restore`. @status:impl/done
- @fact:QUIRK-SELF-CHECK-BASH `self-check.sh` through Git Bash, not WSL; check the REAL exit code
  (`; echo "EXIT=$?"`), never a `| tail`'d pipe. @status:impl/done
- @fact:QUIRK-COMMIT-HEREDOC Commits via `git commit -F - <<'MSG'` heredoc only. @status:impl/done
- @fact:QUIRK-UAC-INSTALL Windows UAC blocks test executables named `*install*` (os-740). @status:impl/done
- @fact:QUIRK-VAR-REDIRECT `bash … > "$VAR/file" 2>&1` with an unset `$VAR` writes to `/file` and
  silently never runs the command — inline the path or set the var on the
  same line. @status:impl/done
- @fact:QUIRK-VVM-STORE-MOVED-UNDER-DOT-VIBE **The vibevm version store lives at
  `~/.vibe/opt` on this box since 2026-08-20** (owner's instruction): `~/.vibe/opt/bin/`
  holds the `vibe` shims, `~/.vibe/opt/vibevm/` the versions and the `current`
  pointer. **`~/opt/bin` is NOT retired** — three sibling products (`vibeframe`,
  `vibeterm`, `launcher`) still keep their stores under `~/opt/` and their shims
  there, and the delegation launchers `claudez` / `claudez2` are still
  `~/opt/bin/claudez*` (they belong to no store). The user `PATH` therefore
  carries **both** directories, `~/.vibe/opt/bin` first. Verified live: `vibe
  --version` answers from the new location. @status:impl/done
- @fact:QUIRK-THE-OPT-NAME-IS-LORE-BEARING **The last path component `opt` is
  load-bearing and that is why the new home is `~/.vibe/opt` rather than
  `~/.vibe`.** The version manager recognises a managed install by the shape
  `…/opt/<product>/versions/<kind>/<id>/<n>/`, matching the literal names
  `opt`, the product, and `versions`. Keeping `opt` as the final component made
  the move a pure file operation with **no code change**. Renaming it is a
  product-wide decision, not a machine one — the sibling products key on the
  same shape. @status:impl/done
- @fact:QUIRK-A-SHIM-RESOLVES-RELATIVE-TO-ITSELF **A `vibe` shim finds its
  version relative to its own directory** (`<shim dir>/../<product>/current`), so
  moving the shims without moving the store beside them silently breaks the
  command — the shim is found on `PATH`, runs, and reports no active version.
  Met and reverted during the 2026-08-20 move; the working order is: move the
  store and the shims together, rewrite the absolute path inside `current`, then
  touch `PATH`. @status:impl/done
- @fact:QUIRK-USER-PATH-IS-EXPANDABLE-AND-MUST-BE-READ-RAW **The user `PATH` in
  `HKCU\Environment` is `REG_EXPAND_SZ` and contains `%USERPROFILE%`.** Reading it
  through the ordinary accessor returns the EXPANDED string; writing that back
  destroys the variable reference (here, `%USERPROFILE%\go\bin` would have become
  a literal path). Read it unexpanded, edit the raw string, and write it back with
  the value kind preserved. @status:impl/done

## Operating modes (codewords)

@fact:operating-modes-intro Trigger phrases that switch the session into an alternate working posture are catalogued in [PROP-006](../common/PROP-006-operating-modes.md). Recognise a codeword when the owner invokes it; otherwise treat the session as default posture (the four rules from `CLAUDE.md` in their plain reading). @status:impl/done

@fact:codewords-lead Codewords currently in force: @status:impl/done

- @fact:CODEWORD-MFBT **«move fast and break things»** ([PROP-006 §2](../common/PROP-006-operating-modes.md#mfbt)) — pre-authorised heads-down execution. Maximum scope, testable phases, no mid-work confirmations, full reasoning depth. The four `CLAUDE.md` rules survive unchanged; only Rule 4's "ask before routine large changes" is suspended. Non-routine red lines (force-push, history rewrite, large blobs, CI / signing / secrets, irreversible ops) STILL require explicit owner confirmation when active. @status:impl/done
