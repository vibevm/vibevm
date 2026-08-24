# F4-JTD-PROBE — что jtd-codegen 0.4.1 на самом деле делает с объединением, словарём и опциональным скаляром

**Чем мерил:** боссов спайк — реальные прогоны `tools/jtd-codegen/jtd-codegen.exe`
(версия по `--version`: `jtd-codegen 0.4.1`) по двум специально написанным
JTD-схемам, вне продуктового дерева (обе схемы и весь выход — в scratchpad
сессии; существенное приведено здесь целиком, потому что каталог одноразовый).
**Что НЕ запускалось:** `cargo` любой, панель, тесты. **Дата:** 2026-08-15.

**Зачем спайк вообще был нужен.** Находка Ф0.2
([`f0-gen-poc.md`](f0-gen-poc.md) §2, строка 1 таблицы) честно записала, что
утверждение базовой линии B11 «`discriminator` → `#[serde(tag)]` РАБОТАЕТ»
**этим выходом не доказано** — в `schemas/list_report.jtd.json` нет
`discriminator`, поэтому поведение просто не упражнялось. Между тем Ф4.1
обязана описать `discriminator`'ом два формата сразу: `repomd.jtd.json`
(тегированное объединение Ф1.5) и `journal.jtd.json` (одиннадцать вариантов
события). Строить фазу на неупражнявшемся утверждении — это ровно та
«фикстура, которая не может упасть», которую §8 запускалок называет самым
дорогим уроком.

## 1. ВЕРДИКТ

`discriminator` **работает**, и все одиннадцать вариантов события выражаются.
Но выход опровергает **три** записанных утверждения, и каждое из трёх меняет
КОНСТРУКЦИЮ Ф4.2, а не её объём:

1. **Постпроцессор Ф4.2 не может быть «Rust-текст → Rust-текст» даже для
   преобразования №1.** Находка Ф0.2 §7 заключила: «Закрытый enum → открытый
   (этот спайк). Чистая постобработка сгенерированного Rust, без второго
   входа. ДОКАЗАНО». Доказана механика, но не вывод: политика открытости
   назначается **пословарно** (план, Приложение Б.1: `PackageKind` —
   открытый, `NamingConvention` — закрытый, `Event.kind` — закрытый,
   `DeliveryMode` — открытый), а в сгенерированном Rust от этой политики нет
   ни следа — оба словаря выходят синтаксически неразличимыми. Постпроцессор,
   открывающий «всякий закрытый enum», откроет `NamingConvention`, что Б.1
   прямо запрещает. Значит **вход из схемы нужен уже преобразованию №1**, а не
   только №3.
