# Ф0.2 — PoC слоя генерации: закрытый словарь → открытый (находка спайка)

**Что это.** Шаг Ф0.2 плана TZ-CHANGE-NATIVE-FORMATS-v0.1 (решение D4, контракт
PROP-044 §4.2/§4.2a, факт `M-OPEN-ENUM-FROM-CLOSED`). Спайк: продуктовое дерево
не тронуто, код жил в одноразовом `_spike-genpoc/` и приведён здесь целиком.
Дата: 2026-08-14.

## 1. ВЕРДИКТ

**Постобработка выхода jtd-codegen 0.4.1 проходима?** ДА С ОГОВОРКАМИ.

Самое страшное из четырёх преобразований wiregen (Приложение А.5) — закрытый
`enum` → открытый `enum { …, Unknown(String) }` с сохранением незнакомой строки —
доказанно проходимо: ~150-строчный постпроцессор строкового сканирования
переписывает реальный выход 0.4.1 по `schemas/list_report.jtd.json`, и все три
гарантии плана (round-trip незнакомого значения, неизменность известных,
побайтовый детерминизм) зелёные на реально сгенерированном коде
(`cargo test` — 8/8). Цена: ~150 строк трансформа + тонкий бинарь + 8 тестов;
ровно одна итерация сборки, и та — баг в утверждении теста (не в трансформе).
**Оговорка, решающая для Ф4:** преобразования №3 (`x-empty`) и №4
(`deny_unknown_fields`) требуют входов, которых в сгенерированном Rust нет —
`metadata."x-empty"` из JTD-схемы и `foreign_parsers` из реестра форматов.
Значит, постпроцессор — НЕ одно-input-текстовое преобразование, а конвейер
`(схема + реестр + сгенерированный Rust) → Rust`. Неизвестное: поведение на
выходах 0.4.1 ДЛЯ ДРУГИХ схем (enum с doc-комментарием между вариантами,
тегированные/discriminator-объединения со структурными вариантами) — проверена
только форма `list_report`; постпроцессор жёстко связан с формой эмиссии 0.4.1,
поему версия генератора должна быть запинена.

## 2. Сверка базовой линии B11 (пять утверждений о выходе 0.4.1)

Вход — `_spike-genpoc/jtd-out/mod.rs` (94 строки), снят боссом с
`tools/jtd-codegen/jtd-codegen.exe` (версия 0.4.1) по схеме
`schemas/list_report.jtd.json`.

| утверждение B11 | подтверждено? | цитата (`jtd-out/mod.rs:строка`) |
|---|---|---|
| 1. `discriminator → #[serde(tag)]` работает | **НЕ ПОДТВЕРЖДЕНО этим выходом** — в `schemas/list_report.jtd.json` нет `discriminator`/`mapping` (только `properties`+`enum`+`ref`), поэтому поведение просто не упражняется; в `jtd-out/mod.rs:1-94` `#[serde(tag` не встречается. Утверждение B11 верно для генератора, но данным файлом не доказуемо. | — (нет discriminator в схеме) |
| 2. enum генерится ЗАКРЫТЫМ, без catch-all | подтверждено | `jtd-out/mod.rs:75-94`: `#[derive(Serialize, Deserialize)]` (75) → `pub enum PackageKind {` (76) → варианты `Feat..Tool` (78-93) → `}` (94); ветки `Unknown`/`#[other]` нет |
| 3. optional → `Option<Box<T>>` + `skip_serializing_if` | подтверждено | `jtd-out/mod.rs:54-55` — `#[serde(skip_serializing_if = "Option::is_none")]` + `pub overridden: Option<Box<bool>>`; то же на `registry` (59-61), `resolved_commit` (64-66), `source_ref` (70-72). Замечание: `boot_snippet` (30-31) = `Option<Box<String>>` БЕЗ skip — он `nullable`, а не optional |
| 4. `deny_unknown_fields` НЕ эмитится | подтверждено | во всём `jtd-out/mod.rs:1-94` строка `deny_unknown_fields` отсутствует (согласуется с заметкой `crates/vibe-wire/src/lib.rs:17-43`) |
| 5. поля camelCase с `#[serde(rename)]` | подтверждено | `content_hash→contentHash` (`:33-34`), `source_url→sourceUrl` (`:45-46`), `resolved_commit→resolvedCommit` (`:64-65`), `source_ref→sourceRef` (`:70-71`), `files_written→filesWritten` (`:36-37`), `boot_snippet→bootSnippet` (`:29-30`) |

