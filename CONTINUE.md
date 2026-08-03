# CONTINUE — cold-resume snapshot (2026-08-04, wind-down №5: волна А закрыта, мандат на Б/В/Г)

**Не цитируй числа из этого файла — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` переписан той же сессией и **суперсидит** этот снапшот.

**TL;DR.** Одна сессия посадила ЧЕТЫРЕ рулёные стройки, и волна А закрыта
целиком: **B-006** (once-each полоса: покрытые юниты де-субституируются,
git-семья эмитится однажды, −404 строки, двойные префиксы 164→0),
**B-031** (хост — пакет **`org.vibevm.core/vibevm`**: миграция 1 893
вхождений по 606 файлам, residue 0; `spec://vibevm/…` парсится и никогда
не резолвится — `LegacyHostAuthority` с подсказкой; SCOPE-HOST —
void-надгробие), **B-028** (флоу публикует ПОЛНУЮ грамматику; версия в
адресе — опциональная фича, без версии — **свежайшая установленная**,
рулинг дословно), плюс routine-пересуды (F-159 → B-022 done; F-146 ×2;
F-169 целиком; близнецы F-147) и хост-фикс терраформа. **Владелец дал
НОВЫЙ мандат: все оставшиеся волны — Б, В, Г** (§7 LOG, запись
2026-08-04, дословно). Панель зелёная (хвост), зеркала синхронны.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — волна Б, батч 1

Добро стоит (§7 LOG «НОВЫЙ МАНДАТ», 2026-08-04) — НЕ переспрашивать.
Порядок волн — карта `TOOLING-MAP.md` §4; развилки владельца из её §5 —
по одной, по мере подхода (мандат на стройку ≠ мандат на его развилки).

1. **Прочти строки батча:** `BACKLOG.md` **B-029** (нейтральный ключ
   гейта + обогащение conform.toml под Go/TS; рулинг 2.1 записан в
   строке: «правда кода сегодня, идиома — записанной стройкой»; моя
   заготовка — нейтральный ключ + языковые алиасы навсегда), **B-034**
   (инвариант gated-or-exempt для Go/TS; единица гейта пер-язык решается
   вместе с B-029), **B-039** (прочти его строку — в этой сессии не
   читалась). Пересечение периметров реши ДО нарезки: B-029+B-034 почти
   наверняка один поток (общая поверхность conform.toml/Config), B-039 —
   по его периметру.
