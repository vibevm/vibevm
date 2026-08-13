# CONTINUE — cold-resume snapshot (2026-08-13, wind-down №19)

**Не цитируй числа отсюда — меряй:**
`vibe progress scan --campaign campaigns/packages-2026-09` →
`python campaigns/packages-2026-09/tasks/{summary,judging-debt,text-stability}.py`.
`spec/WAL.md` переписан этим же сворачиванием и **главнее** этого файла.
Вход новой сессии — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md)
(переписан: теперь он **авторизует стройку**, не доклад-и-ждать).

## TL;DR

Сессия 2026-08-13 получила от владельца ВСЕ ожидавшиеся рулинги и
**ратификацию PROP-044** («Ратификацию на сам PROP-044 даю»), посадила
рулинги идентичности одной посадкой (21 коммит), написала два новых плана
строек и перевела вход следующей сессии в режим исполнения.

**Село кодом и спеками (всё раскатано, панель зелёная):**

- **Грамматика группы = LDH-метки** (`[a-z0-9-]`, дефис не с краю, `_`
  запрещён; ≥1 сегмент — ядро, ≥2 — политика реестра) — `Group::parse`.
- **Плоская координата точкой:** репо-имя `<group>.<name>`
  (`org.vibevm.world.wal`), кэш-каталоги так же; 7 мест композиции.
- **Слот от идентичности:** `vibedeps/<group>.<name>/<version>` (+ in-place
  без версии); 37 слотов перематериализованы, INDEX/STATIC перегенерированы.
- **`materialization = "copy"`** (бывший `"snapshot"`): легаси-значение —
  отказ с рецептом, не alias; слово snapshot теперь значит ровно одно.
- **Спеки рулингов:** доверие (группа — заявка; org ручается; два корня
  по умолчанию) + жизненный цикл (`Removed`+tombstone, видимая
  перерегистрация, `--accept-new`, `--force-replace --reason`) + двухъярусный
  publish — PROP-002 §2.10/§2.13, PROP-008 §2.10; терминология
  snapshot↔frozen/канал/capture — PROP-044 §2b; **каналы целиком** —
  PROP-005 §2.18; D11=warn и D14-умолчание (STABLE→LATEST, заморозка не
  влияет на выбор) — СТОПы в ТЗ сняты; рулинги №4 (двухсортная планка) и №5
  (движки не сливать) — в AUDIT/BACKLOG.

**Планы (три, взаимно сцеплены):**

1. [`TZ-CHANGE-NATIVE-FORMATS-v0.1.md`](campaigns/packages-2026-09/TZ-CHANGE-NATIVE-FORMATS-v0.1.md)
   — главная стройка Ф0–Ф6; ратифицирована, фазы разрешены.
2. [`TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md`](campaigns/packages-2026-09/TZ-IDENTITY-REGISTRY-BUILDS-v0.1.md)
   — слайсы рулингов: S1 двухъярусный publish, S2 переименование живых
   `_`-репо (СТОП-ВЛАДЕЛЕЦ), S3 освежение joiner-юнита addressable-specs,
   S4 каналы / S5 жизненный цикл (заперты до Ф3), S6 движки-по-данным,
   S7 пересуд host-подмножества. §0 — карта гейтов.
3. [`TZ-CHANGE-NATIVE-WAVE2-v0.1.md`](campaigns/packages-2026-09/TZ-CHANGE-NATIVE-WAVE2-v0.1.md)
   — §0 **матрица покрытия PROP-044** (ответ «достаточно ли ТЗ»), W1
   манифест (did-you-mean, `[reserved]`, схема, toml_edit-кодмоды), W2
   локфайл D9 (`--locked`), W3 показ frozen на поверхностях, W4
   не-каталожные форматы + G12/G13. Заперто до Ф1–Ф5. Волна 3 осознанно НЕ
   написана (триггеры: первый поезд эпох; `public = true`).

## Блокер и действие человека

**Блокера нет.** Всё разрешено: свежая сессия стартует по
`NEXT-SESSION-PROMPT.md` и сразу исполняет (Ф0 — спайки без коммитов).
За владельцем остаются только: **S2** (переименование `_`-репо в vibespecs —
нужны org-права: `gh repo rename`, шаги в ТЗ) и нетронутые вопросы **№4-кампания
S7 стартуема без него** и **№5 — построится S6**.

## Рецепт следующего шага (дословно)

Вставить содержимое `NEXT-SESSION-PROMPT.md` (ниже черты) первым сообщением
свежей сессии. Порядок там: boot → 4 документа целиком → короткий доклад →
Ф0 (три спайка, выход в `harvest/f0-*.md`, БЕЗ коммитов) → Ф1… Независимая
полоса при СТОПе: S1 → S3 → S6.

## Неочевидные находки сессии (сверх документов)

- **Седьмое место композиции** пряталось в `vibe-check`
  (`format!("vibedeps/{}-{}/{}")` — невидимо для шаблонных grep'ов по
  `<kind>-<name>`); поймала только полная панель. Урок: искать и по
  `format!`-строкам.
- **Печать (seal) ручается за ВСЕ вердикты файла.** Печать файла со стоячим
  stale = слепое поручительство; прецедент отката — `498e8c8b` (PROP-043).
  Правило вписано в identity-ТЗ §1.4.
- **`BACKLOG.md` вне судимого корпуса кампании** (`merge-verdicts`: «not
  observed») — его правки судейского захода не требуют.
- Док-комментарии кэш-функций и `init.rs` **уже писали `<group>.<name>`
  точкой до рулинга** — код отставал от собственной документации.
