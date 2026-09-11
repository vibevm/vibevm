# Примеры к снятию — очередь для воркера (P.3 → фикстуры) {#root}

<status stage="spec" state="work" comment="ведётся с 2026-09-12; каждая строка — example на странице, чей expect снимается с target/debug/vibe.exe в песочнице фикстуры; после снятия строка получает «снято» и хэш коммита"/>

Правило (PLAN P.3, R-07): `expect` — только реально снятый вывод отладочного
бинарника; дешёвая модель снимает по этому списку и кладёт в
`examples/<фикстура>/expect/<страница>-<id>.txt`, центральная сессия вставляет.
Нормализация — по `example.toml` фикстуры (A0.12): `<TMP>`, `<HOME>`, `<REPO>`,
слэши, `vibe <VERSION>`.

## Фикстуры {#fixtures}

| Фикстура | Что содержит | Как готовится |
|---|---|---|
| `none` | пустая песочница без проекта и без store | ничего |
| `empty` | пустая песочница; store прогрет пакетом `org.vibevm.world/wal@1.0.0` и его замыканием (офлайн-установка возможна) | `vibe cache add org.vibevm.world/wal` при подготовке фикстуры; `example.toml` объявляет `VIBE_OFFLINE=1` для команд установки |
| `hello-vibe-empty` | как `empty`, плюс проект `hello-vibe/` после `vibe init hello-vibe` без пакетов | `vibe init hello-vibe` |
| `hello-vibe` | как `hello-vibe-empty`, плюс установленный `org.vibevm.world/wal` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes --offline` |

## Очередь {#queue}

| Страница | id | Команда | Фикстура | Состояние |
|---|---|---|---|---|
| `start/what-vibevm-is` | `version` | `vibe --version` | `hello-vibe` | вставлено вручную (`vibe 1.0.0`), подтвердить снятием |
| `start/what-a-project-contains` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `start/install-vibe` | `windows-install` | `powershell -ExecutionPolicy Bypass -File .\install.ps1` | `none` | вне песочницы — снимается с дистрибутива при релизе; до этого `expect` пуст и пример помечается `when="os:windows"` |
| `start/install-vibe` | `version` | `vibe --version` | `hello-vibe` | вставлено вручную |
| `start/first-project` | `init` | `vibe init hello-vibe` | `empty` | ждёт |
| `start/first-project` | `install` | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes` | `hello-vibe-empty` | ждёт (офлайн из прогретого store) |
| `start/first-project` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `start/first-project` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `start/first-project` | `check` | `vibe check --path hello-vibe` | `hello-vibe` | ждёт |
| `model/two-trees` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `model/boot-lane` | `tree` | `vibe tree --plain --path hello-vibe` | `hello-vibe` | ждёт |
| `model/packages-and-kinds` | `list` | `vibe list --path hello-vibe` | `hello-vibe` | ждёт |
| `model/registries` | `registry-list` | `vibe registry list --path hello-vibe` | `hello-vibe` | ждёт |
| `model/lock-and-store` | `cache-path` | `vibe cache path` | `hello-vibe` | ждёт (нормализация `<HOME>`) |
| `model/versions` | `outdated` | `vibe outdated --path hello-vibe` | `hello-vibe` | ждёт (офлайн: ожидается сообщение об офлайне или пустой список — решить по снятию) |
