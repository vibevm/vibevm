# F42C-REEXPORT — что на самом деле содержит строка «Ф4.2c — реэкспорт рукописных типов»

**Чем мерил:** чтение дерева боссом — Read/Grep/Glob плюс читающие
shell-команды. Ни одной правки; ни `git`-верба, меняющего состояние. Замер от
корня хостового дерева, на посадке `9c27a1ff` (после закрытия блока Ф4.2b).

**Дата:** 2026-08-17.

**Почему замер понадобился.** В плане Ф4.2c — три строки: «реэкспорт рукописных
типов и сгенерированные ридеры трёх поверхностей без ридера (G11); отдельный
подшаг, потому что меняет граф типов продукта». Приложение А.5a добавляет, что
без доменных типов реэкспорт был бы понижением — и Ф4.2b-7 это уже закрыла. Но
типы состоят не только из полей.

---

## 1. ВЕРДИКТ

**Реэкспорт «как есть» сегодня невозможен, и мешают ему ЧЕТЫРЕ вещи, из
которых план называет ноль.** Каждая — не объём, а конструкция.

| # | что мешает | измерено | класс |
|---|---|---|---|
| A | **Трейт-этаж исчезает.** Рукописные типы выводят `Debug, Clone, PartialEq, Eq` (+`Default` у восьми, +`Copy, PartialOrd, Ord, Hash, ValueEnum` у `PackageKind`); сгенерированные выводят **только `Serialize, Deserialize`** — 74 из 74 | §2 | тот же класс, что О3: реэкспорт понижает, только этажом выше — там поля, здесь трейты |
| B | **`ValueEnum` структурно несовместим с открытым словарём.** `PackageKind` открыт с Ф4.2b-1 (`Unknown(String)`), а `clap::ValueEnum` выводится только по вариантам БЕЗ нагрузки | §3 | открытие словаря и его CLI-роль столкнулись; в generated-дереве это никого не задевало |
| C | **Определений `PackageKind` не два, а ТРИ**, и реэкспорт решает, какое выживает: `vibe-core/src/package_ref/kind.rs`, `vibe-index/src/types/kinds.rs`, сгенерированное | §4 | Б.1 назвала «пять больных словарей»; здесь виден третий носитель, которого перечень не называл |
| D | **Методы и трейт-impl'ы негде держать.** На рукописных типах 15 инherent-методов плюс `Display`×2 и `FromStr`×1; в `generated/**` руками не пишут | §5 | план это предвидел («методы живут в отдельных impl-файлах РЯДОМ»), но не назвал их числа и не назвал `Display`/`FromStr` |

**Что при этом НЕ мешает, хотя могло бы.** Во всех схемах дерева **ноль**
float-типов (`float32`/`float64` — единственное вхождение прозой, внутри
`description`, объясняющей, почему `float64` отвергнут), поэтому `Eq` выводим по
всей поверхности и от формы не зависит. Одной развилкой меньше.

---

## 2. Трейт-этаж: что выводят обе стороны

**Рукописная сторона** (`grep -rn -B1 "^pub struct \|^pub enum " crates/vibe-index/src/types/ | grep derive`):

```
entry/aggregate.rs:61  Debug, Clone, PartialEq, Eq, Serialize, Deserialize
entry/content.rs:17    Debug, Clone, PartialEq, Eq, Serialize, Deserialize
entry/content.rs:35    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
entry/content.rs:57    Debug, Clone, PartialEq, Eq, Serialize, Deserialize
entry/content.rs:69    Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
entry/content.rs:87    Debug, Clone, PartialEq, Eq, Serialize, Deserialize
entry/relations.rs:13  Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
entry/relations.rs:27  Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
entry/relations.rs:39  Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
entry/relations.rs:53  Debug, Clone, PartialEq, Eq, Serialize, Deserialize
entry/relations.rs:58  Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
entry/relations.rs:70  Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize
kinds.rs:17            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum, Serialize, Deserialize
kinds.rs:87            Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize
```

**Сгенерированная сторона** (`grep -rho "^#\[derive([^]]*)\]" crates/vibe-wire/src/generated/ | sort | uniq -c`):

```
     74 #[derive(Serialize, Deserialize)]
      2 #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
```