2. **Опциональный скаляр приходит как `Option<Box<T>>` — то есть с ТРЕТЬИМ
   состоянием на оси, которую контракт объявил двухсоставной.** Ф1.4 решением
   Р1 отвергла `Option<bool>` для `frozen`/`yanked` именно поэтому
   (PROP-044 [`##TERMS-SNAPSHOT-FROZEN-CHANNEL`](../../spec/common/PROP-044-change-native-formats.xml#laws)
   — «одна булева ось без третьего состояния»). Генератор возвращает это
   состояние сам. Значит у Ф4.2 есть **пятое** преобразование, которого
   Приложение А.5 не называет: свернуть `Option<Box<скаляр>>` в
   `скаляр + #[serde(default, skip_serializing_if = …)]`. Без него замена
   рукописных типов реэкспортом сгенерированных **нарушает ратифицированный
   контракт**.
3. **Варианты тегированного объединения выходят newtype'ами, а не
   структурными.** Приложение А.2 плана рисует `Published { entry: VersionEntry }`;
   генератор даёт `Published(EventPublished)` с отдельной структурой на каждый
   вариант. На проводе это одно и то же (serde внутренне-тегированный newtype
   разворачивает поля рядом с тегом), в Rust — другое: каждое сопоставление в
   проекторе меняет форму. Радиус — плечи `match` по всем одиннадцати
   вариантам.

И **седьмое** преобразование, найденное прогоном clippy (§4a): сгенерированное
объединение роняет `large_enum_variant`, как только его нагрузка настоящего
размера, — тот самый линт, ради которого Ф3.1 боксировала запись вручную. Руками
сгенерированный код не правят, а панель гоняет `clippy --workspace -- -D
warnings`, поэтому боксирование крупного варианта обязан делать генератор.

Плюс два факта, которые НЕ опровержения, а недостающие входы:

4. **`metadata` с ключами `x-…` генератор принимает молча** (exit 0, ни
   предупреждения) и в Rust не переносит. Канал для аннотаций Ф4.2
   (`x-empty`, и по п.1 — политика словаря) **существует и проверен**; читать
   его обязан наш слой, а не jtd-codegen.
5. **`timestamp` даёт `DateTime<FixedOffset>`, дерево живёт на `DateTime<Utc>`**
   (26 вхождений в `crates/vibe-index/src`). Реэкспорт сгенерированного типа
   без шестого преобразования сдвинул бы тип времени по всему крейту.

## 2. Что упражнялось: схема-проба журнала

Схема воспроизводит форму Приложения А.2 плана целиком: внешняя запись
`{at, actor, event}`, `discriminator: "kind"` с **одиннадцатью** вариантами —
включая вариант с ПУСТОЙ полезной нагрузкой (`entry_set_replaced`), вариант с
вложенной структурой (`published` → `version_entry`), вариант с
опциональным полем (`removed.version`) и два вложенных словаря
(`naming_convention`, `package_kind`).

```json
{
  "properties": {
    "at": { "type": "timestamp" },
    "actor": { "type": "string" },
    "event": { "ref": "event" }
  },
  "definitions": {
    "event": {
      "discriminator": "kind",
      "mapping": {
        "initialised": { "properties": { "registry": {"type":"string"}, "registry_url": {"type":"string"},
                                          "naming": {"ref":"naming_convention"}, "generator": {"type":"string"},
                                          "schema_version": {"type":"uint32"} } },
        "published":   { "properties": { "entry": { "ref": "version_entry" } } },
        "frozen":      { "properties": { "group": {"type":"string"}, "name": {"type":"string"},
                                          "version": {"type":"string"}, "content_hash": {"type":"string"} } },
        "yanked":      { "properties": { "group": {"type":"string"}, "name": {"type":"string"},
                                          "version": {"type":"string"}, "reason": {"type":"string"} } },
        "removed":     { "properties": { "group": {"type":"string"}, "name": {"type":"string"} },
                         "optionalProperties": { "version": { "type": "string" } } },
        "renamed":     { "properties": { "from_group": {"type":"string"}, "from_name": {"type":"string"},
                                          "to_group": {"type":"string"}, "to_name": {"type":"string"} } },
        "notice":      { "properties": { "group": {"type":"string"}, "name": {"type":"string"}, "text": {"type":"string"} } },
        "channel_set": { "properties": { "group": {"type":"string"}, "name": {"type":"string"},
                                          "channel": {"type":"string"}, "version": {"type":"string"} } },
        "channel_unset": { "properties": { "group": {"type":"string"}, "name": {"type":"string"}, "channel": {"type":"string"} } },
        "force_replaced": { "properties": { "group": {"type":"string"}, "name": {"type":"string"}, "version": {"type":"string"},
                                             "old_hash": {"type":"string"}, "new_hash": {"type":"string"}, "reason": {"type":"string"} } },
        "entry_set_replaced": { "properties": {} }
      }
    },
    "naming_convention": { "enum": ["fqdn", "flat"] },
    "package_kind": { "enum": ["feat", "flow", "lang", "mcp", "stack", "tool"] },
    "version_entry": {
      "properties": { "group": {"type":"string"}, "name": {"type":"string"},
                       "version": {"type":"string"}, "kind": {"ref":"package_kind"} },
      "optionalProperties": { "keywords": {"elements":{"type":"string"}}, "yanked": {"type":"boolean"},
                               "frozen": {"type":"boolean"}, "must_understand": {"elements":{"type":"string"}} }
    }
  }
}
```

Прогон и его дословный вывод (код выхода 0):

```
📝 Writing Rust code to: …/spike/out
📦 Generated Rust code.
📦     Root schema converted into type: JournalRecord
📦     Definition "event" converted into type: Event
📦     Definition "naming_convention" converted into type: NamingConvention
📦     Definition "package_kind" converted into type: PackageKind
📦     Definition "version_entry" converted into type: VersionEntry
```

## 3. Выход по существу — что именно эмитировано

Шапка файла: `// Code generated by jtd-codegen for Rust v0.2.1` — при бинаре
**0.4.1**. Строка версии в артефакте недостоверна (это уже отмечала Ф0.2 §6);
пинить надо бинарь.

**Объединение — тег на месте, варианты newtype'ы:**

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
    #[serde(rename = "channel_set")]        ChannelSet(EventChannelSet),
    #[serde(rename = "channel_unset")]      ChannelUnset(EventChannelUnset),
    #[serde(rename = "entry_set_replaced")] EntrySetReplaced(EventEntrySetReplaced),
    #[serde(rename = "force_replaced")]     ForceReplaced(EventForceReplaced),
    #[serde(rename = "frozen")]             Frozen(EventFrozen),
    #[serde(rename = "initialised")]        Initialised(EventInitialised),
    #[serde(rename = "notice")]             Notice(EventNotice),
    #[serde(rename = "published")]          Published(EventPublished),
    #[serde(rename = "removed")]            Removed(EventRemoved),
    #[serde(rename = "renamed")]            Renamed(EventRenamed),
    #[serde(rename = "yanked")]             Yanked(EventYanked),
}
```

Одиннадцать из одиннадцати. Порядок вариантов — **алфавитный по строке
провода**, не порядок `mapping` в схеме: генератор сортирует (в схеме
`initialised` первый, в выходе — шестой). Для байт-стабильности это хорошо
(порядок ключей JSON не зависит от порядка написания схемы), для читаемости
диффа — то, о чём надо знать заранее.

Пустой вариант выражается: `pub struct EventEntrySetReplaced {}` — на проводе
`{"kind":"entry_set_replaced"}`.

**Словари — оба закрытые и синтаксически НЕРАЗЛИЧИМЫЕ:**

```rust
#[derive(Serialize, Deserialize)]
pub enum NamingConvention {           // Б.1: должен остаться ЗАКРЫТЫМ
    #[serde(rename = "flat")] Flat,
    #[serde(rename = "fqdn")] Fqdn,
}

