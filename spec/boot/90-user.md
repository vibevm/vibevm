# User overrides

User-owned boot snippet. `vibe install`/`uninstall` never touches this file. Add any project-specific conventions that should be read at session boot.

## Communication style

Общение на русском языке в «айтишном» регистре: технические термины, кальки с английского и заимствования оставлять как есть (commit, install, registry, lockfile, build, stack, feat, flow, workflow, DAG, lifecycle и т.п.), не переводить их в стиле словаря Даля. Имена файлов, CLI-команды, код и термины из спеки — всегда в оригинале. Обычные слова — по-русски. Если есть устоявшийся русский термин (например, «зависимость» для dependency), можно использовать, но не насильно.

## Repository access

- **vibevm source repo (this repository):** `git@gitverse.ru:anarchic/vibevm.git`. Web: `https://gitverse.ru/anarchic/vibevm`.
- **Package registry (future, M1+):** `git@gitverse.ru:anarchic/vibespecs.git`. The local `packages/` tree in this repo will migrate to that registry when M1 lands.
- **SSH:** ключ для GitVerse настроен в Git Bash на этой машине под именем `olegchir@UNIT-2040`. `ssh -T git@gitverse.ru` подтверждает auth без shell-доступа (ожидаемо).
- **Proven commands on this machine:**
  - Clone (verified): `git clone git@gitverse.ru:anarchic/vibevm.git`.
  - First push (verified on 2026-04-17 against a fresh empty repo): `git push -u origin main`. Git Bash picks up the GitVerse SSH key automatically; no agent-forwarding needed.
  - Routine push: `git push origin main`. Force-push and history rewrite are NOT done without owner approval — see the Rule 4 list in `CLAUDE.md`.