Две богатые строки — `FormatId` и `ForeignParsers`, и они приходят НЕ от
jtd-codegen, а из TOML-ветки реестра (`format_id/mod.rs`), которую эмитит наш
собственный строковый билдер. То есть **у выхода jtd-codegen трейт-этажа нет
вообще**, и всё, что его несёт, — рукописное.

**Что это стоит при реэкспорте.** Утрата `Debug` лишает тип возможности попасть
в сообщение ассерта или лога; утрата `PartialEq` ломает всякий `assert_eq!` по
записи; утрата `Clone` ломает всякое `.clone()`; утрата `Default` ломает
`::default()` — таких сайтов в `crates/vibe-index/src` **48**
(`grep -rn "::default()" crates/vibe-index/src --include="*.rs" | wc -l`).
Компилятор поймает каждый, поэтому молчаливого перелома здесь нет — но это
означает, что шаг без ответа на (A) просто не соберётся.

*Замечание, которое не надо потерять:* тестовый модуль
`crates/vibe-index/src/types/entry/tests.rs` (281 строка) стоит целиком на
`PartialEq` и `Debug`.

---

## 3. `ValueEnum` против открытого словаря

`crates/vibe-index/src/types/kinds.rs:12` импортирует `clap::ValueEnum`, и обе
`kinds.rs`-перечисления его выводят (`:17`, `:87`). Причина записана в дереве
прозой, а не догадывается: `crates/vibe-index/src/scanner/manifest.rs:14`
говорит «…needing the `Ord` + `clap::ValueEnum` that the …» — то есть трейт
взят осознанно и под конкретную нужду.

**Столкновение.** `clap`'s `ValueEnum` выводится только для перечислений, все
варианты которых БЕЗ нагрузки. После Ф4.2b-1 сгенерированный `PackageKind` несёт
`Unknown(String)` — вариант с нагрузкой. Значит реэкспортированный `PackageKind`
**не может** вывести `ValueEnum`, и сайт `--kind` перестаёт собираться.

Пока словарь был открыт только в `generated/**`, столкновения не существовало:
у сгенерированного типа не было ни одного продуктового потребителя
(находка `f4-transform-radius` §8). Реэкспорт и есть событие, которое их сводит.

**Смежное, той же природы:** `PackageKind::all() -> &'static [PackageKind]`
(`kinds.rs:48`) — у открытого типа «все» перестают быть всеми; сигнатура
компилируется, но её ИМЯ начинает лгать.

---

## 4. Носителей словаря видов ТРИ, и реэкспорт выбирает выжившего

```
crates/vibe-core/src/package_ref/kind.rs   — определение
crates/vibe-index/src/types/kinds.rs:21    — определение
crates/vibe-wire/src/generated/**          — 7 сгенерированных копий
```

Радиус имени (`grep -rln "PackageKind" crates/ --include="*.rs" | grep -v vibe-wire`):
**76 файлов**. По крейтам (`grep -rho "PackageKind" crates/<c>/src | wc -l`):

```
vibe-index 158 · vibe-core 81 · vibe-registry 23 · vibe-cli 22 · vibe-resolver 8
```

Это меняет постановку вопроса. План говорит «`vibe-index` реэкспортирует
сгенерированные типы»; но `vibe-core::PackageKind` — отдельное определение в
крейте, от которого `vibe-index` зависит, и оно не под этим реэкспортом. Пока
`vibe-index` не станет реэкспортировать `vibe-core`'s (или наоборот),
**словарь останется раздвоенным ровно так, как Б.1 и описывает** — просто
переедет шов.

*Что уже сторожит:* `naming_convention_serde_matches_vibe_core_wire`
(`kinds.rs:170`) — межкрейтовый паритет есть ТОЛЬКО у `NamingConvention`;
у `PackageKind` его по-прежнему нет (подтверждает О4).

---

## 5. Что придётся переселить рядом с `generated/**`

Inherent-методы и трейт-impl'ы на рукописных типах
(`grep -rn "^impl \|^    pub fn " crates/vibe-index/src/types/`, без тестов):

| тип | методы |
|---|---|
| `PackageEntry` | `new`, `finalise` |
| `NameEntry` | `new`, `finalise` |
| `VersionEntry` | `minimal`, `sort_key` |
| `FeaturesEntry`, `I18nEntry`, `CompatibilityEntry`, `ProvidesEntry`, `RequiresEntry`, `ObsoletesEntry`, `ConflictsEntry` | `is_empty` ×7 |
| `PackageKind` | `as_str`, `all`, `impl Display`, `impl FromStr` |
| `NamingConvention` | `as_str`, `repo_name`, `impl Display` |
| `Repomd` | (impl-блок `:37`) |
| `RepomdFileEntry` | `directory`, `file` |

