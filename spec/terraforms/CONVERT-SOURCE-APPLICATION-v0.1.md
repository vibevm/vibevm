# CONVERT-SOURCE — стройка глагола и флип корпуса на XML {#root}

_STATUS: K1 в делегированной стройке — 2026-08-23 · закон: `spec/common/PROP-051-refactor-umbrella.md` · пивот готов заранее (PROP-045, `specdoc/4`, хост уже `spec_format = "xml"`)

<status stage="impl" state="work" comment="нарезка K1-K7; числа инвентаря заполняются dry-run'ом после K1"/>

## Периметр {#perimeter}

@fact:CONVERT-PERIMETER Мандат владельца покрывает **авторские исходники
спецификаций**: хостовое дерево `spec/` и каждый член `packages/`.
Вне периметра: `vibedeps/` и весь материализованный/генерённый стейт
(PROP-051 ##VIBEDEPS-NEVER-CONVERTED — переезжает рематериализацией),
`CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (имена — контракт харнессов),
`refs/**` (чужое), фикстуры тестов (данные, не исходники), корневые
`*.md` и `campaigns/**` (операционные документы, не спеки; захотим —
тем же глаголом отдельной волной по слову владельца). @status:impl/work

## Нарезка {#slices}

@fact:K1-TOOL **K1 — глагол.** Ядро классификации в `vibe-specdoc`
(`convert.rs`: класс 1/2/3 по PROP-051 ##HONESTY-BY-REVERSE + loss-diff)
и тонкая команда `vibe refactor convert-source` в `vibe-cli`
(walk + skip-правила ##CONVERT-SOURCE-SURFACE, prompt/`--force`/
`--dry-run`, пофайловый отчёт, коды выхода). Юниты на ядро (все три
класса, комменты как content-loss, оба направления) + e2e на голом
корпусе (tempdir): чистый файл, файл с комментом, `already`, dry-run,
непарность после конверсии. Делегируется; спека коммитится до спавна.
@status:impl/work

@fact:K2-INVENTORY **K2 — инвентарь.** `--dry-run --to xml` над `spec/` и
`packages/` — классы по всем ~115 + ~1511 файлам; числа сюда в план.
Хвост класса 2 (комменты/экзотика) разбирается руками ДО массового
прогона: REVIEW-маркеры и содержательные комменты переносятся в текст
или сознательно отпускаются. @status:impl/plan

@fact:K3-HOST-SPEC **K3 — хостовый `spec/`.** Конверсия всего дерева
(включая `spec/WAL.md` и терраформы; генерённый `spec/boot/*` глагол
пропускает сам по маркеру) + свип by-name ссылок на переименованные
пути: инлайн-ссылки в прозе спек, буты/хуки/`TOOLING-MAP.md`, текст
CLAUDE.md, указывающий на `spec/WAL.md`. Гейт: `vibe check` чист,
`wal_wellformed` держит XML-источник через проекцию. @status:impl/plan

@fact:K4-PACKAGES **K4 — `packages/`.** Каждый член (world, ai-native,
fractality — включая specspace-документы fractality) конвертируется тем
же глаголом; свип их внутренних by-name ссылок и бут-контрактов.
@status:impl/plan

@fact:K5-REMATERIALISE **K5 — рематериализация.** `rm -rf
~/.vibe/cache/<группы>` (B-101), `vibe install` — слоты и ленты
перерождаются из XML-исходников. Ожидаемый дифф STATIC.xml: только
provenance-комменты (расширение источника `.md` → `.xml`; парити-закон
PROP-045 ##INHERITANCE-PARITY), содержимое вкладов byte-identical.
Любое другое движение — находка. @status:impl/plan

@fact:K6-GATES **K6 — гейты и свипы.** Полная панель по хвосту; specmap
регенерация (пути юнитов переезжают на `.xml`); судейский долг остаётся
«no debt» (адресация format-blind — PROP-045 ##ADDRESSING-UNCHANGED);
грепом — ни одной живой ссылки на сконвертированный `.md`-путь;
WAL-ожидания обновлены. @status:impl/plan

@fact:K7-WINDDOWN **K7 — финализация.** Статусы PROP-051 → impl/done,
этот план закрывается приёмкой, остатки — в BACKLOG. @status:impl/plan

## Приёмка {#acceptance}

@fact:CONVERT-ACCEPTANCE В периметре не остаётся ни одного авторского
`.md`-спека (вне периметра — нетронуто); панель `all green`;
`vibe check` 0/0/0; judging-debt «no debt»; STATIC.xml — только
ожидаемый provenance-дифф; `vibe refactor convert-source --dry-run`
по обоим деревьям отвечает «already» на 100 % файлов. @status:impl/plan
