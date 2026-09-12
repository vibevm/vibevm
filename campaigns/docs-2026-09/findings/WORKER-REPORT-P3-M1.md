# WORKER-REPORT-P3-M1 — обязательства документации: реестры, индекс, имена

Пакет: `campaigns/docs-2026-09/findings/PACKET-P3-M1.md` (+ общая часть
`PACKET-P3-M-COMMON.md`), атом A3.13. Дата: 2026-09-12. Без push.

## Коротко для оркестратора

Десять документов пройдены, поставлено **138 пометок** `action="continue"
actionstage="doc"`: **109 маркеров `user`**, **47 `author`**, `dev` — 0,
`agent` — 0 (109 + 47 = 156 = 138 пометок + 18 пометок с двумя аудиториями).
Скрипт — `campaigns/docs-2026-09/findings/P3-M1-mark.py`, прогон
воспроизводим с нуля. Правка ровно одна на факт: строка
` action="continue" actionstage="doc" audience="…"` сразу после `status="…"`;
ни один другой байт не изменён (проверено построчно, см. гейт 4).

Одно отклонение и одна находка по инструменту — §3.

## 1. Решения

**Критерий.** Помечал факт, если человек этой аудитории **столкнётся с ним
сам**: команда и её обязательный результат, поле манифеста или lock-файла,
которое он пишет или читает, значение по умолчанию, на которое он полагается,
отказ и его причина, ограничение, которое его остановит. Не помечал
обоснования, историю, отвергнутые альтернативы, внутренние инварианты
(крейты, трейты, точки инъекции, механику проекции индекса), всё со статусом
не `done` и всё, что документ сам называет «specified, not shipped».

**Аудитории.**

- `user` — тот, кто настраивает реестры, ставит и снимает пакеты, читает
  lock-файл, уходит в offline, держит собственный приватный реестр или
  redirect-заглушку. Сюда же владелец организации-реестра: он остаётся
  оператором `vibe`, а не автором пакета.
- `author` — тот, кто раскладывает и публикует пакет: раскладка репозитория,
  тег версии, токены публикации, `[provides]` / `[obsoletes]` / `[conflicts]`,
  bridge-пакеты, embedded-источники, имя репозитория из координаты.
- `user,author` — факты идентичности и грамматики (`group`, pkgref, identity
  tuple, `[requires]`): обе стороны пишут их руками.
- `dev` — **ноль**. Ни один документ пакета не несёт того, что обязаны
  рассказать три страницы архитектуры (слои крейтов, швы, закон wire-форматов,
  карта трассируемости, правила коммитов). Устройство git-бэкенда, проекции
  индекса и резолвера — ровно тот случай, когда по умолчанию не помечают.
- `agent` — **ноль**. Ни один из десяти документов не про boot-лейн, скилл,
  `explain`/`query`/`select` или законы текста для агентов.

**Подсказка по цитатам использована как подсказка, не как автомат.** Сверил
свой список с 56 `rule ref="…"` из `vibevm-docs`, попадающими в мои документы, и
с аудиторией цитирующей страницы (`<status stage="doc" … audience="…"/>`).
Совпадение почти полное; расхождения осознанные:

- **процитировано, но не помечено:** `PROP-010#USER-LEVEL-REGISTRIES` и
  `PROP-010#HASH-INTEGRITY-GATE` — статусы `spec/work` и `impl/work`, помечать
  недоделанное запрещено (страницы `howto/use-a-private-registry` и
  `model/lock-and-store` уже на них ссылаются — это вопрос центральной сессии,
  не мой);
  `PROP-008#ROW-SHORT-BEHAVIOUR`, `PROP-008#HASH-UNCHANGED`,
  `PROP-002#CACHE-CANONICAL-ROOT` — дублируют помеченный соседний факт (§4).
- **помечено без цитаты:** команды `vibe cache …`, флаги `--offline` /
  `--prefer-local` / `--no-default-registry`, словарь `[provides]`/`[requires]`,
  лестница токенов публикации, грамматика git-source. Страницы A3.5–A3.11 их
  ещё не цитируют — по плану покрытие доводится позже.

**Тонкие места, решённые явно.**

- **Строки таблиц.** Помечал только там, где ячейка несёт контракт целиком
  (`REG-FIELD-*`, `ROW-QUALIFIED-*`); четыре ячейки «что делает vibevm» в
  таблице режимов `auth` не помечены — их содержание уже несут `AUTH-REGIMES`
  и `AUTH-AWARE-401`.