Итого **15 inherent-методов плюс `Display` ×2 и `FromStr` ×1**. План предвидел
переезд («методы живут в отдельных impl-файлах РЯДОМ, не в generated»), но не
называл трейт-impl'ы — а они принадлежат ОРФАННОМУ правилу: `impl Display for
PackageKind` законен только в крейте, которому принадлежит тип. После реэкспорта
тип принадлежит `vibe-wire`, значит `Display`/`FromStr` обязаны переехать ТУДА
либо быть заменены на newtype-обёртку у потребителя. Это развилка, а не деталь.

Отдельно: семь предикатов `is_empty` — ровно те, на которых стоит политика
пустого рукописной стороны (Б.2), и они же — то, что Р14 объявила НЕ
переносимым в генератор («генератор начал бы эмитить ПОВЕДЕНИЕ, а не форму»).

---

## 6. G11: три поверхности без ридера — состояние на сегодня

```
crates/vibe-index/src/index/inverted.rs:200  pub fn write_capability(…)
crates/vibe-index/src/index/inverted.rs:215  pub fn write_purl(…)
crates/vibe-index/src/index/primary.rs:43    pub fn write(…) -> (WrittenFile, WrittenFile)
crates/vibe-index/src/index/primary.rs:75    pub fn read(dir) -> Vec<VersionEntry>
crates/vibe-index/src/index/primary.rs:84    pub fn parse(bytes) -> Vec<VersionEntry>
```

Подтверждает О5 поимённо: у `inverted.rs` нет ни одного `read`/`parse` — обе
публикуемые поверхности (`by-cap`, `by-purl`) пишутся и никогда не читаются
нашим кодом. У `primary.rs` `read`/`parse` ЕСТЬ, но `write` возвращает ДВА
`WrittenFile` — несжатый и `.gz`, — а читается только несжатый: третья
поверхность без ридера это `primary.jsonl.gz`.

---

## 7. Ребро зависимости, которое шаг обязан перевернуть

`crates/vibe-index/Cargo.toml`, секция `[dev-dependencies]`, с комментарием,
который называет этот самый шаг:

> «Dev-only on purpose: until F4.2 replaces the hand-written types with
> re-exports of the generated ones, the library itself keeps no runtime edge on
> vibe-wire.»

То есть комментарий — часть периметра: он перестанет быть правдой в ту же
посадку и обязан быть переписан ею же.

---

## 8. Как воспроизвести

```bash
# A — трейт-этаж обеих сторон
grep -rn -B1 "^pub struct \|^pub enum " crates/vibe-index/src/types/ | grep derive
grep -rho "^#\[derive([^]]*)\]" crates/vibe-wire/src/generated/ | sort | uniq -c

# «ноль float'ов» — единственное вхождение прозой внутри description
grep -rn "float32\|float64" schemas/ formats/vocabularies.json

# B — ValueEnum и его причина
grep -rn "ValueEnum" crates/vibe-index/src --include="*.rs"
sed -n '12,20p' crates/vibe-index/src/scanner/manifest.rs

# C — три носителя словаря и радиус имени
grep -rln "PackageKind" crates/ --include="*.rs" | grep -v "^crates/vibe-wire" | wc -l
for c in vibe-index vibe-core vibe-registry vibe-cli vibe-resolver; do \
  printf "%s=%s\n" "$c" "$(grep -rho PackageKind crates/$c/src | wc -l)"; done

# D — что переселяется
grep -rn "^impl \|^    pub fn " crates/vibe-index/src/types/ | grep -v tests.rs

# 48 сайтов ::default()
grep -rn "::default()" crates/vibe-index/src --include="*.rs" | wc -l

# G11
grep -n "pub fn read\|pub fn parse\|pub fn write" \
  crates/vibe-index/src/index/inverted.rs crates/vibe-index/src/index/primary.rs

# ребро
sed -n '44,58p' crates/vibe-index/Cargo.toml
```

Числа главнее текста: если строка здесь и вывод команды разошлись — верна
команда.
