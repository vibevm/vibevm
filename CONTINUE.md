# CONTINUE — холодный резюм (чекпойнт 2026-08-23, перед компактификацией)

> WAL (`spec/WAL.md`) — канонический ЖИВОЙ статус и главнее этого снапшота,
> если они разойдутся.

## TL;DR

**Сегодня за один день спроектирована, ратифицирована владельцем и целиком
построена система видимости зависимостей (PROP-050)** — W1–W8 посажены,
PROP-050 в статусе BUILT, финальная панель `all green`, зеркала синхронны,
дерево чисто (HEAD `aaeecc2e`). Параллельно живёт прежнее состояние релиза
1.0.0: всё готово, ждёт владельческой инспекции и PAT для C6 (публикации).

## Два активных трека

1. **Видимость (PROP-050) — ЗАВЕРШЕНА.** Остатки осознанно в
   `BACKLOG.md` B-102…B-105; два из них ждут владельческого слова (B-103
   budget-cap, B-105 concept-гейт).
2. **Релиз 1.0.0 — ждёт владельца.** Блокер: новый GitHub PAT → C6-волна →
   тег v1.0.0. Чеклист: `RELEASE-INSPECTION-CHECKLIST.md`; ТЗ:
   `campaigns/packages-2026-09/TZ-RELEASE-1.0-v0.1.md` (строка `_STATUS:`).

## Что построено сегодня (карта для холодного читателя)

**Закон:** `spec/common/PROP-050-dependency-visibility.md` (BUILT; все десять
владельческих развилок F1–F10 отрулены; единственный неотрулённый скетч —
`##CONCEPT-GATE-DIRECTION`). **Хроника стройки с приёмкой построчно:**
`spec/terraforms/VISIBILITY-BUILD-PLAN-v0.1.md`. **Prior-art и замеры:**
`spec/research/dependency-visibility-2026-08/` (3 отчёта воркеров + 04-measurements).

Модель одним абзацем: на ребре — `access` (public-дефолт / private =
dev-мир декларанта / friends-only = круг) и `friend` (false-дефолт; голая
friends-only-метка имплицирует свой грант — F10); на узле — `[visibility]`
(friends / unfriend / allow-friends: нет=открыто, []=запечатано, список=круг;
ignore-concept-warnings) и `[override]` (штатный ТИХИЙ взлом чужих рёбер,
any-node, ближний к корню побеждает). Печать гейтит ДРУЖБУ (внутрянку цели),
не доставку самой цели. Замыкание C(R) и эффективное множество E(R) —
`vibe_core::visibility::analyze` (joint fixpoint, экзистенциальность по
цепям). Слоение: видимость → резолв → link-ось → линкер → ленты; форс
`static-transitive` не пробивает видимость.

Код по крейтам: `vibe-core` (manifest/package/visibility.rs — словарь;
visibility/{graph,closure,effective,diagnostics,query}.rs — движок + мир
установленного); `vibe-install` (visibility_projection.rs — проекция,
итерация строгости cap 4, FilteringDepProvider; plan/apply/record — E(R) и
провенанс в lock v6); `vibe-cli` (why.rs, friends.rs,
install/closure_diff.rs, tree-суффиксы, registry.rs — конструкция ячеек);
`vibe-check` (visibility_hygiene.rs; snippet_presupposition смягчена).
Сквозные доказательства: `crates/vibe-cli/tests/cli_visibility_lanes.rs` (5)
и `cli_visibility_power.rs` (6).

Числа: хост 37 пакетов (7 root-edge + 30 public-chain), `STATIC.xml`
248 429 B byte-identical через обе переустановки; расчётный эффект W7 для
стороннего потребителя `delegation-first`: ≈32 → 2 пакета.

## Блокер и точное действие владельца

- **Релиз:** выдать PAT → сказать слово → C6-волна → тег v1.0.0 (ТЗ §10).
- **Стройка:** рулинги по B-103 (budget-cap на ленту) и B-105
  (`when="concept:X"`) — по желанию; опция «wal у хоста через
  friends-only-redbook вместо прямого ребра» — вынесена, не срочно.

## Рецепт следующих шагов (если владелец скажет продолжать без него)

1. B-104: развести омоним ключа `override` (легаси `[[override]]`-пины vs
   visibility-таблица) — no-legacy право есть; периметр:
   `vibe-core/src/manifest/package/visibility.rs` (ManifestWire union) +
   `project.rs` (OverrideSection) + спека `##OVERRIDE-KEY-COEXISTENCE`.
2. B-102: удешевить `edge_contributes`
   (`vibe-install/src/visibility_projection.rs:138`) — батч-proof по цели.
3. 37 stale + аудит `2026-08-06-01` — прежняя очередь (см. WAL known-issues).

## Небанальные находки сессии (уроки — в WAL-констрейнтах, тут указатели)

- `codexrunner exec` читает stdin → только `< /dev/null` (иначе виснет).
- Схемный бамп через живые worktree = межволновой дрейф фикстур → lock-шапки
  в тестах только `CURRENT_SCHEMA_VERSION`.