- **Дубликаты между документами.** `--offline` описан и в PROP-002 (раздел
  `#offline-local`), и в PROP-010 §2.5. Помечен PROP-010 (его дом по теме и его
  якоря цитирует руководство), PROP-002 — нет. Так же TTL кэша: помечен
  `PROP-001#FRESHNESS-TTL` (§2.5 PROP-001 явно не superseded),
  `PROP-002#CACHE-TTL` — нет.
- **PROP-016** — документ про зеркала исходников самого vibevm, то есть про
  мейнтейнера. Единственная пометка — `ORTHOGONALITY-LAW`: пользователь обязан
  не спутать зеркало исходников с реестром пакетов, иначе настроит не то.
- **PROP-005** — восемь пометок на 384 факта `done`. Наружу из индекса торчит
  ровно столько: он опционален, он умеет молча отсутствовать и громко
  отказывать, его адрес настраивается тремя ступенями, репозитории остаются
  истиной, и без него не работает `vibe search`. Остальное — устройство.

## 2. Гейты

**1. `target/debug/vibe.exe facts check --exhaustive` — до и после.**

```
до:    progress check: clean (380 files, 24 warning(s))     exit=0
после: progress check: clean (372 files, 24 warning(s))     exit=0
```

Ошибок 0 и предупреждений 24 — без изменений. Разница в числе файлов **не моя**:
это соседний воркер убрал/перегенерировал фикстуру
`vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/examples/none/tree/registry/…`
(8 файлов). Построчный `diff` двух выводов даёт ровно эти строки и строку итога;
ни один из моих десяти файлов в разнице не участвует.

**2. `target/debug/vibe.exe progress report --view doc --audience <a>` —
число `<marker …>`.**

Глобальные числа в дереве **не атрибутируемы**: рядом коммитят M2…M5, и между
двумя моими замерами `user` вырос с 196 до 307 без единой моей правки. Поэтому
приведены обе величины — глобальная (на момент замера, справочно) и мой вклад
(маркеры, чей `path` — один из моих десяти файлов):

| аудитория | мой вклад до | мой вклад после | глобально после (справочно) |
|---|---|---|---|
| `user`   | 0 | **109** | 307 |
| `author` | 0 | **47**  | 217 |
| `dev`    | 0 | **0**   | 5   |
| `agent`  | 0 | **0**   | 40  |

«До» — ноль по построению: ни один факт в моих десяти документах не нёс
`action=`/`actionstage=` (проверено отдельным проходом, §5).

**3. `python campaigns/docs-2026-09/tasks/zone-gate.py`.**

Финальный прогон:

```
zone gate: clean                                               exit=0
```

Путь к этому результату стоит записать, он содержит находку. Первый прогон дал
три строки: одну в `P3-M1-mark.py` (шаблон `private infra` поймал имя якоря
спеки — см. О-2) и две в чужом отчёте `WORKER-REPORT-P3-M5.md`, который цитировал
мою строку. Свою я убрал вместе с пометкой; чужой файл не трогал — к моменту
финального прогона M5 поправил его сам. Отдельно прогнал все четыре шаблона
гейта (`scratch path`, `ip address`, `token-like`, `private infra`) по своим
двум файлам зоны — `P3-M1-mark.py` и этому отчёту: `CLEAN`, exit=0. Эта
проверка нашла в черновике отчёта ещё одно ложное срабатывание — номер
подраздела вида `§a.b.c.d` читается шаблоном как IP-адрес; в тексте он заменён
именем якоря.

**4. `git diff --stat` по моим десяти файлам.**

```
 .../vibespecs/common/PROP-016-source-mirrors.xml   |   2 +-
 .../common/PROP-029-fully-qualified-addresses.xml  |  10 +-
 .../modules/vibe-index/PROP-005-package-index.xml  |  16 +--
 .../modules/vibe-registry/PROP-001-git-backend.xml |   6 +-
 .../PROP-002-decentralized-registry.xml            | 122 ++++++++++---------
 .../vibe-registry/PROP-008-qualified-naming.xml    |  34 +++---
 .../vibe-registry/PROP-010-local-package-cache.xml |  36 +++---
 .../vibe-registry/PROP-021-submodule-sources.xml   |  10 +-
 .../vibe-registry/PROP-023-bridge-packages.xml     |  16 +--
 .../vibe-registry/PROP-030-embedded-registry.xml   |  24 ++--
 10 files changed, 138 insertions(+), 138 deletions(-)
```

