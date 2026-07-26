# vibe init редизайн — создание проектов, пакетов, групп

## Концепция

`vibe init` становится multi-form командой: project / package / group, с позиционными аргументами (вместо только `--path`). Все формы поддерживают интерактивные промпты (npm-style) и неинтерактивные флаги.

## Формы вызова (8 вариантов из ТЗ)

### Project (создаёт vibe.toml + структуру проекта)

| Вызов | Что делает |
|-------|------------|
| `vibe init projectname` | Создаёт `projectname/` (если нет), там проект. Имя проекта = basename пути. Без пакета. |
| `vibe init org.vibevm.apple projectname` | Создаёт `projectname/`, там проект + пакет `packages/org.vibevm.apple/main/v0.1.0/` с `link = "static"`. |
| `vibe init org.vibevm.apple` | В **текущей** директории проект + пакет `packages/org.vibevm.apple/main/v0.1.0/` static. |
| `vibe init org.vibevm.apple/orange` | В текущей директории проект + пакет `packages/org.vibevm.apple/orange/v0.1.0/` static. |

### Package (добавляет пакет в существующий проект)

| Вызов | Что делает |
|-------|------------|
| `vibe init package org.vibevm.apple/orange` | В текущей директории (корень проекта/пакета) создать пакет `packages/org.vibevm.apple/orange/v0.1.0/` dynamic. |
| `vibe init package org.vibevm.apple/orange directorypath` | В указанной директории создать пакет dynamic. |

### Group (добавляет группу — каталог packages/<group>/)

| Вызов | Что делает |
|-------|------------|
| `vibe init group org.vibevm.apple` | В текущей директории создать группу `packages/org.vibevm.apple/` (каталог). |
| `vibe init group org.vibevm.apple directorypath` | В указанной директории. |

## Разбор аргументов (dispatcher)

`vibe init` принимает 0–3 позиционных аргумента + опциональный подкомандный keyword `package` / `group`:

```
vibe init [package|group] [pkgref] [path]
```

Логика разбора:
1. Первый позиционный = `package` или `group`? → подкоманда создания пакета/группы.
2. Иначе: первый позиционный — это pkgref (содержит `/` или `.` → это group/name) или путь (без точек → путь)?
   - Содержит `.` и `/` → `org.vibevm.apple/orange` (pkgref) → проект с пакетом в CWD.
   - Содержит `.` но не `/` → `org.vibevm.apple` (group only) → проект с пакетом `main` в CWD.
   - Не содержит `.` → путь. Если есть второй позиционный → второй = pkgref → проект с пакетом по пути.
3. Второй/третий позиционный — путь (если есть).

## Интерактивные промпты (npm-style)

При создании проекта/пакета в TTY-режиме:

1. **Package kind** — `Select`: flow / feat / stack / tool / mcp (по умолчанию `tool`).
2. **Version** — `Input`: по умолчанию `0.1.0`.
3. **Authors** — `Input`: разделённые запятыми (из git config user.name/email если есть).
4. **License** — `Select`: UPL-1.0 / MIT / Apache-2.0 / Proprietary (по умолчанию UPL-1.0).
5. **Description** — `Input`: однострочное описание.
6. **Format** — `Select`: simple / normal (по умолчанию simple).

Для `vibe init projectname` (без пакета): только name, version, authors.

Неинтерактивный режим (`--unattended` или нет TTY): все значения из флагов или дефолтов.

## Неинтерактивные флаги

```
--name <name>              имя проекта/пакета
--version <version>        версия (по умолчанию 0.1.0 для пакета, 0.0.1 для проекта)
--author <name>            автор (можно повторять)
--license <license>        лицензия
--description <desc>       описание
--kind <flow|feat|stack|tool|mcp>  тип пакета
--format <simple|normal>   формат пакета
--link <static|dynamic>    link type (static для project+pkg, dynamic для package)
```

Старый `--path` остаётся для back-compat (эквивалент последнего позиционного).

## Что создаётся

### Проект (`vibe init projectname` или `vibe init org.vibevm.apple [path]`)