## 3. Постпроцессор — код целиком

`_spike-genpoc/src/transform.rs` (277 строк с тестами; ядро ~150). Эмит fmt-чист
(rustfmt не меняет сгенерированный артефакт — проверено).

```rust
//! Text-to-text post-processor for jtd-codegen 0.4.1 output.
//!
//! Rewrites every closed serde `enum` jtd-codegen emits into the OPEN form
//! PROP-044 §4.2a requires:
//!
//! ```ignore
//! pub enum Name { V1, V2, …, Unknown(String) }
//!
//! impl Serialize for Name { /* known variant -> its wire string; Unknown -> the raw string */ }
//! impl<'de> Deserialize<'de> for Name { /* known wire string -> variant; other -> Unknown(s) */ }
//! ```
//!
//! Strategy: a line-oriented state machine. jtd-codegen 0.4.1 emits enums in a
//! rigid, near-canonical shape (verified against the real output for
//! `schemas/list_report.jtd.json` — see `jtd-out/mod.rs`), so a line scanner is
//! enough for the spike. This is deliberately NOT a Rust parser; the harvest
//! finding (`harvest/f0-gen-poc.md` §6) records exactly where that gets fragile.

/// One unit variant collected while scanning an enum body.
#[derive(Clone)]
struct Variant {
    /// Wire string taken from `#[serde(rename = "…")]`; falls back to the
    /// lowercased identifier when the generator omitted a rename.
    wire: String,
    /// The Rust identifier jtd-codegen minted (`Feat`, `Flow`, …).
    ident: String,
}

/// Rewrite every closed serde enum in `src` into an open enum plus the matching
/// hand-written `Serialize` / `Deserialize` impls.
///
/// Structs and any serde-bearing derive that is *not* followed by a
/// `pub enum …` are passed through verbatim. Returns `Err` if the scanner hits
/// a line inside an enum body it does not understand, so a drift in the
/// generator's output is loud rather than silent.
pub fn open_enums(src: &str) -> Result<String, String> {
    let lines: Vec<&str> = src.lines().collect();
    let n = lines.len();
    let mut out = String::with_capacity(src.len() + 512);
    let mut i = 0;
    while i < n {
        if let Some(detection) = detect_serde_enum(&lines, i) {
            let (variants, close_idx) = scan_variants(&lines, detection.body_idx)?;
            out.push_str(&emit_open_enum(
                &detection.name,
                &variants,
                &detection.extras,
            ));
            out.push('\n');
            out.push_str(&emit_impls(&detection.name, &variants));
            i = close_idx + 1;
            continue;
        }
        out.push_str(lines[i]);
        out.push('\n');
        i += 1;
    }
    Ok(out)
}

/// A serde-bearing `#[derive(...)]` immediately preceding a `pub enum Name {`.
struct EnumDetection {
    /// Non-(Serialize/Deserialize) derives to keep on the rewritten enum.
    extras: Vec<String>,
    /// Index of the `pub enum Name {` line.
    body_idx: usize,
    /// The enum's identifier.
    name: String,
}