138 изменённых строк = 138 пометок. Пары `-`/`+` симметричны потому, что
атрибут вставляется **внутрь существующей строки**, а не новой строкой.
Построчная проверка всех 138 пар (каждая `+`-строка минус ровно одна вставка
даёт байт в байт исходную `-`-строку, и вставка стоит непосредственно после
`status="…"`):

```
changed lines: 138
audience tallies: [('author', 47), ('user', 109)]
VERDICT: CLEAN — every changed line is exactly one attribute insertion
exit=0
```

**5. XML.** Скрипт после правок парсит каждый тронутый файл через
`xml.etree.ElementTree`; падения нет. Файлы читаются и пишутся с
`newline=""` — переводы строк (везде LF, BOM нет) сохранены побайтово.

## 3. Отклонения

**О-1. Превышение ориентира 5–15 % на пяти документах.** Ориентир — доля
помеченного от фактов `done`. Итог по пакету — **10,8 %** (138 из 1279), внутри
полосы. По документам: PROP-010 17,5 %, PROP-029 16,7 %, PROP-002 16,5 %,
PROP-030 16,0 %, PROP-008 15,9 %, PROP-023 15,4 %. Причина одна и та же:
это документы, у которых наружная поверхность и есть почти всё содержание —
команды `vibe cache`, поля `[[registry]]`, грамматика pkgref, флаги `--offline`
/ `--prefer-local`, ключи lock-файла. Урезать их до 15 % пришлось бы, снимая
факты, которые проходят критерий буквально. Компенсируется PROP-005 (2,1 %),
PROP-001 (3,3 %) и PROP-016 (3,8 %) — три документа про устройство.

**О-2. Один проходящий критерий факт не помечен из-за гейта.** PROP-005 §2.3
несёт рядом с помеченным `##REPOS-AUTHORITATIVE` второй факт — «если индекс
расходится с действительностью, побеждает действительность». Он подходит под
критерий, но **имя его якоря содержит слово, которое шаблон `private infra`
гейта зоны ловит как имя VPN-протокола**, и любая строка скрипта или отчёта,
называющая этот якорь, роняет `zone-gate.py` (exit=1). Первый прогон так и
упал; я снял пометку, а не стал прятать имя разбиением строки — маскировка
приватного гейта хуже потери одной пометки. Смысл при этом почти не потерян:
помеченный `##REPOS-AUTHORITATIVE` уже говорит, что репозитории пакетов —
источник истины, а индекс — производный горячий кэш.

*Рекомендация центральной сессии (решение не моё):* сузить шаблон так, чтобы
он не срабатывал внутри имени якоря спеки (например, требовать, чтобы за словом
не следовал дефис с продолжением), и тогда вернуть пометку одной строкой в
`P3-M1-mark.py`. Ту же коллизию независимо заметил воркер M5. Рядом стоит
второе ложное срабатывание того же рода: шаблон `ip address` ловит ссылку на
подраздел вида `§a.b.c.d` — а такие ссылки в отчётах по спекам неизбежны.

**О-3. Ничего не собирал.** `cargo` не запускался; все гейты — через готовый
`target/debug/vibe.exe`.

## 4. Таблица «сомнения» — рассмотрено и не помечено