- `vibe install` обязан читать неподдерживаемую схему lock как пустую
  (регенерационный глагол) — починено (`b8b61f74`) + юнит.
- Windows-локи бьют свап `crates/vibe-wire/src/generated` → снести
  `generated.new-*`, перегнать check-codegen.
- Conform R-001: конструкция провайдер-ячеек — ТОЛЬКО в
  `vibe-cli/src/registry.rs` (за это уже платили красной панелью).
- Честный стоп воркера с точным списком минимального расширения периметра —
  штатный исход; пакет правится по его списку и респавнится (W2 v1→v2).

## Карта репозитория (крупно)

`crates/` — рабочее пространство vibe (core / install / cli / check /
workspace / resolver / registry / facts / specdoc / wire / mcp / xtask …);
`packages/` — авторские prompt-пакеты (org.vibevm.world редбук и флоу,
org.vibevm.ai-native стеки, org.vibevm.fractality); `vibedeps/` —
материализованные слоты (генерят); `spec/` — законы: `common/PROP-0xx`,
`modules/<crate>/PROP-0xx`, `boot/` (генерённые ленты STATIC/INDEX),
`terraforms/` (планы), `research/`; `campaigns/packages-2026-09/` —
релизная кампания (ТЗ, суд, harvest); корень: CLAUDE=AGENTS=GEMINI.md,
TASKS.md, BACKLOG.md, AUDIT.md, SPECSPACES.md, AGENT-MODE.toml.

## Действующие архитектурные решения (длинно — в законах, тут список)

Токеномика — центральный закон (PROP-048; STATIC = кэш-стабильный префикс;
THE-LAYER-LAW). Видимость выше материализации (PROP-050 §3). Дефолты:
presence щедрое (`access=public`), дружба скупая (`friend=false`), F10.
No-legacy: схема одна, lock v6, откатов нет. Роли [project]/[package]
эквипотентны (PROP-024, consumer_node). Жанр сниппетов: никакой
пресуппозиции чужих дисциплин (PROP-049, смягчена до warnings по F7).
Делегирование-first: codexrunner → claudez (claudez2 запрещён), YOLO-заборы
= текст пакета + боссово ревью диффа; спеки/вердикты/коммиты не делегируются.
Судейская юрисдикция кампании НЕ покрывает spec/common/PROP-050 (долг 0/0 —
подтверждено; статусы стройки ведены вручную).

## Последние 26 коммитов

aaeecc2e chore(lock): timestamp from the W5b live smoke reinstall
e3a37dbd docs(spec): PROP-050 is BUILT — statuses flipped, backlog drained…(W8)
4268008d feat(cli): closure diff on install/update + tree provenance (W5b)
4ae826e3 docs(plan): W7 landed
d1d6d629 docs(research): W7 measurements — byte-identical lanes, 32→2
20658ad0 chore(install): host reinstall after the W7 narrowing
0aa66062 feat(packages): fractality group narrows deliberately (W7)
2480a1a0 docs(plan): W5a landed, panel green
27ae1393 chore(specmap): visibility query/observe edges
75cd38c5 refactor(cli): cells return to the registry seam (R-001)
f2531a68 fix(tests): omnibus lock assertion follows CURRENT_SCHEMA_VERSION
c7d4017a feat(observe): vibe why, vibe friends, visibility_hygiene (W5a)
e20b5589 docs(plan): W6 landed
07443301 test(cli): e2e power goldens (W6)
1b645cd1 chore(lock): host reinstall on the E(R) machinery — v6
b8b61f74 fix(install): unsupported-schema lock reads as empty on install
adeef22d docs(plan): W3 landed
2fbd9cc3 test(cli): e2e lane goldens (W3)
7034e743 docs(spec): PROP-009/034/035/038 — the linker's world is E(R)
6f02ff95 docs(plan): W2 landed
243dbb73 feat(install): resolution and lock live on E(R) (W2)
074a45f0 docs(plan): W4 closed; W2 v2 in flight after an honest stop
ad6d5caf feat(check): concepts gate learns the mute + seeping exemption (W4b)
1899680b docs(plan): W1 and W4a landed
981496df docs(spec): PROP-050 — joint fixpoint corrected at landing…
342b3aae feat(core): the visibility vocabulary and the pure closure engine (W1)

## Быстрый старт

```sh
bash tools/self-check.sh            # панель; вердикт только по хвосту
cargo xtask specmap                 # после spec/code-правок
cargo run -q -p vibe-cli -- why org.vibevm.world/wal      # наблюдаемость живьём
cargo run -q -p vibe-cli -- install --path . --assume-yes # переустановка хоста
cargo xtask mirror                  # раскатка (ff-only)
python campaigns/packages-2026-09/tasks/judging-debt.py   # судейский долг
```

Аудит здоровья: активное подмножество — `AUDIT.md` (P1 `2026-08-06-01` —
пересуд будущей кампанией).