/// If `lines[at]` is a serde-bearing derive immediately preceding a `pub enum`,
/// describe it; otherwise `None` (the caller copies the line through unchanged).
fn detect_serde_enum(lines: &[&str], at: usize) -> Option<EnumDetection> {
    let inner = lines[at]
        .trim()
        .strip_prefix("#[derive(")?
        .strip_suffix(")]")?;
    let derives: Vec<&str> = inner.split(',').map(str::trim).collect();
    let has_ser = derives.iter().any(|d| *d == "Serialize");
    let has_de = derives.iter().any(|d| *d == "Deserialize");
    if !(has_ser && has_de) {
        return None;
    }
    let mut j = at + 1;
    while j < lines.len() {
        let t = lines[j].trim();
        if t.is_empty() || t.starts_with("//") {
            j += 1;
            continue;
        }
        // Only an enum is rewritten; a struct (or anything else) is left alone.
        if let Some(rest) = t.strip_prefix("pub enum ") {
            let name = rest.split('{').next()?.trim().to_string();
            let extras = derives
                .iter()
                .filter(|d| !matches!(**d, "Serialize" | "Deserialize"))
                .map(|s| s.to_string())
                .collect();
            return Some(EnumDetection {
                extras,
                body_idx: j,
                name,
            });
        }
        return None;
    }
    None
}

/// Scan the enum body starting just after the `pub enum Name {` line, returning
/// the collected unit variants and the index of the closing `}`.
fn scan_variants(lines: &[&str], body_idx: usize) -> Result<(Vec<Variant>, usize), String> {
    let mut variants = Vec::new();
    let mut pending_wire: Option<String> = None;
    let mut k = body_idx + 1;
    while k < lines.len() {
        let t = lines[k].trim();
        if t == "}" {
            return Ok((variants, k));
        }
        if t.is_empty() {
            k += 1;
            continue;
        }
        if let Some(rest) = t.strip_prefix("#[serde(rename = ") {
            // rest looks like `"feat")]`
            let wire = rest
                .trim_end_matches(")]")
                .trim()
                .trim_matches('"')
                .to_string();
            pending_wire = Some(wire);
            k += 1;
            continue;
        }
        if let Some(ident) = t.strip_suffix(',') {
            let ident = ident.trim();
            if is_ident(ident) {
                let wire = pending_wire.take().unwrap_or_else(|| ident.to_lowercase());
                variants.push(Variant {
                    wire,
                    ident: ident.to_string(),
                });
                k += 1;
                continue;
            }
        }
        return Err(format!(
            "open_enums: unsupported line inside enum body: {:?}",
            lines[k]
        ));
    }
    Err("open_enums: enum body has no closing brace".into())
}

/// Emit the rewritten open enum (`Unknown(String)` appended; serde derives
/// dropped unless non-serde extras remain).
fn emit_open_enum(name: &str, variants: &[Variant], extras: &[String]) -> String {
    let mut s = String::new();
    if !extras.is_empty() {
        s.push_str(&format!("#[derive({})]\n", extras.join(", ")));
    }
    s.push_str(&format!("pub enum {} {{\n", name));
    for v in variants {
        s.push_str(&format!("    {},\n", v.ident));
    }
    s.push_str("    Unknown(String),\n");
    s.push_str("}\n");
    s
}

/// Emit hand-written `Serialize` / `Deserialize` impls for the open enum.
fn emit_impls(name: &str, variants: &[Variant]) -> String {
    let mut s = String::new();

    s.push_str(&format!("impl Serialize for {} {{\n", name));
    s.push_str("    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>\n");
    s.push_str("    where\n");
    s.push_str("        S: serde::Serializer,\n");
    s.push_str("    {\n");
    s.push_str("        let wire: &str = match self {\n");
    for v in variants {
        s.push_str(&format!(
            "            {}::{} => \"{}\",\n",
            name, v.ident, v.wire
        ));
    }
    s.push_str(&format!(
        "            {}::Unknown(s) => s.as_str(),\n",
        name
    ));
    s.push_str("        };\n");
    s.push_str("        serializer.serialize_str(wire)\n");
    s.push_str("    }\n");
    s.push_str("}\n\n");

    s.push_str(&format!("impl<'de> Deserialize<'de> for {} {{\n", name));
    s.push_str("    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>\n");
    s.push_str("    where\n");
    s.push_str("        D: serde::Deserializer<'de>,\n");
    s.push_str("    {\n");
    s.push_str("        let s = String::deserialize(deserializer)?;\n");
    s.push_str("        Ok(match s.as_str() {\n");
    for v in variants {
        s.push_str(&format!(
            "            \"{}\" => {}::{},\n",
            v.wire, name, v.ident
        ));
    }
    s.push_str(&format!("            _ => {}::Unknown(s),\n", name));
    s.push_str("        })\n");
    s.push_str("    }\n");
    s.push_str("}\n");

    s
}