Те же файлы, что сегодня + опциональный пакет:
- `vibe.toml` — `[project]` (name/version/authors), без `[[registry]]` (уже в global).
- `vibe.lock` — пустой.
- `spec/{boot,common,modules,...}/` — структура.
- `.gitignore` + `.vibe/.gitignore`.
- Если pkgref указан — `packages/<group>/<name>/v0.1.0/vibe.toml` с `[package]` + `link = "static"` в `[boot_snippet]`.
- Boot artifacts regenerated.

### Пакет (`vibe init package org.vibevm.apple/orange [path]`)

- `packages/<group>/<name>/v0.1.0/vibe.toml` с `[package]` + `link = "dynamic"`.
- `packages/<group>/<name>/v0.1.0/spec/boot/`, `README.md`.
- Если проектный `vibe.toml` уже есть — regenerate boot artifacts.

### Группа (`vibe init group org.vibevm.apple [path]`)

- `packages/<group>/` — пустой каталог группы.

## Структура кода

### CLI changes

`crates/vibe-cli/src/cli/pkg.rs` — `InitArgs` получает позиционные аргументы:
```rust
pub struct InitArgs {
    pub subcommand: Option<InitSubcommand>,   // package | group | None
    pub pkgref: Option<String>,                // org.vibevm.apple/orange или org.vibevm.apple
    pub path: Option<PathBuf>,                 // путь (позиционный или --path)
    // существующие флаги остаются (--name, --stack, --registry-url, etc.)
    // новые:
    pub kind: Option<PackageKind>,
    pub version: Option<String>,
    pub author: Vec<String>,
    pub license: Option<String>,
    pub description: Option<String>,
    pub format: Option<PackageFormat>,
    pub link: Option<String>,
}

pub enum InitSubcommand { Package, Group }
```

### init.rs dispatch

```rust
pub fn run(ctx, args) -> Result<()> {
    match (args.subcommand, &args.pkgref) {
        (Some(Package), Some(pkgref)) => create_package(ctx, args, pkgref),
        (Some(Group), Some(group))    => create_group(ctx, args, group),
        (None, Some(pkgref_or_path))  => create_project_with_or_without_pkg(ctx, args),
        (None, None)                  => create_project_in_cwd(ctx, args), // today's behaviour
    }
}
```

### Новые функции

- `create_package()` — создаёт `packages/<group>/<name>/v<version>/vibe.toml`.
- `create_group()` — создаёт `packages/<group>/` каталог.
- `create_project_with_package()` — full project + package.
- `interactive_prompts()` — `dialoguer::Input` / `Select` для npm-style问答.
- `detect_git_author()` — читает `git config user.name` + `user.email` для default authors.

### Templates

Новый шаблон `templates/package-vibe.toml` для пакета:
```toml
[package]
group = "{group}"
name = "{name}"
kind = "{kind}"
version = "{version}"
authors = [{authors}]
license = "{license}"
description = "{description}"
format = "{format}"

[boot_snippet]
source = "spec/boot/{slot}-tool-{name}.md"
link = "{link}"
```

## Реализация по коммитам

1. **`feat(cli): InitArgs positional args + subcommands`** — CLI parsing changes.
2. **`feat(init): interactive prompts (npm-style)`** — dialoguer Input/Select.
3. **`feat(init): create_package + create_group`** — package/group creation.
4. **`feat(init): project-with-package + static link`** — combined project+package forms.
5. **`test(init): e2e for all 8 forms`** — cli_init tests.
6. **`docs: update init help + README`** — documentation.

## Совместимость

- `vibe init --path .` (старый синтаксис) продолжает работать — back-compat.
- `--name`, `--stack` остаются.
- `--registry-url` / `--no-registry` остаются.
- Неинтерактивный режим по умолчанию если нет TTY (как сегодня).

## Что НЕ в scope

- `vibe init package ... --from-template <url>` (scaffolding из шаблона) — future.
- Кастомные templates (npm init @scope/template) — future.
- Перенос существующих проектов в новую структуру — отдельная задача.
- `spec/WAL.md` создание — остаётся отдельной задачей (init intentionally не создаёт WAL).