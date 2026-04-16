# User overrides

User-owned boot snippet. `vibe install`/`uninstall` never touches this file. Add any project-specific conventions that should be read at session boot.

## Communication style

Общение на русском языке в «айтишном» регистре: технические термины, кальки с английского и заимствования оставлять как есть (commit, install, registry, lockfile, build, stack, feat, flow, workflow, DAG, lifecycle и т.п.), не переводить их в стиле словаря Даля. Имена файлов, CLI-команды, код и термины из спеки — всегда в оригинале. Обычные слова — по-русски. Если есть устоявшийся русский термин (например, «зависимость» для dependency), можно использовать, но не насильно.

## Repository access

- **vibevm source repo (this repository):** `git@gitverse.ru:anarchic/vibevm.git`. Web: `https://gitverse.ru/anarchic/vibevm`.
- **Package registry (future, M1+):** `git@gitverse.ru:anarchic/vibespecs.git`. The local `packages/` tree in this repo will migrate to that registry when M1 lands.
- **SSH:** ключ для GitVerse настроен в Git Bash на этой машине под именем `olegchir@UNIT-2040`. `ssh -T git@gitverse.ru` подтверждает auth без shell-доступа (ожидаемо).
- **Proven commands on this machine:**
  - Clone: `git clone git@gitverse.ru:anarchic/vibevm.git` (verified against the empty repo).
  - Push (routine): `git push origin <branch>` from this repo — main working branch is `main`.