/// Cheap identifier check — ASCII alpha/underscore lead, then alphanumeric/`_`.
fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_structs_and_plain_code_untouched() {
        // A serde-bearing derive before a struct is NOT rewritten.
        let src = "#[derive(Serialize, Deserialize)]\npub struct S {\n    \
                   #[serde(rename = \"a_b\")]\n    aB: u32,\n}\n";
        assert_eq!(open_enums(src).unwrap(), src);
    }

    #[test]
    fn opens_a_closed_enum() {
        let src = "#[derive(Serialize, Deserialize)]\npub enum K {\n    \
                   #[serde(rename = \"x\")]\n    X,\n\n    #[serde(rename = \"y\")]\n    Y,\n}\n";
        let out = open_enums(src).unwrap();
        assert!(out.contains("pub enum K {\n"));
        assert!(out.contains("    X,\n"));
        assert!(out.contains("    Y,\n"));
        assert!(out.contains("    Unknown(String),\n"));
        assert!(out.contains("impl Serialize for K {"));
        assert!(out.contains("impl<'de> Deserialize<'de> for K {"));
        assert!(out.contains("\"x\" => K::X"));
        assert!(out.contains("K::Unknown(s) => s.as_str()"));
        assert!(!out.contains("#[derive(Serialize, Deserialize)]\npub enum"));
    }

    #[test]
    fn keeps_non_serde_derives_on_the_enum() {
        let src = "#[derive(Serialize, Deserialize, Clone, Debug)]\npub enum K {\n    \
                   #[serde(rename = \"x\")]\n    X,\n}\n";
        let out = open_enums(src).unwrap();
        assert!(out.contains("#[derive(Clone, Debug)]\npub enum K {"));
        // Serialize/Deserialize are gone from the enum's *derive* …
        assert!(!out.contains("#[derive(Serialize"));
        assert!(!out.contains("#[derive(Deserialize"));
        // … and present only as the hand-written impls.
        assert!(out.contains("impl Serialize for K {"));
        assert!(out.contains("impl<'de> Deserialize<'de> for K {"));
    }

    #[test]
    fn rejects_a_non_unit_variant_loudly() {
        // A struct-variant (`X { f: u32 }`) is not a unit variant -> error,
        // never a silent mis-rewrite.
        let src = "#[derive(Serialize, Deserialize)]\npub enum K {\n    X { f: u32 },\n}\n";
        assert!(open_enums(src).is_err());
    }
}
```

## 4. Результат трансформации — ключевые куски

Зафиксировано в `_spike-genpoc/src/generated_open.rs` (119 строк; `ListReport` и
`ListEntry` переданы насквозь без изменений — их `#[derive(Serialize,
Deserialize)]` сохранён). Только `PackageKind` переписан (`generated_open.rs:75-119`):

```rust
pub enum PackageKind {
    Feat,
    Flow,
    Lang,
    Mcp,
    Stack,
    Tool,
    Unknown(String),
}

impl Serialize for PackageKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire: &str = match self {
            PackageKind::Feat => "feat",
            PackageKind::Flow => "flow",
            PackageKind::Lang => "lang",
            PackageKind::Mcp => "mcp",
            PackageKind::Stack => "stack",
            PackageKind::Tool => "tool",
            PackageKind::Unknown(s) => s.as_str(),
        };
        serializer.serialize_str(wire)
    }
}