| Документ · якорь | Почему не помечено |
|---|---|
| PROP-002 `##TRUST-MIRROR-HATCH` | `spec/done`, но текст сам говорит «specified, not shipped» — флага нет |
| PROP-002 `##OFFLINE-LOCAL-ONLY` | дублирует помеченный `PROP-010#OFFLINE-LOCAL-ONLY` |
| PROP-002 `##CACHE-TTL`, `##CACHE-CANONICAL-ROOT` | дублируют `PROP-001#FRESHNESS-TTL` и `PROP-010#THE-STORE-IS-DOT-VIBE-CACHE` |
| PROP-002 `##ROW-AUTH-*-WHAT-VIBEVM-DOES` (4 ячейки) | содержание несут `##AUTH-REGIMES` и `##AUTH-AWARE-401` |
| PROP-002 `##NONE-DEFAULT-SAFE` | пересказ `##AUTH-AWARE-401` без нового контракта |
| PROP-002 `##ERR-401-403` и ещё 5 строк `##ERR-*` | тексты сообщений; пометка обязала бы цитировать их дословно |
| PROP-002 `##GS-ARRAY-MIGRATION` | миграция со старой формы массива, историческое |
| PROP-002 `##MIR-PRIORITY-ORDER`, `##MIR-CANONICAL-FALLTHROUGH` | следуют из `##MIRROR-LAYER` и поля `priority` |
| PROP-002 `##REDIRECT-AUTH-LAYERS` | нужен только владельцу реестра с приватной целью; пограничный |
| PROP-002 `##REG-FIELD-NAME`, `##REG-FIELD-REF` | `name` — служебный алиас, `ref` зарезервирован и не читается |
| PROP-005 §2.3, второй факт | см. О-2 (гейт зоны) |
| PROP-005 `##INT-OUTDATED-FAST`, `##REPOMD-TRUST-POINT` | производительность и внутренняя точка доверия каталога |
| PROP-005 §2.11 (`vibe-index` CLI, ~30 фактов) | отдельный бинарь оператора реестра; руководство его не ведёт |
| PROP-008 `##ROW-SHORT-BEHAVIOUR` | дублирует помеченный `##INDEX-DEPENDENCY` |
| PROP-008 `##HASH-UNCHANGED` | следует из `##IDENTITY-TUPLE`; вклад `group` в хэш — деталь |
| PROP-008 `##SEGMENT-COUNT-IS-REGISTRY-POLICY` | политика конкретного реестра, а не контракт ядра |
| PROP-008 `##INSTALLED-STATE-RESOLVES-LOCALLY` | удобство `uninstall`/`update` без сети; пограничное |
| PROP-010 `##LAYOUT-EXTRACTED-DIRECTORIES` | раскладка стора; пользователь ходит туда через `vibe cache list` |
| PROP-010 `##CMD-PATH`, `##CMD-CHECK-REPAIR` | `path` — вывод одного пути, `--repair` — флаг помеченного `##CMD-CHECK` |
| PROP-010 `##LAYER-CACHE/VIBEDEPS/LOCK` | «что коммитить» — дом этого факта PROP-009, не здесь |
| PROP-016 `##SPLIT-REGISTRY`, `##CRED-SEPARATION` | файл токена публикации канонично описан в PROP-002 `##TOK-PER-HOST-FILE` |
| PROP-021 `##EMBED-SNAPSHOT`, `##EMBED-IN-PLACE` | режимы материализации — дом PROP-022 |
| PROP-023 `##INDEPENDENT-REPOSITORY-DEFAULT` | процесс внутри проекта («спросить владельца»), не контракт продукта |
| PROP-023 `##SKILL-VIA-INCLUDE` | селектор `include` — дом PROP-015 |
| PROP-029 `##SCOPE-SELF-COORDINATE` | статус `spec/work` |
| PROP-029 `##ADDR-NO-BARE-ON-DISK` | следствие помеченного `##ADDR-LAW` |
| PROP-001 `##SELECTION-RULE`, `##CATCH-ALL` | приоритет `--registry` живее описан в PROP-030; `CATCH-ALL` — текст ошибки |

## 5. Факты с уже стоящим `action=` / `actionstage=`

**Пусто.** Отдельный проход по всем десяти файлам до правок: 1279 фактов
`done`, из них несущих `action=` или `actionstage=` — **0**. Скрипт, кроме
того, падает на любом якоре, который уже нёс бы такой атрибут, так что
случайно перекрыть чужую пометку он не может.

## 6. Числа по документам

| Документ | фактов `done` | помечено | доля | `user` | `author` |
|---|---:|---:|---:|---:|---:|
| `modules/vibe-registry/PROP-002-decentralized-registry.xml` | 370 | 61 | 16,5 % | 48 | 20 |
| `modules/vibe-index/PROP-005-package-index.xml` | 384 | 8 | 2,1 % | 8 | 0 |
| `modules/vibe-registry/PROP-010-local-package-cache.xml` | 103 | 18 | 17,5 % | 18 | 0 |
| `modules/vibe-registry/PROP-008-qualified-naming.xml` | 107 | 17 | 15,9 % | 15 | 10 |
| `modules/vibe-registry/PROP-030-embedded-registry.xml` | 75 | 12 | 16,0 % | 12 | 0 |
| `modules/vibe-registry/PROP-001-git-backend.xml` | 90 | 3 | 3,3 % | 3 | 0 |
| `modules/vibe-registry/PROP-021-submodule-sources.xml` | 42 | 5 | 11,9 % | 0 | 5 |
| `modules/vibe-registry/PROP-023-bridge-packages.xml` | 52 | 8 | 15,4 % | 1 | 8 |
| `common/PROP-016-source-mirrors.xml` | 26 | 1 | 3,8 % | 1 | 0 |
| `common/PROP-029-fully-qualified-addresses.xml` | 30 | 5 | 16,7 % | 3 | 4 |
| **Итого** | **1279** | **138** | **10,8 %** | **109** | **47** |