#[derive(Serialize, Deserialize)]
pub enum PackageKind {                // Б.1: должен стать ОТКРЫТЫМ
    #[serde(rename = "feat")] Feat,
    …
    #[serde(rename = "tool")] Tool,
}
```

Ровно одна и та же форма. Отличить их постпроцессору **не по чему** — вот
почему вывод Ф0.2 §7.1 «без второго входа» неверен.

**Опциональные поля — `Option<Box<T>>` даже для скаляра:**

```rust
pub struct VersionEntry {
    …
    #[serde(rename = "frozen")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frozen: Option<Box<bool>>,          // ← третье состояние у булевой оси
    #[serde(rename = "yanked")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yanked: Option<Box<bool>>,
    #[serde(rename = "must_understand")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mustUnderstand: Option<Box<Vec<String>>>,
}
```

Сегодняшний рукописный дом — `#[serde(default, skip_serializing_if = "is_false")]
pub yanked: bool` (решение Р1 Ф1.4). Сгенерированный тип не эквивалентен ему:
он ВВОДИТ различие «поля нет» против «поле есть и равно false», которое
PROP-044 §2b объявляет несуществующим.

**Время:** `pub at: DateTime<FixedOffset>` против рукописного
`pub at: DateTime<Utc>` (`crates/vibe-index/src/journal/record.rs:23`).
Сегодня ни один файл под `crates/vibe-wire/src/generated/` не импортирует
`chrono` вовсе — это новая поверхность для крейта провода.

**Имена полей:** идентификаторы camelCase (`newHash`, `contentHash`,
`registryUrl`, `schemaVersion`, `mustUnderstand`) при верной строке провода в
`#[serde(rename)]` — подтверждает B11 №5; закрывается преобразованием А.5 №2.

## 4. Вторая проба: канал аннотаций

Схема с `metadata` на словарях, на коллекции и на скаляре:

```json
{
  "properties": {
    "kind":   { "ref": "package_kind" },
    "naming": { "ref": "naming_convention" },
    "files":  { "elements": { "type": "string" }, "metadata": { "x-empty": "emit" } }
  },
  "optionalProperties": {
    "keywords": { "elements": { "type": "string" }, "metadata": { "x-empty": "omit" } },
    "yanked":   { "type": "boolean", "metadata": { "x-default": "false" } }
  },
  "definitions": {
    "package_kind":      { "metadata": { "x-vocabulary": "open"   }, "enum": ["feat","flow","lang","mcp","stack","tool"] },
    "naming_convention": { "metadata": { "x-vocabulary": "closed" }, "enum": ["fqdn","flat"] }
  }
}
```

