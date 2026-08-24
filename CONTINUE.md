# CONTINUE — холодный резюм (чекпойнт 2026-08-24, поздний вечер)

> WAL (`vibevm/vibespecs/WAL.xml`) — канонический живой статус; при
> расхождении он главнее этого снапшота.

## TL;DR — день из трёх посаженных волн

1. **Переезд vibevm/** (PROP-052) — утро: 4 корня + 86 пакетов + флип +
   свип; панель, clean-room, specmap-паритет; `ecb0a5de…4edb5f2c`.
2. **Normal-флип** — все 42 живых слота vibepacks в `format = "normal"`;
   компилятор дозрел до XML-мира: seed стрипает `.xml`, multi-doc
   closure получает doc-slug (вне boot/ и contract/), qualify узнал
   ячеечные (K6.5) и цитатные факты, вложенные ноды поглощаются
   (READ-ONCE), `@spec` — агентское ребро (вклейка только `#use`;
   реализованный ##OPEN-CLOSURE-EXPLOSION: 250 KB → 2.5 MB → обратно).
   `b56c604b / b82b7423 / db4a36e8`.
3. **Install-фиксы + vibe clean** — вечер:
   - `vibe install <pkg> [--offline]` больше НЕ сносит мир (полная
     резолюция: named свежо + остальное на пинах) — владельческая
     репродукция закрыта e2e-тестами;
   - ноль зависимостей — штатный no-op (`vibe init && vibe install` из
     коробки);
   - same-version дрейф local-источника: write-once кэш рефрешится по
     хэшу (`insert_current_at`, все 5 фетч-путей);
   - **`vibe clean`** (PROP-053): derived-only (vibedeps + generated
     STATIC/INDEX/INLINE по маркеру); лок, авторское, машинный кэш —
     неприкосновенны; цепочка **`vibe clean install [pkgref]`** =
     wipe → мир из лока → refresh названных.
   `1cfdec31 / eb9103ab`. Панель all green; specmap 0 unresolved.

## Ключевые уроки дня (они же в WAL constraints)

- `@spec` не вклеивается компилятором — только `#use` (иначе лейн ×10).
- Кэш write-once + hash-guard: не возвращать голый `insert_at` в фетчи.
- Панель на занятом боксе — `CARGO_BUILD_JOBS=4` (0xc0000142 ловится и
  одной панелью на полном параллелизме линков).
- Фрозен-слоты не чинятся; синк-таргеты не правятся руками.

## Кандидаты следующей сессии (владельческие развилки)

1. Релиз 1.0.0: инспекция → PAT → С6 → тег (TASKS.md / TZ-RELEASE §10).
2. B-107 (пакетный XML-корпус фактов), B-103/B-105.
3. Добить статусы `impl/plan → impl/done` в PROP-053 / PROP-011-рулингах
   (код зелёный, чисто доковая правка).

## Быстрый старт

```sh
git log --oneline -10          # три волны дня сверху
CARGO_BUILD_JOBS=4 bash tools/self-check.sh
./target/debug/vibe.exe clean install --path . --assume-yes   # новый глагол
```