2. **Конвейер тот же, что посадил волну А:** boss-дизайн там, где
   контракт меняется (Config-ключи вендорятся ×6 — это правка движка:
   sync-engines тем же заходом, fmt по пакетным workspace'ам, панель);
   код — claudez-пакетами по закону транспорта (self-verify: check +
   targeted tests + clippy -D warnings + `wc -l` ≤ 600); ревью по
   WORKER-REPORT; вердикты/коммиты — босс.
3. **После посадки батча** — пересуды его строк (F-185 ждёт parity-семью)
   тем же mirror → merge-verdicts → seal (не сцеплять), затем следующий
   батч Б: (B-033 + B-030) → (B-036 + B-037 + B-038) → (B-025 + B-026;
   на B-025 висит последний якорь F-146, на B-026 — F-206). B-035
   (лупом после каждого батча), B-003 — попутно.
4. **Волна В после Б** (или чересполосно при непересекающихся
   периметрах): B-019а + B-016.1 + B-017 (+решение B-024) →
   B-018.1/.2 → B-018.4 + B-016.2 → **B-020** (разблокирует четыре
   interim-аннотации LEDGER-INTENT) + B-021 (+решение B-014).
   **Волна Г** — параллельно-оппортунистически: B-040, B-005, F-132,
   B-010-check.

## Рулинги владельца этой сессии (все дословно в durable)

- **B-006 одобрен** («согласен с твоими рекомендациями a1 b1 c1») + две
  его пробы усилили правило до де-субституции и явной границы частичного
  покрытия (остаток — забота hoisting-плана, триггер DRIFT-030).
  Дизайн: `spec/design/lane-composition-dedup.md` (approved).
- **B-031 одобрен** («1. координаты: группа org.vibevm.core, имя vibevm.
  2. жесткая ошибка с подсказкой 3. все живые поверхности») + личное
  задание боссу — проверка метаданных рефакторинга — исполнено и
  записано дизайном §5.1 (реестр ключуется путями; specmap
  регенерируется; 126 пинов `~rN` инертны; ledger soft-miss'ит; посадка
  несёт mirror → спот-чек → mass re-seal). Дизайн:
  `spec/design/host-as-package.md` (approved).
- **B-028 рулинг**: «Я хочу чтобы указание версий было опциональной
  фичей. Если версия не указана - используется самая свежая» — прочитано
  как «свежайшая УСТАНОВЛЕННАЯ» (semver-newest слот; единственное
  детерминированное офлайн-чтение) и так записано во всех носителях.
- **НОВЫЙ МАНДАТ**: «Хочу все остальные волны сделать» — Б/В/Г целиком;
  T/F/G — вне добра; публикация — после конца рефакторинга; версии не
  бампать до пред-публикационной границы.

## Что построено (карта посадки за сессию)

- **vibe-workspace:** `desubstitute_covered_units` (bootgen.rs; чистый
  проход, страж покрытия, де-субституция/заглушка) + арка `elided` в
  render_static + `unit_substituted`/`elided` в BootEntry/DependencyBoot;
  тесты `tests/lane_dedup.rs` Т1–Т7.
- **vibe-spec:** `compile_static_qualified` (per-node qualify, вторая
  проходка межузловых ссылок, `AmbiguousShortLink`; pipeline/tests.rs
  вынесен); `SelfCoordinate` + self-ветка `spec_root` +
  `LegacyHostAuthority`/`SelfCoordinateVersioned` (UnknownHost мёртв);
  `resolver/version_order.rs` (freshest, без крейта semver) + F1–F6.
- **vibe-core:** `ProjectSection.group: Option<Group>`; корневой
  `vibe.toml` несёт `group = "org.vibevm.core"`.
- **vibe-cli:** константа HOST_NAMESPACE мертва; поле модели
  `self_coord` (wire-ключ `host_namespace` сохранён serde-rename).
- **Миграция:** `campaigns/packages-2026-09/tasks/migrate-b031.py`
  (байтовый, dry-run/wet/verify, идемпотентный) — уже отработал;
  `--verify` residue 0 — стоячая проверка.
- **Контракты:** PROP-009 §2.3 `##STATIC-EMITS-ONCE-EACH`; PROP-038
  §2.1 elides-фраза; PROP-035 §8 per-node + §6 freshest
  (`##URI-VERSION-OPTIONAL`/`##ROUTER-VERSION`) + §6
  `##ROUTER-SELF-COORDINATE`; PROP-029 §4 — SCOPE-HOST void-надгробие +
  `##SCOPE-SELF-COORDINATE` + changelog B-031.
- **Пакеты:** флоу addressable-specs `{#uri-scheme}` — полная грамматика
  (якоря строк живы, +GROUP-NAME/VERSION/REVISION-PIN+lead); redbook
  гл.1/гл.2 цитируют секцию; core-ai-native LEDGER-INTENT — четыре
  interim'а (B-020/B-015-ключи); ENGINE-CONFORM §2 — честная глубина
  ts-tsc + деферрал B-023 дословно.

## Уроки сессии (вписаны в durable: WAL #constraints + закон §8)

- **Real exits:** exit-коды читаются настоящими, не через pipe/grep
  (оплачено: красный доктест прочитан как зелёный; панель поймала).
- **Файловый бюджет в пакетах:** `wc -l` ≤ 600 на тронутый .rs — в
  self-verify каждого код-пакета (§8, четвёртый оплаченный факт).
- **fmt/vendor-охват:** хостовый fmt не достаёт восемь пакетных
  workspace'ов; после правок пакетных крейтов — fmt по манифестам +
  sync-engines + rematerialise.
- **Watcher-ловушка:** греп лога воркера по маркеру ловит текст ПАКЕТА
  в первом событии — вахта по `"type":"result"`.
- **python open('/tmp/…')** на Windows пишет в корень диска — скретчпад.
- **Legacy-фикстура** держится `concat!`-склейкой — мигратор её не
  тронет; не «чинить».
- **Seal-отказы** по файлам с несуженными якорями — честное состояние
  (четыре файла ждут судейского захода), не ошибка.

## Очередь владельца

Пусто блокирующего. Стоячее: одиннадцать пер-строчных развилок карты
(по одной, по мере подхода batch'ей), открытые строки аудита
(`AUDIT.md` §2026-08-03), DBT-0023, MT-02/MT-03, пред-публикационная
граница (минт+публикация = одна операция), known-issues WAL (ratchet
42, стейл-описание в package-tree.schema).

## Где стоит работа

- `main`, оба зеркала синхронны (последний фан-аут в этой записи —
  см. `git log`; роллаут — ТОЛЬКО `cargo xtask mirror`). Дерево чистое.
- Панель зелёная — «self-check: all green» прочитан хвостом.
- Реестр: 88/179, owed 6, resolved 142 — меряй командами.
- Архив воркеров: `C:\Users\olegc\git\v\cache\agents\sorted\
  {E4-W1-LANE-DEDUP,E4-W2-NODE-QUALIFY,E5-B031-SWEEP,E6-W1-SELF-COORD,
  E6-W2-MIGRATE-SCRIPT,E7-W1-FRESHEST}\` — пакет+лог+штампованный
  отчёт+meta с вердиктом в каждом.

## Карта чтения новой сессии

`CLAUDE.md` → бут по INDEX → `spec/WAL.md` (констрейнты — ВЕСЬ список;
там теперь self-координата, freshest, once-each, real-exits,
package-fmt) → этот файл → `campaigns/packages-2026-09/
SUBAGENT-LAUNCHERS.md` ЦЕЛИКОМ (§8 — четыре факта) + SUBAGENT-MODE.toml
перед КАЖДЫМ fan-out → `TOOLING-MAP.md` §4–§5 (порядок волн + одиннадцать
развилок) → `BACKLOG.md` B-029/B-034/B-039 → план §5E + §7 LOG с конца
(записи 2026-08-04: мандат — последняя).

## Недавняя цепочка коммитов (сессия, сверху — свежие)

```
566ca667 docs(campaign): wave А closes whole
0a2cf315 docs(campaign): B-028 closes — ruled and executed the same hour
addbc78c docs(spec): one grammar, one home — versions optional
eb78b62c feat(vibe-spec): unversioned address → freshest installed
5e811924 docs(campaign): B-031 closes — landed, measured, re-sealed
ec41e71b fix(core-ai-native): grammar doctests follow the coordinate form
38b44dc4 fix(core-ai-native): grammar tests read the coordinate form
81e6d834 style(vibe-install): spec-attribute import follows its users
14841cd4 fix(vibe-install): deviates speak spec://, imports trim
28eb8617 refactor(vibe-install): three files split at seams (600 budget)
497d547d fix(vibe-spec): legacy-form fixture migration-proof (concat!)
8a92ed4b style(packages): migration's fmt tail across package workspaces
e25b2dc5 docs(spec): host exemption dies with a tombstone
23162e3f refactor(spec): the host authority migrates (1893/606, residue 0)
0d2c6eef feat(vibe-spec): the host resolves as its own package coordinate
61641267 feat(campaign): the B-031 migrator
0780b72a docs(campaign): owner rules B-031
51eb17ed docs(campaign): B-031 census + sketch recorded
5b284399 docs(design): B-031 sketch on a measured census
de070928 docs(campaign): file-length budget joins packet self-verify
aa740348 refactor(vibe-spec): grown files split along feature seams
94c9d2db docs(campaign): B-006 closes
d45a49d8 feat(vibe-spec): normal closures qualify per node
e7bf3349 feat(boot): the host lane emits the git family once
68529118 feat(vibe-workspace): the lane composes once-each
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # exit — настоящий, хвостом; фоном — bare-форма
python campaigns/packages-2026-09/tasks/migrate-b031.py --verify  # residue 0
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