Результат: **exit 0, ни одного предупреждения**, выход побайтово такой же, как
без аннотаций. То есть:

- канал для `x-empty` (А.5 №3) **проверен и работает**;
- канал для политики словаря (нужный по п.1 вердикта) — тот же самый, тоже
  работает;
- цена: `jtd-codegen` их **не переносит** в Rust, поэтому конвейер Ф4.2
  обязателен — `(схема + реестр + сгенерированный Rust) → Rust`, как и
  заключала Ф0.2 §7 для преобразований №3 и №4. Новое здесь только то, что
  преобразование №1 попадает в тот же класс.

## 4a. Третья проба: clippy — и фикстура, которая сначала НЕ МОГЛА упасть

Вопрос: сгенерированное объединение держит нагрузку в отдельной структуре
варианта по значению (`Published(EventPublished)`), тогда как рукописный тип
боксирует её явно (`Published { entry: Box<VersionEntry> }`,
`crates/vibe-index/src/journal/record.rs:47-53`, и причина вписана в
док-комментарий: «запись каталога ~900 байт против единиц у любого другого
варианта»). Значит `clippy::large_enum_variant` должен вернуться — а
сгенерированный файл руками не правят.

**Первый прогон дал «чисто» — и это был неверный ответ.** Одноразовый крейт
вокруг выхода §3, с тем же `#![allow(non_snake_case)]`, что стоит сегодня в
`crates/vibe-wire/src/lib.rs:52`:

```
cargo clippy --manifest-path …/clippycrate/Cargo.toml --all-targets -- -D warnings
CLIPPY-EXIT=0
```

Ноль линтов. Принять это за ответ было бы ровно тем сертификатом регрессии, о
котором предупреждает
[`SUBAGENT-LAUNCHERS.md`](../SUBAGENT-LAUNCHERS.md)
`#fact-a-fixture-that-cannot-fail-proves-nothing-and-the-detector-may-be-downstream`:
`version_entry` пробы §2 — четырёхполевая заглушка, тогда как настоящая запись
несёт около тридцати полей и вложенные подсекции. Фикстура не упражняла тот
единственный линт, ради которого запускалась. Пойман этот дефект здесь, на
боссовой собственной работе, и в том же прогоне, а не ревью.

**Второй прогон, фикстурой, которая упасть МОЖЕТ.** `version_entry` раздут до
настоящей формы — 21 обязательное поле плюс 7 опциональных, из них пять
вложенных структур (`compatibility`, `provides`, `requires`, `obsoletes`,
`conflicts`), — прочие варианты не тронуты:

```
CLIPPY3-EXIT=101
error: large size difference between variants
help: consider boxing the large fields or introducing indirection in some other
      way to reduce the total size of the enum
   |
69 -     Published(EventPublished),
69 +     Published(Box<EventPublished>),
```

То есть clippy сам называет то преобразование, которое Ф3.1 сделала рукой.
Выходов три, и выбор — боссов:

1. **генератор боксирует крупный вариант** — по аннотации схемы либо по порогу
   размера; честно, но это седьмое преобразование и порог придётся назвать;
2. **`#![allow(clippy::large_enum_variant)]` в сгенерированном модуле** —
   дёшево и глушит сигнал, который в этом дереве уже один раз поймал реальный
   дефект (Ф3.1, вариант ~900 байт); прецедент подавления в дереве
   отсутствует: `grep large_enum_variant` по `crates/` и `xtask/` вне
   `generated/**` даёт ноль — линт не подавляли никогда;
3. **`Event` остаётся рукописным**, а схема его только описывает — тогда G9
   держится процедурой, а не машиной (та же развилка, что в §5).

Прецедент внутрифайлового `allow` в сгенерированном коде существует:
`crates/vibe-wire/src/generated/format_id/mod.rs:13` несёт
`#![allow(clippy::match_same_arms)]`. То есть путь (2) технически проторён —
и именно поэтому его цена должна быть названа вслух, а не выбрана по удобству.

## 4b. Четвёртая проба: G9 нарушен УЖЕ, и JTD его починить не может