impl<'de> Deserialize<'de> for PackageKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "feat" => PackageKind::Feat,
            "flow" => PackageKind::Flow,
            "lang" => PackageKind::Lang,
            "mcp" => PackageKind::Mcp,
            "stack" => PackageKind::Stack,
            "tool" => PackageKind::Tool,
            _ => PackageKind::Unknown(s),
        })
    }
}
```

С enum снят `#[derive(Serialize, Deserialize)]` (иначе derive и ручной impl
столкнулись бы). `Unknown` пишет исходную строку как есть (`serialize_str`), а
при чтении незнакомая строка проваливается в `_ => Unknown(s)`, а не роняет
разбор. Заметка о деталях borrow-check: рука `_ => Unknown(s)` двигает `s`
после `match s.as_str()` — NRL пропускает это (борроу из `as_str()` гаснет до
движения), компиляция чистая.

## 5. Приёмка: тесты и их вывод

Приёмочные тесты — `_spike-genpoc/tests/roundtrip.rs` (первые три — точно три
гарантии плана; четвёртый доказывает воспроизводимость артефакта):

```rust
use genpoc::generated_open::PackageKind;
use genpoc::transform::open_enums;

const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[test] // Acceptance §1: a value absent from the schema survives a round trip.
fn unknown_value_is_preserved_round_trip() {
    let v: PackageKind = serde_json::from_str("\"plugin\"").unwrap();
    assert!(
        matches!(&v, PackageKind::Unknown(s) if s.as_str() == "plugin"),
        "expected Unknown(\"plugin\"), got a named variant"
    );
    let back = serde_json::to_string(&v).unwrap();
    assert_eq!(back, "\"plugin\"");
}

#[test] // Acceptance §2: the six known values map to named variants and back.
fn known_values_round_trip_unchanged() {
    for raw in ["flow", "feat", "lang", "mcp", "stack", "tool"] {
        let v: PackageKind = serde_json::from_str(&format!("\"{}\"", raw)).unwrap();
        assert!(
            !matches!(&v, PackageKind::Unknown(_)),
            "{raw:?} must map to a named variant, not Unknown"
        );
        let back = serde_json::to_string(&v).unwrap();
        assert_eq!(back, format!("\"{}\"", raw));
    }
}

#[test] // Acceptance §3: deterministic bytes; deserialize->serialize idempotent.
fn serialization_is_byte_deterministic() {
    for raw in ["flow", "feat", "plugin", "lang", "totally-new"] {
        let bytes = format!("\"{}\"", raw);
        let v: PackageKind = serde_json::from_str(&bytes).unwrap();
        let a = serde_json::to_vec(&v).unwrap();
        let b = serde_json::to_vec(&v).unwrap();
        assert_eq!(a, b, "non-deterministic serialization for {raw:?}");
        assert_eq!(a, bytes.as_bytes(), "wrong payload for {raw:?}");
        let v2: PackageKind = serde_json::from_slice(&a).unwrap();
        assert_eq!(serde_json::to_vec(&v2).unwrap(), a, "idempotency broken for {raw:?}");
    }
}

#[test] // Acceptance §6: the committed artifact equals open_enums(real input).
fn regeneration_is_stable() {
    let in_path = format!("{}/jtd-out/mod.rs", CRATE_DIR);
    let out_path = format!("{}/src/generated_open.rs", CRATE_DIR);
    let src = std::fs::read_to_string(&in_path).expect("read jtd-out/mod.rs");
    let produced = open_enums(&src).expect("open_enums succeeds on the real input");
    let committed = std::fs::read_to_string(&out_path).expect("read generated_open.rs");
    assert_eq!(produced, committed,
        "src/generated_open.rs must be exactly open_enums(jtd-out/mod.rs)");
}
```

Дословный вывод `cargo test --manifest-path _spike-genpoc/Cargo.toml` (код
выхода 0):