(Колонки `user` и `author` считают маркеры, поэтому их сумма — 156: восемнадцать
пометок несут обе аудитории.)

## 7. Таблица пометок

Все пути — относительно `vibevm/vibespecs/`.

### `modules/vibe-registry/PROP-002-decentralized-registry.xml` — 61

| Якорь | Аудитории | Почему |
|---|---|---|
| `SHAPE-OWN-REPO` | user,author | Пакет — отдельный репозиторий; права даёт хостинг |
| `SHAPE-REGISTRY-ARRAY` | user,author | `[[registry]]` массив, `[[mirror]]`, `[[override]]` существуют |
| `SHAPE-PLAIN-GIT-URL` | user | URL реестра — обычный git-URL, сокращений хоста нет |
| `IDENTITY-TUPLE` | user,author | Что такое идентичность пакета и его content_hash |
| `IDENTITY-CONSEQUENCE` | user | Расхождение байтов между источниками — фатальная ошибка |
| `EFF-LOCKFILE-STABLE` | user | Смена зеркала или хоста не меняет lock-файл |
| `EFF-MIRROR-SUBSTITUTION` | user | Подменённое зеркало падает до записи на диск |
| `EFF-FORCE-PUSH-CAUGHT` | user,author | Переставленный тег ловится на следующей установке |
| `REGISTRY-ARRAY` | user | Форма блока `[[registry]]` в `vibe.toml` |
| `REG-FIELD-URL` | user | `url` — корень организации, не репозиторий пакета |
| `REG-FIELD-NAMING` | user | Значения `naming` и что `fqdn` — умолчание |
| `REGISTRY-WALK-ORDER` | user | Обход по порядку; версии между реестрами не объединяются |
| `AUTH-REGIMES` | user | Поле `auth` и его четыре значения |
| `TOKEN-ENV-DEFAULTING` | user | Как выводится имя переменной токена реестра |
| `TOKEN-NEVER-ON-DISK` | user | Токен не попадает в lock-файл и вывод |
| `GLOBAL-REGISTRY-FILE` | user | Машинный `~/.vibe/registry.toml` несёт те же секции |
| `MERGE-PROJECT-FIRST` | user | Проект выигрывает у машинных настроек при совпадении имени |
| `ENABLED-FLAG` | user | `enabled = false` выключает реестр, не удаляя запись |
| `MIRROR-LAYER` | user | Что такое `[[mirror]]` и когда он опрашивается |
| `MIR-CANONICAL-IN-LOCKFILE` | user | В lock-файл пишется канонический URL, не зеркало |
| `MIRROR-INTEGRITY-MANDATORY` | user | Проверка зеркала обязательна и не отключается |
| `REGISTRY-WALK-SEMANTICS` | user | Какой отказ реестра останавливает установку, а какой нет |
| `AUTH-AWARE-401` | user | 401 на публичном реестре — «нет пакета», на приватном — ошибка |
| `MIRROR-WALK-SEMANTICS` | user | Зеркало проваливается дальше по любой недоступности |
| `OVERRIDE-SHORT-CIRCUIT` | user | `[[override]]` обходит слой реестров целиком |
| `OVERRIDE-SEMANTICS` | user | Override не ослабляет целостность; в lock — `overridden` |
| `GIT-SOURCE-DECL` | user | Зависимость можно объявить прямо из git-репозитория |
| `GS-WIRE-FORM` | user | `[requires.packages]` — таблица; строка либо инлайн-таблица |
| `GS-EXACTLY-ONE-REF` | user | Ровно один из `tag`/`rev`/`branch`, иначе отказ разбора |
| `GS-RESOLUTION-ORDER` | user | Порядок: override, git-source, реестр |
| `GS-IDENTITY` | user | У git-source та же идентичность по содержимому |
| `GS-IDENTITY-VERIFICATION` | user | Чужой репозиторий не может выдать себя за другой пакет |
| `GS-MUTABILITY` | user | `install` не гонится за веткой, `update` — гонится |
| `GS-AUTH-EXPLICIT` | user | `auth` git-source не наследуется от реестра того же хоста |
| `SOURCE-KIND-VALUES` | user | Значения `source_kind` в lock-файле |
| `REDIRECT-STUB` | user | Реестр может держать заглушку вместо содержимого |
| `REDIRECT-MARKER-FILE` | user | Заглушка несёт `vibe-redirect.toml` вместо манифеста |
| `RD-STEP-HOP-LIMIT` | user | Цепочки редиректов запрещены, один переход |
| `REDIRECT-TAG-VISIBILITY` | user | Версии в организации задаются тегами заглушки |
| `REDIRECT-SYNC-HELPER` | user | `vibe registry redirect-sync` переносит теги цели |
| `REDIRECT-LOCKFILE-FIELD` | user | Поле `via_redirect` в lock-файле |
| `REDIRECT-CLI` | user | Команда `vibe registry redirect` и её флаги |
| `RD-TRUST-FLAG` | user | `--trust-redirect` — принять смену цели осознанно |
| `FLAT-LAYOUT` | author | Содержимое пакета лежит плоско в корне репозитория |
| `LAYOUT-TAG-VERSION` | author | Версия — тег `v<semver>`, метка подвижная |
| `LOCKFILE-V2` | user | Форма записи пакета в `vibe.lock` |
| `LF-ROOT-DEPENDENCIES` | user | Снять чисто транзитивную зависимость нельзя |
| `CAP-PROVIDES` | author | `[provides].capabilities` — что пакет объявляет |
| `CAP-REQUIRES-PACKAGES` | user,author | `[requires].packages` — основная форма зависимости |
| `CAP-REQUIRES-CAPABILITIES` | user,author | Зависимость по способности, а не по имени |
| `CAP-REQUIRES-ANY` | user,author | `[[requires_any]]` — дизъюнкция «ровно один из» |
| `CAP-OBSOLETES` | author | `[obsoletes]` помечает вытесняемые пакеты |
| `CAP-CONFLICTS` | author | `[conflicts]` — взаимно исключающие установки |
| `PUBLISH-UTILITY` | author | `vibe registry publish <path>` и границы его работы |
| `PUBLISH-MUTABLE-VERSIONS` | author | Повторная публикация версии — обычная замена |
| `PUB-TOKEN-LOADING` | author | Порядок источников токена публикации |
| `TOK-HOST-ENV-VAR` | author | Переменная токена на хост и её старшинство |
| `TOK-PER-HOST-FILE` | author | Файл токена на хост в каталоге настроек |
| `TOKEN-SECRECY-INVARIANT` | author | Куда токен попадает и куда не попадает никогда |
| `PUB-ADAPTER-SELECTION` | author | Адаптер выбирается по хосту; чужой хост — ошибка |
| `PUBLISH-NEVER-RULES` | author | Чего публикация не делает: rewrite, force, чужая организация |