- `_` не было ни в одной живой группе/имени (сужение бесплатно); имена и
  раньше были LDH (`validate_package_name`).
- `prune_stale_slots` слеп к форме имени → перематериализация сама вычистила
  37 старых слотов.
- `specmap.toml` нёс external-root старым слотом (после починки 229→208
  warnings).
- Для волны 2 проверено: `toml_edit = "0.23"` уже в workspace (кодмоды W1
  готовы к постройке); флага `--locked` в CLI нет (W2 вводит); `.vibe/cache`
  уже identity-keyed (PROP-000:123 отставал — починено).
- Ратификация пришла серединой хода и записана в 4 местах: статус PROP-044,
  оба ТЗ, строка №1 в BACKLOG.

## Где стоит работа

- Ветка `main`, дерево чистое; **21 коммит этой сессии раскатан**
  (`cargo xtask mirror`; `mirror --check` — gitverse sync, github sync,
  HEAD `6cd2f995`).
- Полная панель `bash tools/self-check.sh` — **зелёная** (последний полный
  прогон — слайс copy; после него только docs-коммиты).
- Судейство: **0 неосуждённых, 0 осиротевших**; 33 файла stale — стоячий
  долг, адресован кампанией S7 (рулинг №4). Новые/правленые факты сессии
  (33+18+17+1) осуждены и запечатаны тем же заходом.
- Открытые находки аудита — активное подмножество в [`AUDIT.md`](AUDIT.md);
  `2026-08-06-01` (P1) переведена в «ruled — re-judgement campaign pending».

## Карта репозитория (что где)

- `spec/common/PROP-044…` — ратифицированный контракт форматов;
  `spec/modules/vibe-registry/PROP-002` (§2.10 publish, §2.13 lifecycle),
  `PROP-008` (§2.1 грамматика, §2.5 репо-имя, §2.10 доверие);
  `spec/modules/vibe-index/PROP-005` (§2.18 каналы);
  `spec/modules/vibe-workspace/PROP-022` (слот, режим `copy`).
- `campaigns/packages-2026-09/` — три ТЗ (выше), `tasks/*.py` (суд),
  `run/` (кэш вердиктов), `harvest/` (сюда лягут находки Ф0),
  `SUBAGENT-LAUNCHERS.md` (транспортный закон воркеров — босс читает ЦЕЛИКОМ
  перед фан-аутом).
- `crates/` — 19 крейтов + xtask; предмет строек: `vibe-publish` (S1),
  `vibe-index` (Ф1–Ф6, S4), `vibe-core/manifest` (W1/W2),
  `vibe-registry` (резолвер, S4/S5).
- Корень: `BACKLOG.md` (пять вопросов: №1 ЗАКРЫТ ратификацией; №3/№4/№5
  решены рулингами), `AUDIT.md`, `NEXT-SESSION-PROMPT.md`, `specmap.json`.

## Решения в силе (опорные; длинно — в спеках)

- **PROP-044 ратифицирован** — стоячий закон каждого формата; терминология
  §2b обязательна (snapshot↔frozen антонимы; «канал» — указатели; capture —
  провайдерское).
- Идентичность: LDH-группы; композит `<group>.<name>` — валидный
  обратный FQDN; слот и кэш — от идентичности; координата = полная строка
  версии (включая `+…`), тай-брейк natural sort.
- Доверие: группа — заявка; внутри реестра ручается организация
  (модерация); между реестрами — `content_hash` + порядок реестров; корни
  по умолчанию — оба vibespecs.
- Резолвер: STABLE если есть, иначе LATEST; заморозка не влияет на выбор;
  канал — явное согласие на мутабельность.
- Усилия не экономятся; объём — не довод. Раскатка только `cargo xtask
  mirror`. Никогда `git add -A`. Печать суда — только за проверенное.

## Последние коммиты сессии (свежие сверху)

```
6cd2f995 docs(handoff): the entry prompt turns from report-and-wait to build
560e8f67 chore(campaign): the ratification pass re-vouches the contract
874561fd docs(campaign): wave 2 closes the coverage gaps PROP-044 still had
7a81956e docs(spec): PROP-044 is ratified, and every gate that waited knows it
c2dec4ef docs(campaign): the rulings get their build plan while the context is hot
498e8c8b chore(campaign): the copy rename is judged, and a blind vouch is taken back
d84cf5b6 docs(spec): dead slot pointers follow the re-materialised tree
d4d6c475 feat(core): the default materialisation mode says copy
6f4b5750 fix(check): the lockfile-files check composes the identity slot
6e90854e chore(campaign): the identity rulings are judged in the same pass
189152af docs(specmap): the map follows the identity rulings
30049e81 docs(backlog): three owner rulings land in their rows
1cd487b3 docs(audit): the proof bar splits by claim kind
1e2c9ebe docs(campaign): the executor's plan absorbs the week's rulings
9fc59c01 docs(spec): channels become author pointers with computed built-ins
3106bf0d docs(spec): snapshot and frozen become antonyms across the contract
13c8638a docs(spec): the group is a claim, and the registry vouches for it
df5d4563 feat(workspace): the slot carries identity, not kind
e55856b7 docs(spec): the dot join reaches every surface that spelled the old one
fb7760da feat(registry): the flat coordinate joins group and name with a dot
7a5d8f4b feat(core): group segments become LDH labels, matching real domains
```

## Быстрый старт

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # реальный код выхода, вердикт из хвоста
cargo xtask specmap --check
cargo xtask mirror --check        # синхронность зеркал
```

_WAL — канонное живое состояние; при расхождении верить ему, не этому файлу._