```
    Compiling genpoc v0.1.0 (C:\Users\olegc\git\v\vibevm\.wt\F0-GENPOC\_spike-genpoc)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.75s
     Running unittests src\lib.rs (...)
running 4 tests
test transform::tests::leaves_structs_and_plain_code_untouched ... ok
test transform::tests::rejects_a_non_unit_variant_loudly ... ok
test transform::tests::opens_a_closed_enum ... ok
test transform::tests::keeps_non_serde_derives_on_the_enum ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\roundtrip.rs (...)

running 4 tests
test known_values_round_trip_unchanged ... ok
test serialization_is_byte_deterministic ... ok
test unknown_value_is_preserved_round_trip ... ok
test regeneration_is_stable ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Где доказан каждый пункт приёмки плана:

- **§1 (`"plugin"` → `Unknown("plugin")` → `"plugin"`):** тест
  `unknown_value_is_preserved_round_trip` — `ok`.
- **§2 (известные без изменений):** тест `known_values_round_trip_unchanged`
  (шесть значений, включая проверку, что они НЕ попадают в `Unknown`) — `ok`.
- **§3 (побайтовый детерминизм):** тест `serialization_is_byte_deterministic`
  (две сериализации равны + идемпотентный цикл) — `ok`.
- **воспроизводимость артефакта (приёмка пакета §6):** тест
  `regeneration_is_stable` + `postproc --check` (см. §8) — `ok` / `MATCH`.

## 6. Где этот подход хрупок

Подход — строковый сканер, а не парсер Rust. Он держится на жёсткой форме эмиссии
jtd-codegen 0.4.1. Конкретно:

- **Форма enum'а.** Рассчитано на `#[derive(Serialize, Deserialize)]` (с
  произвольным довеском `Clone`/`Debug`) → `pub enum Name {` → варианты вида
  `#[serde(rename = "x")] Ident,`. Смена версии генератора (другой порядок
  атрибутов, `#[serde(rename_all=…)]` вместо по-вариантного rename, иное
  форматирование) ломает распознавание. Заголовок файла гласит `// Code
  generated by jtd-codegen for Rust v0.2.1` при бинарнике 0.4.1 — сам по себе
  сигнал, что строка версии ненадёжна; пинить надо бинарник, а не верить ей.
- **Тегированные/discriminator-объединения (структурные варианты).**
  `scan_variants` понимает ТОЛЬКО единичные варианты; строка вроде
  `X { f: u32 }` даёт ГРОМКУЮ ошибку (тест `rejects_a_non_unit_variant_loudly`),
  не молчаную порчу. Но это значит, что D2-объединение `RepomdFileEntry`
  (тегированное, со структурными полями) НЕ покрывается этим преобразованием —
  для него нужен отдельный путь кодогенерации. Это согласовано со сферой §4.2a
  (она про словари-значения, не про объединения).
- **doc-комментарии между вариантами или над enum'ом.** Не проверено.
  `#[serde(rename)]`-комментарий-над-вариантом — OK, но `/// …` между вариантами
  текущий сканер не различает (он ищет либо rename, либо `Ident,`); строку
  doc-комментария он воспримет как «неподдержанную строку» и ГРОМКО откажет.
  В реальном выходе 0.4.1 для `package_kind` doc-комментариев нет.
- **Вложенные enum'ы** (определённые внутри тела структуры/модуля с отступом) —
  не обработаны (эмиссия предполагается на верхнем уровне; jtd-codegen с
  `--root-name` так и кладёт). Предположение, не проверено на других схемах.
- **wire-формат = JSON.** Ручной impl сериализует ВСЕ варианты (включая
  единичные) как строку (`serialize_str`). Для JSON-провода (провод проекта,
  PROP-044) это верно и совпадает со схемой. Но для любого бинарного формата
  (bincode и т.п.), где единичный вариант прежде шёл индексом, представление
  меняется: `Unknown(String)` — длина+строка, известные — единичный вариант.
  Это изменение поведения для не-JSON форматов; в рамках провода vibevm
  безопасно, но должно быть оговорено.
- **Коллизия имён.** Если в схеме встретится значение `"unknown"`, сгенерированный
  идентификатор и ветка `Unknown(String)` столкнутся. Крайний случай, не проверен.