### `modules/vibe-index/PROP-005-package-index.xml` — 8

| Якорь | Аудитории | Почему |
|---|---|---|
| `INDEX-OPTIONAL` | user | Индекс необязателен; без него работает живой путь |
| `A-PROBE-HAS-THREE-OUTCOMES-NOT-TWO` | user | Отказ индекса не молчит, в отличие от отсутствия |
| `AN-ABSENT-INDEX-FALLS-BACK-WITHOUT-A-WORD` | user | Отсутствующий индекс — тишина и откат на `ls-remote` |
| `INDEX-URL-CONFIG` | user | Ключ `index_url`, включая значение `none` |
| `INDEX-URL-DEFAULT` | user | Куда указывает индекс, если ключ не задан |
| `INDEX-URL-TODAY-IS-AN-ENVIRONMENT-VARIABLE` | user | Переменная перекрывает ключ манифеста |
| `REPOS-AUTHORITATIVE` | user | Репозитории — истина, индекс — производный кэш |
| `INT-SEARCH` | user | `vibe search` существует только поверх индекса |

### `modules/vibe-registry/PROP-010-local-package-cache.xml` — 18

| Якорь | Аудитории | Почему |
|---|---|---|
| `CACHE-MACHINE-GLOBAL` | user | Стор один на машину и переезжает с домом настроек |
| `CACHE-ACCRETIVE` | user | Версия из стора не вытесняется автоматически никогда |
| `EXPLICIT-RECLAIM` | user | Место освобождает только явная команда оператора |
| `THE-SETTINGS-HOME-IS-DOT-VIBE-NOT-XDG` | user | Дом настроек — `~/.vibe`, а не XDG |
| `THE-STORE-IS-DOT-VIBE-CACHE` | user | Стор лежит в `~/.vibe/cache/` |
| `REGISTRIES-KEEP-THEIR-OWN-FILE` | user | Реестры в отдельном файле — его можно отдать коллеге |
| `PROJECT-OVERRIDES` | user | Проектные реестры всегда выигрывают у машинных |
| `OFFLINE-FLAG` | user | Глобальный `--offline` запрещает сеть на запуск |
| `OFFLINE-LAYERING` | user | Флаг, `VIBE_OFFLINE`, ключ `[net]` — в этом порядке |
| `OFFLINE-LOCAL-ONLY` | user | Что считается локальным источником в offline |
| `OFFLINE-HARD-ERROR` | user | Отсутствие локально — жёсткая ошибка с рецептом |
| `OFFLINE-NO-DEGRADE` | user | Offline не деградирует до частичного результата |
| `AS-OF-LAST-REFRESH` | user | Offline-резолв может выбрать версию старее онлайнового |
| `A-CACHE-HIT-IS-AUTHORITATIVE-FOR-AVAILABILITY` | user | Пакет из стора ставится, даже если исчез из реестра |
| `CMD-LIST` | user | `vibe cache list` — что доступно без сети |
| `CMD-ADD` | user | `vibe cache add` — прогрев перед уходом в offline |
| `CMD-CLEAN` | user | `vibe cache clean` — освобождение места |
| `CMD-CHECK` | user | `vibe cache check` — единственная полная сверка хэшей |

