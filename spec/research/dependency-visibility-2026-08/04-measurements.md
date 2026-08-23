# PROP-050 W7 — измерения посадки видимости (##VERIFY-MEASURE)

> Замеры стройки, 2026-08-23. Хост: `org.vibevm.core/vibevm`, 37 пакетов в lock.
> «До» = main до W7-меток (машинерия W1–W6 уже посажена, дефолты public/false);
> «после» = после осознанного сужения (private-метки fractality-группы) и переустановки.

## Хост-мир (потребительский корень этого репозитория)

| метрика | до | после | дельта |
|---|---|---|---|
| членов в `vibe.lock` | 37 | 37 | 0 — у хоста все дисциплины свои, прямыми рёбрами |
| `spec/boot/STATIC.xml`, байт | 248 429 | 248 429 | **byte-identical** |
| `spec/boot/INDEX.md`, байт | 1 512 | 1 512 | topo-пересортировка 3 строк, состав не менялся |
| provenance в lock | — (v5, полей не было) | 7 × `root-edge`, 30 × `public-chain` | схема v6 |
| время `vibe install` (свежий бинарь, тёплый store) | ~секунды | ~секунды | без деградации от проекции+итерации |

Ключевой совместимостный факт: переустановка хоста на новой машинерии
(strict-чтение, public-дефолт) воспроизвела ленты **байт-в-байт** — ровно
обещание ##DEFAULT-PUBLIC-RATIONALE / ##migration-flag-day-scope.

## W7-сужение: private-метки

Помечено `access = "private"` (v1.0.0-слоты; замороженные v0.x не тронуты):

- `delegation-first` → redbook, rust-ai-native (delegation-rules ОСТАВЛЕН public — директива тянет свой калькулюс);
- `delegation-rules` → redbook, rust-ai-native;
- `fractality` → redbook, rust-ai-native, wal-specspaces.

Обоснование — ##REEXPORT-USAGE-NORM: это фоновые дисциплины автора пакета
(его dev-мир), не субстанция для потребителя; сниппеты самодостаточны
(пресуппозиционный гейт зелёный без этих рёбер).

Эффект в хост-lock: `dependencies` пакета delegation-first сжался с
3 рёбер до 1 (`delegation-rules`), delegation-rules — до 0: lock-контракт
теперь честно отражает просачивающуюся поверхность, не dev-мир.

## Расчётный эффект для стороннего потребителя

Проект, тянущий ТОЛЬКО `flow:org.vibevm.fractality/delegation-first`:

| | до | после |
|---|---|---|
| членов E(R) | ~32 (redbook-паровоз: 21 член + git-practices 4 + wal-specspaces + rust-стек 3 + core) | **2** (delegation-first + delegation-rules) |
| смысл | WAL-класс утечки: «один edge тянет мир» | директива + её калькулюс |

Проверено сквозными голденами жанра (`cli_visibility_lanes.rs`:
private-ребро не доезжает до lock/vibedeps/INDEX/STATIC; ромбы и
friends-цепи — по своим сценариям).

## Наблюдаемость на живом мире (пост-W5a)

- `vibe why org.vibevm.world/git-atomic-commits` → `present: public-chain via
  org.vibevm.core/vibevm -> org.vibevm.world/redbook ->
  org.vibevm.world/git-practices -> org.vibevm.world/git-atomic-commits`;
- `vibe why org.vibevm.world/wal` → `present: root-edge`;
- `vibe friends org.vibevm.world/redbook` → `open… actual friends: none…
  in root closure: no` (у хоста грантов нет — дефолт `friend = false`).

## Снапшот-пин

Состав хост-лент пинится живыми тестами (redbook-roundtrip корпус,
vibedeps-материализация, lane-голдены W3/W6 на герметичных мирах);
одноразовый лендинг-дифф W7 отревьюен вручную: изъятий из хост-лент нет,
изменения — только lock-контракты и topo-порядок INDEX.