- **Что проверено vs предположено.** ПРОВЕРЕНО: этот конкретный выход 0.4.1
  (`list_report`), fmt-чистота эмита, все три гарантии round-trip, детерминизм.
  ПРЕДПОЛОЖЕНО (не упражнялось): та же форма на других схемах, наличие
  doc-комментариев, вложенные enum'ы.

## 7. Что это значит для фазы Ф4

Приложение А.5 плана — четыре преобразования выхода jtd-codegen, в порядке. Где
лежат они на том же механизме, а где требуют иного:

1. **Закрытый enum → открытый (этот спайк).** Чистая постобработка
   сгенерированного Rust, без второго входа. ДОКАЗАНО. Механизм тот же, что в
   `transform.rs`.
2. **camelCase → snake_case (А.5 №2).** Тот же чисто-текстовый механизм, ПРОЩЕ
   (локальное правило на каждую строку-поле: переименовать идентификатор в
   snake_case и поправить/снять `#[serde(rename)]`). Снимает
   `#![allow(non_snake_case)]`. Низкий риск; отдельно не проверялся.
3. **Политика `x-empty` (А.5 №3).** НЕ чистый текст: правило живёт в
   `metadata."x-empty"` JTD-схемы, а в сгенерированном Rust его нет. Постпроцессор
   обязан читать схему и сшивать её с полями по путям. Значит, «Rust-текст →
   Rust-текст» НЕ достаточно; `vibe-wire-gen` — конвейер
   `(схема + реестр + сгенерированный Rust)`. Это реальный довод в сторону
   собственного эмиттера по JTD (он владеет схемой нативно).
4. **`deny_unknown_fields` по роли (А.5 №4).** Вход — реестр форматов
   (`foreign_parsers = "none"` → deny); само преобразование тривиально
   (вставить атрибут над нужными структурами), но вход — реестр, не выводимый
   из Rust.

**Ветка Ф4.2:** постпроцессор поверх jtd-codegen (D4) ЖИЗНЕСПОСОБЕН и дешевле
собственного эмиттера по JTD — НО при условии, что он берёт схему и реестр как
дополнительные входы (а не только сгенерированный Rust), и что jtd-codegen 0.4.1
запинен (постпроцессор связан с формой его эмиссии). Собственный эмиттер меняет
эту связанность на больший объём кода (но PROP-044 §4.2 выбрал JTD за бедность —
эмиттер ограничен по сложности). Рекомендация: **D4 (постпроцессор)** с входами
`(schema, registry, generated-Rust)`; пин 0.4.1. Самое дорогое из четырёх —
№1 — уже доказано; №2/№4 — дёшевы; №3 — умеренный (сшивка полей схемы с
полями типа). Оценка объёма Ф4.2: несколько сотен строк в `xtask/src/wiregen/`,
не крупная стройка.

## 8. Как воспроизвести

Одноразовый крейт `_spike-genpoc/` (НЕ входит в workspace корневого `Cargo.toml`;
несёт пустую таблицу `[workspace]`, иначе cargo отказывается собирать). Из корня
рабочего дерева:

```bash
# 1. регенерировать зафиксированный артефакт из входа
cargo run --manifest-path _spike-genpoc/Cargo.toml --bin postproc
#   -> postproc: .../jtd-out/mod.rs -> .../src/generated_open.rs (3346 bytes)

# 2. доказать воспроизводимость (exit 0 = MATCH; артефакт не написан руками)
cargo run --manifest-path _spike-genpoc/Cargo.toml --bin postproc -- \
    --check _spike-genpoc/jtd-out/mod.rs _spike-genpoc/src/generated_open.rs
#   -> postproc --check: MATCH (3346 bytes)

# 3. зелёные тесты (8/8) + fmt-чистота
cargo test --manifest-path _spike-genpoc/Cargo.toml
cargo fmt --manifest-path _spike-genpoc/Cargo.toml -- --check
```

Каталог `_spike-genpoc/` одноразовый и в основное дерево не переезжает — поэтому
существенный код (постпроцессор целиком + открытый enum с ручными impl) приведён
в §3 и §4 этой находки.