### `modules/vibe-registry/PROP-008-qualified-naming.xml` — 17

| Якорь | Аудитории | Почему |
|---|---|---|
| `GROUP-MANDATORY` | user,author | Поле `group` в `[package]` обязательно |
| `GROUP-GRAMMAR` | user,author | Грамматика группы: доменные метки, без подчёркиваний |
| `IDENTITY-TUPLE` | user,author | Идентичность — `(group, name, version, content_hash)` |
| `NAME-UNIQUE-IN-GROUP` | author | Имя уникально внутри группы, не внутри вида |
| `GROUP-CHANGE-NEW-PACKAGE` | user,author | Смена группы — новый пакет, а не переименование |
| `KIND-METADATA` | user,author | `kind` обязателен, но ничего не идентифицирует |
| `PKGREF-GRAMMAR` | user,author | Грамматика pkgref, которую набирают и пишут |
| `KIND-VALIDATION` | user | Префикс вида проверяется; несовпадение — ошибка |
| `SHORT-CLI-ONLY` | user,author | Короткое имя не попадает в манифест никогда |
| `JOINER-UNDERSCORE` | author | Имя репозитория пакета — `<group>.<name>` |
| `RESOLVE-ONCE-WRITE-QUALIFIED` | user | В `[requires]` пишется полная координата |
| `INDEX-DEPENDENCY` | user | Без индекса короткие имена реестра недоступны |
| `LOCKFILE-AUTHORITATIVE` | user | Короткое имя предпочитает уже зафиксированное в lock |
| `COLLISION-BEHAVIOR` | user | Что происходит при неоднозначном коротком имени |
| `EXIT-CODE-7` | user | Код выхода 7 — «неоднозначный пакет» |
| `GROUP-IS-A-CLAIM` | user,author | Группа — заявление; владение доменом не проверяется |
| `DEFAULT-TRUSTED-REGISTRIES` | user | Доверяются два корня; остальные — только вашим действием |

### `modules/vibe-registry/PROP-030-embedded-registry.xml` — 12