Закон G9 (PROP-044 §8
[`##AGENT-GATES`](../../spec/common/PROP-044-change-native-formats.xml#agents)):
«a vocabulary exists in exactly one schema; both wire sides, Rust types, docs
and prose lists are generated from it». Ф4.1 пишет `entry.jtd.json`, где нужен
`package_kind`. Вопрос: где ему жить, чтобы копия была одна.

**Факт, который надо было увидеть до Ф4.1: копий уже ДВЕ.** Словарь
`package_kind` записан дословно в
`schemas/list_report.jtd.json` (блок `definitions.package_kind`) и в
`schemas/registry_sync_report.jtd.json` (там же по структуре) — шесть
одинаковых значений в обоих. То есть G9 нарушен в слое схем **сегодня**, до
всякой Ф4, и `entry.jtd.json` стала бы третьей копией, а не первым нарушением.

**И JTD не даёт способа это починить.** Форма `ref` в JTD (RFC 8927)
разрешается ТОЛЬКО против `definitions` того же документа: ни `$id`, ни
URI-разрешения, ни межфайловых ссылок в языке нет. Проверено прогоном —
схема с одним `{"ref": "package_kind"}` и пустыми `definitions`:

```
GEN4-EXIT=101
thread 'main' panicked at 'no entry found for key',
  /project/crates/core/src/codegen/mod.rs:123:44
```

Два вывода, и второй важнее первого. *(i)* Межфайловая ссылка невозможна —
значит разделение словаря между схемами обязан взять на себя НАШ слой
генерации, ровно по букве PROP-044 §4.2 («everything the language cannot
express … is emitted by our own generator layer»): восьмой механизм —
**включение словаря**, когда схема называет словарь ссылкой вида
`metadata."x-vocabulary-ref": "package_kind"`, а наш слой подставляет его
определение из ОДНОГО документа-словника перед вызовом jtd-codegen.
*(ii)* На висячей ссылке jtd-codegen **паникует**, а не отказывает
диагностикой — то есть у нашей ошибки формы сегодня нет ни сообщения, ни
рецепта, только stack trace. Это прямо противоречит
[`##AGENT-MESSAGES`](../../spec/common/PROP-044-change-native-formats.xml#agents)
(«a gate's message is the only documentation that is reliably read»), и валидацию
ссылок наш слой обязан делать САМ, до спавна бинаря.

*Отвергнутая альтернатива, названная чтобы её не изобретали заново:* сложить
все форматы в один JTD-документ ради общих `definitions` — тогда каталог схем
перестаёт быть каталогом форматов, путь схемы больше не несёт эпоху
(PROP-044 §4.6), и ломается всё, ради чего Ф4.0 учит генератор ходить по
подкаталогам.

## 5. Что из этого следует для нарезки Ф4

**Ф4.1 (схемы) — и восьмой механизм генератора.** По §4b: словарь живёт в
ОДНОМ документе-словнике, а схемы ссылаются на него аннотацией, которую
подставляет наш слой; заодно этот слой валидирует ссылки сам, потому что на
висячей jtd-codegen паникует. Две сегодняшние копии `package_kind` в
`list_report` и `registry_sync_report` уходят в ту же подстановку — иначе Ф4
закрывает G9 на новых форматах и оставляет открытым на старых.

Форма `discriminator`/`mapping` пригодна для
`journal.jtd.json` и `repomd.jtd.json` — доказано прогоном. Каждая схема
обязана нести:
- `metadata."x-vocabulary": "open"|"closed"` на КАЖДОМ `enum` — отсутствие
  аннотации должно быть ошибкой генерации, как А.5 №3 уже требует для
  коллекций (решение обязано быть записано в схеме, а не выведено);
- `metadata."x-empty"` на каждой коллекции (уже в А.5);
- **новое:** аннотацию на опциональном скаляре, говорящую «это не Option, это
  значение с умолчанием» — иначе п.2 вердикта воспроизводится в типе.

**Ф4.2 (генератор).** Преобразований **шесть**, а не четыре:
1. закрытый enum → открытый, **по аннотации схемы, а не по факту закрытости**;
2. camelCase → snake_case (как в А.5);
3. `x-empty` по схеме (как в А.5);
4. `deny_unknown_fields` по реестру (как в А.5);
5. **`Option<Box<скаляр>>` → `скаляр` + `default` + `skip_serializing_if`** по
   аннотации умолчания — иначе ратифицированная двухсоставная ось получает
   третье состояние;
6. **`DateTime<FixedOffset>` → `DateTime<Utc>`** (либо явное решение босса
   двинуть дерево на `FixedOffset`, что дороже и без выгоды);
7. **боксирование крупного варианта объединения** (§4a) — иначе панель краснеет
   на `large_enum_variant`, а править сгенерированный файл руками нельзя.

И **восьмой** вопрос, который преобразованием не решается и потому назван
отдельно: **доменные типы.** Рукописная запись типизирована
`PackageKind`, `Group`, `semver::Version`
(`crates/vibe-index/src/types/entry/mod.rs` — `pub group: Group`,
`pub version: Version`, `pub kind: PackageKind`), рукописное событие — теми же
плюс `NamingConvention` (`journal/record.rs:42-45`). JTD `type: "string"` даёт
`String`, и выразить newtype схема не умеет. Значит «замена рукописных типов
реэкспортом сгенерированных» в сегодняшней формулировке Ф4.2 означает
**понижение доменных типов до строк по всему индексу** — потерю ровно той
типизации, ради которой эти newtype'ы заведены. Либо генератор получает
аннотацию отображения (`x-rust-type: "Group"`), либо формулировка шага меняется.
Развилка боссова и записывается в план, а не оставляется исполнителю.

Плюс правило пропуска: **тегированное объединение постпроцессор не трогает.**
Сегодня Ф0.2-сканер пропускает его СЛУЧАЙНО — `detect_serde_enum`
(`f0-gen-poc.md` §3) ищет `pub enum` строкой сразу за `#[derive(…)]`, а у
объединения между ними стоит `#[serde(tag = "kind")]`, и функция возвращает
`None`. Результат верный, механизм — совпадение. В Ф4.2 пропуск должен быть
НАЗВАННЫМ правилом с тестом, иначе первый же генератор, поставивший атрибут в
другом порядке, начнёт переписывать объединения.

**Радиус п.3 вердикта (newtype-варианты).** Замена рукописного `Event`
реэкспортом сгенерированного меняет форму каждого плеча `match` в проекторе.
Это цена, которую надо назвать в шаге Ф4.2 заранее, а не встретить при сборке.
Альтернатива — оставить `Event` рукописным и описать схемой только те форматы,
где типы реэкспортируются; но тогда `journal.jtd.json` описывает формат,
типы которого живут отдельно, и G9 держится процедурой, а не машиной.
Развилка боссова, и она названа здесь, а не оставлена исполнителю.

## 6. Как воспроизвести

Обе схемы приведены в §2 и §4 целиком — каталог спайка одноразовый, поэтому
несущее приведено здесь (тот же приём, что у Ф0.2 §3). Из корня рабочего
дерева:

```bash
tools/jtd-codegen/jtd-codegen.exe --version          # jtd-codegen 0.4.1
mkdir -p /tmp/probe/out /tmp/probe/out2
# положить §2 в /tmp/probe/journal_probe.jtd.json, §4 в /tmp/probe/meta_probe.jtd.json
tools/jtd-codegen/jtd-codegen.exe /tmp/probe/journal_probe.jtd.json --rust-out /tmp/probe/out  --root-name journal_record
tools/jtd-codegen/jtd-codegen.exe /tmp/probe/meta_probe.jtd.json    --rust-out /tmp/probe/out2 --root-name probe
```

Сверка утверждений §3 — чтением `out/mod.rs`; сверка §4 — тем, что
`out2/mod.rs` не содержит ни одной подстроки `x-`.
Рукописный двойник для сравнения: `crates/vibe-index/src/journal/record.rs:22-34`.

Прогон §4a: одноразовый крейт с `serde` + `chrono`, в `src/lib.rs` —
`#![allow(non_snake_case)]` и следом выход генератора целиком; затем
`cargo clippy --manifest-path <крейт>/Cargo.toml --all-targets -- -D warnings`.
На выходе §2 — exit 0 (фикстура не упражняет линт); на раздутом
`version_entry` (21 обязательное поле + 7 опциональных, пять из них вложенные
структуры) — exit 101 с `large_enum_variant`. Обе схемы получаются из §2
добавлением полей; раздутая приведена не дословно ровно потому, что её
содержание — «настоящий размер», а не конкретные имена.
