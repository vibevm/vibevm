# CONTINUE — холодный резюм (чекпойнт 2026-08-24, ночь)

> WAL (`vibevm/vibespecs/WAL.xml`) — канонический живой статус; при
> расхождении он главнее этого снапшота.

## TL;DR — день из четырёх посаженных волн (все на зеркалах)

1. **Переезд vibevm/** (PROP-052) — 4 корня хоста + spec 86 пакетов +
   флип `USE_NEW_LAYOUT` + свип; specmap-паритет 0 unresolved,
   clean-room `init→install→check`, агентская проба маршрутизации PASS.
   `ecb0a5de … 4edb5f2c`.
2. **Normal-флип** — все 42 живых слота vibepacks в `format = "normal"`;
   компилятор дозрел до XML-мира (PROP-035, четыре поправки): seed
   стрипает `.xml`; multi-doc closure — doc-slug вне контракт-домов
   (`boot/`, `contract/`); qualify узнал ячеечные (K6.5) и цитатные
   факты; вложенные ноды поглощаются (READ-ONCE); **`@spec` — агентское
   ребро, вклеивает только `#use`** (реализованный
   ##OPEN-CLOSURE-EXPLOSION: лейн 250 KB → 2.5 MB → обратно к
   снипетному размеру). `b56c604b / b82b7423 / db4a36e8`.
3. **Install-фиксы + vibe clean** (PROP-011 три рулинга; PROP-053
   новый): `vibe install <pkg> [--offline]` больше не сносит мир
   (полная резолюция: named свежо + прочие на пинах); ноль зависимостей
   — штатный no-op (`vibe init && vibe install` из коробки);
   same-version дрейф local-источника лечится hash-guard'ом write-once
   кэша (`insert_current_at`, все 5 фетч-путей). **`vibe clean`** —
   mvn-clean со спецификой промптов: derived-only (vibedeps +
   generated STATIC/INDEX/INLINE по маркеру); лок, авторское и машинный
   кэш неприкосновенны; цепочка **`vibe clean install [pkgref]`** =
   wipe → мир из лока → refresh названных. Шесть e2e-тестов.
   `1cfdec31 / eb9103ab`.
4. **Bootstrap-генераторы трёх стеков** (находка владельца из
   consumer-воркспейса contentdevtools): `… init` писал
   `spec_roots = ["spec"]` и ходил по `vibedeps/<slot>/<ver>/spec`.
   Rust-фикс портирован из consumer-репликации (их 9cc8161) в канон;
   тот же жанр закрыт в typescript и go (+ regression-ассерты);
   fresh-project e2e всех трёх стеков свипнуты; mcp-пакеты — через
   sync-engines. Перематериализация стала боевой проверкой hash-guard:
   слоты подхватили фикс без бампа версии. `f4fbee14`.

Панель all green после каждой волны. Дерево чисто, зеркала синхронны.

## Ключевые уроки дня (они же в WAL constraints)

- `@spec` не вклеивается компилятором — только `#use` (иначе лейн ×10).
- Кэш write-once + hash-guard: не возвращать голый `insert_at` в фетчи;
  дрейф той-же-версии для local-источников — норма, guard его ловит.
- Панель на занятом боксе — `CARGO_BUILD_JOBS=4` (полный параллелизм
  линкеров ловит 0xc0000142 даже одной панелью).
- Пакетные воркспейсы форматируются СВОИМ `cargo fmt` (хостовый их не
  видит); синк-таргеты (mcp и vendor) не правятся руками — только
  авторская копия + `cargo xtask sync-engines`.
- Фрозен-слоты (v0.6–v0.8) — история: не чинятся под новые раскладки.
- WAL-проза: строка-продолжение факта не должна начинаться с `+ ` —
  MD-проекция читает её list-item'ом (anchored-when-marked ловит).

## Кандидаты следующей сессии (владельческие развилки — не стартовать самому)

1. **Релиз 1.0.0**: инспекция по `RELEASE-INSPECTION-CHECKLIST.md` →
   новый PAT → С6-публикация → тег (TASKS.md / TZ-RELEASE §10).
2. **B-107**: наблюдать ли пакетный XML-корпус фактов (3 мёртвых
   exclude в facts.toml; корпус 98 файлов — .md-остаток). Плюс прежние
   B-103/B-105.
3. Добить статусы `impl/plan → impl/done` в PROP-053 и PROP-011-рулингах
   (код и тесты зелёные — чисто доковая правка).
4. TS fresh-e2e на этом боксе красен из-за отсутствующих
   `tools/*/node_modules` (`npm install` в ts-extract / ts-oracle
   лечит; панель этот класс штатно скипает).

## Быстрый старт

```sh
git log --oneline -12                    # четыре волны дня сверху
CARGO_BUILD_JOBS=4 bash tools/self-check.sh   # нарратит [N/54], heartbeat 30s
./target/debug/vibe.exe clean install --path . --assume-yes   # новый глагол
cargo xtask specmap && cargo xtask sync-engines               # оба нарратят
```