| Якорь | Аудитории | Почему |
|---|---|---|
| `AMBIENT-DEFAULT` | user | Встроенный реестр подставляется без настройки проекта |
| `EXPLICIT-ABOVE` | user | Порядок: override, path, git, слой реестров |
| `KNOB-DEFAULT` | user | Умолчание приоритета следует происхождению установки |
| `KNOB-SUPPRESS` | user | `--no-default-registry` и переменная отключают встроенный |
| `ENUM-UNION` | user | Перечисление версий объединяет встроенный и объявленный |
| `FLAG-EMBEDDED-SHORT-CIRCUIT` | user | Флаг убирает поход в сеть для встроенных координат |
| `LOCAL-AUTO-OPEN` | user | `packages/` проекта открывается сам, без настройки |
| `LOCAL-NO-PREFER-FLAG` | user | `--no-prefer-local` отключает проектные пакеты на команду |
| `LOCAL-SOURCE-KIND` | user | `source_kind = "local"` переносим между машинами |
| `LOCK-EMBEDDED` | user | `source_kind = "embedded"` в lock-файле |
| `GUARD-CI-OFF` | user | В CI и `--frozen` встроенный реестр выключен |
| `GUARD-WARN` | user | `vibe check` предупреждает о непереносимом lock-файле |

### `modules/vibe-registry/PROP-001-git-backend.xml` — 3

| Якорь | Аудитории | Почему |
|---|---|---|
| `RISK-GIT-IN-PATH` | user | Нужен `git` в PATH; без него — понятная ошибка |
| `FRESHNESS-TTL` | user | Кэш реестра свеж час; `registry sync` обновляет принудительно |
| `OPEN-GIT-BINARY-PATH` | user | Переменная `VIBE_GIT_BINARY` задаёт путь к git |

### `modules/vibe-registry/PROP-021-submodule-sources.xml` — 5

| Якорь | Аудитории | Почему |
|---|---|---|
| `WHAT-VVM-DOES` | author | Подмодули пакета забираются и обновляются вместе с ним |
| `FORM-DEPENDENCY-DECLARED` | author | Блок `[[embedded_source]]` и что он обязан нести |
| `NOT-A-PACKAGE` | author | Встроенный репозиторий — не второй пакет и не узел графа |
| `PUBLISH-FLATTENS-GITLINKS` | author | Публикация распрямляет gitlink в обычные файлы |
| `DECLARED-SOURCE-AUTH` | author | Источник сверяется по коммиту и по хэшу дерева |

### `modules/vibe-registry/PROP-023-bridge-packages.xml` — 8

| Якорь | Аудитории | Почему |
|---|---|---|
| `BRIDGE-DEF` | user,author | Что такое bridge-пакет и кто за него отвечает |
| `BRIDGE-FLAG` | author | `[package].bridge = true` помечает пакет мостом |
| `CLASS-VENDORED` | author | Вендоринг: просто файлы, без дополнительной машинерии |
| `CLASS-SUBMODULE` | author | Подмодуль в авторстве, распрямление при публикации |
| `CLASS-REFERENCE` | author | Ссылочная форма: upstream не копируется в репозиторий |
| `AUTHORSHIP-SEPARATION` | author | `authors` и `upstream_authors` не сливаются никогда |
| `GROUP-PROVENANCE` | author | Координата называет продукт, а не упаковщика |
| `LICENSE-BOUNDARY` | author | Лицензия моста и лицензия upstream — разные записи |

### `common/PROP-016-source-mirrors.xml` — 1

| Якорь | Аудитории | Почему |
|---|---|---|
| `ORTHOGONALITY-LAW` | user | Зеркало исходников и реестр пакетов — разные вещи |

### `common/PROP-029-fully-qualified-addresses.xml` — 5

| Якорь | Аудитории | Почему |
|---|---|---|
| `ADDR-LAW` | user,author | Адрес всегда несёт группу и имя, в каждом месте |
| `ADDR-SHORT-NAMES` | user | Короткое имя живёт только как разовый ввод в CLI |
| `CARRIER-PKGREF-FORM` | user,author | Форма pkgref в манифестах и lock-файле |
| `CARRIER-SPEC-URI-FORM` | author | Форма авторитета `spec://` в цитатах |
| `CARRIER-REPO-NAME-FORM` | author | Плоское имя репозитория и где в нём граница |

## 8. Что не сделано

- Вторая пометка в PROP-005 §2.3 — снята из-за ложного срабатывания гейта
  зоны (О-2); решение о шаблоне гейта за центральной сессией.
- `vibe doc check --coverage` не запускал: атом его не требует (по плану
  `--min 0` считает центральная сессия), а сама команда собирается сейчас
  соседним воркером.
