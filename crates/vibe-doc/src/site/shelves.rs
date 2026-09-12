//! Shelves, languages and relations — computed from the edges, stored
//! nowhere (PROP-057 `##REL-OFFICIAL-IS-CONVERGENCE`,
//! `##REL-NO-OFFICIAL-FLAG`, `##REL-REVERSE-QUERIES-SITE-SIDE`,
//! `##LOC-OFFICIAL-TRANSLATION`, `##LOC-NO-TRANSLATIONS-TABLE`).
//!
//! Three questions a package page answers about other packages: who
//! documents me, who adapted that documentation into another language,
//! and who depends on me. None of them can be answered by reading one
//! package — every one is the OTHER end of an edge somebody else
//! declared — and none of them is stored anywhere. They are folded out
//! of the catalog in memory at every build, which is the whole of
//! `##REL-REVERSE-QUERIES-SITE-SIDE`.
//!
//! ## Officiality is a convergence, and that is why it cannot be a flag
//!
//! Documentation is **official** when both ends agree: the subject named
//! the package in `[documentation]`, and the package named the subject
//! in `[[documents]]`. One edge alone is **community** — which is not a
//! lesser state but a different one, and the site shows it beside the
//! official shelf rather than hiding it. A translation is official when
//! it declared `translates` on its source AND was published by the same
//! group under the naming convention.
//!
//! A stored `official = true` would be a second source of truth, and the
//! design error `##REL-NO-OFFICIAL-FLAG` names by hand: three
//! contributors could then fight over the word, and a mirror could ship
//! the answer without the edges that justify it.
//!
//! ## The default convention, and what replaces it
//!
//! A subject that declares no `[documentation]` at all gets one for
//! free: `<name>-docs` in its own group is its primary documentation. A
//! subject that DOES declare `[documentation]` replaces the convention
//! entirely — so a subject naming one package makes every other one
//! community, `<name>-docs` included.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#REL-OFFICIAL-IS-CONVERGENCE");

use std::collections::BTreeMap;

use vibe_wire::generated::shared::VersionEntry;

/// How strongly a relation is confirmed from above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    /// The one a reader is sent to first.
    Primary,
    /// Confirmed by the other end.
    Official,
    /// Declared from one side only. A state, never a lesser one.
    Community,
}

impl Rank {
    /// The mark `##DISC-STAR-MEANS-CONFIRMED` puts on a confirmed
    /// relation, and nothing at all on one that is not.
    pub fn star(self) -> &'static str {
        match self {
            Rank::Primary | Rank::Official => "★",
            Rank::Community => "",
        }
    }

    /// The word beside the mark. Three signals have to agree — the mark,
    /// the word and the order — or a reader learns to trust none.
    pub fn word(self) -> &'static str {
        match self {
            Rank::Primary => "primary",
            Rank::Official => "official",
            Rank::Community => "community",
        }
    }
}

/// What one package states that makes an edge.
///
/// Built from a catalog record or from a manifest on disk, because the
/// site has both kinds of source and they carry the same statements: the
/// registry answers with an index entry, the host's checkout answers
/// with the file the entry would have been made from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub group: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub title: String,
    /// The language of a `doc` package — `[i18n].canonical`, and there
    /// is no separate `lang` field (`##LOC-LANGUAGE-FIELD`).
    pub lang: String,
    /// `[[documents]]` — the subjects this documentation documents.
    pub documents: Vec<String>,
    /// `[documentation]` — `(primary, official)` as the SUBJECT names
    /// them, or `None` when it names nothing and the convention applies.
    pub documentation: Option<(Option<String>, Vec<String>)>,
    /// `[translates]` — the documentation this package adapts.
    pub translates: Option<String>,
    /// The coordinates this version requires.
    pub requires: Vec<String>,
}

impl Facts {
    /// `<group>/<name>`.
    pub fn coordinate(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }

    /// Is this a documentation package?
    pub fn is_doc(&self) -> bool {
        self.kind == "doc"
    }
}

/// The default language of a package that declares none (PROP-003 §2.7).
const DEFAULT_LANGUAGE: &str = "en";

/// Read one catalog record.
pub fn from_entry(entry: &VersionEntry) -> Facts {
    Facts {
        group: entry.group.to_string(),
        name: entry.name.clone(),
        version: entry.version.to_string(),
        kind: entry.kind.as_str().to_string(),
        title: entry.title.clone().unwrap_or_else(|| entry.name.clone()),
        lang: entry
            .i18n
            .as_ref()
            .and_then(|i| i.default.clone())
            .unwrap_or_else(|| DEFAULT_LANGUAGE.to_string()),
        documents: entry.documents.iter().map(|d| d.package.clone()).collect(),
        documentation: entry
            .documentation
            .as_ref()
            .map(|d| (d.primary.clone(), d.official.clone())),
        translates: entry.translates.as_ref().map(|t| t.package.clone()),
        requires: entry
            .requires
            .as_ref()
            .map(|r| r.packages.iter().map(|p| coordinate_of(p)).collect())
            .unwrap_or_default(),
    }
}

/// Read the same statements off a manifest on disk.
///
/// The host's checkout is in no index — its root is a `[project]` and
/// its in-tree packages are not published — so the edges it declares
/// have to be read where they are written. Two sources, one vocabulary:
/// this reads exactly the members [`from_entry`] reads, and a `Facts`
/// does not remember which of the two it came from.
///
/// Read as TOML data, like every other reading of a foreign manifest in
/// this library: a strict parse would drop a package's edges over a
/// field this function never looks at.
pub fn from_manifest(package_dir: &std::path::Path) -> Option<Facts> {
    let text = std::fs::read_to_string(package_dir.join("vibe.toml")).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let read = |table: &str, key: &str| {
        value
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let card = |key: &str| read("package", key).or_else(|| read("project", key));
    let name = card("name")?;
    Some(Facts {
        group: card("group")?,
        version: card("version")?,
        kind: card("kind").unwrap_or_else(|| "pack".to_string()),
        title: card("title").unwrap_or_else(|| name.clone()),
        name,
        lang: read("i18n", "canonical").unwrap_or_else(|| DEFAULT_LANGUAGE.to_string()),
        documents: value
            .get("documents")
            .and_then(toml::Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.get("package")?.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        documentation: value.get("documentation").map(|table| {
            (
                table
                    .get("primary")
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned),
                table
                    .get("official")
                    .and_then(toml::Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(|one| one.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        }),
        translates: value
            .get("translates")
            .and_then(|t| t.get("package"))
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        // `[requires.packages]` is keyed by the coordinate, with the
        // constraint as the value — the reverse of an index record's
        // flat list, and the reason this is read here rather than
        // reshaped somewhere both could share.
        requires: value
            .get("requires")
            .and_then(|r| r.get("packages"))
            .and_then(toml::Value::as_table)
            .map(|table| table.keys().cloned().collect())
            .unwrap_or_default(),
    })
}

/// The coordinate inside a requirement, which carries a constraint
/// beside it. Everything up to the first `@`, space or `:` — the three
/// spellings a requirement is written in — and the whole string when it
/// carries none.
fn coordinate_of(requirement: &str) -> String {
    requirement
        .split([' ', '@'])
        .next()
        .unwrap_or(requirement)
        .trim()
        .to_string()
}

/// One row on a shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub coordinate: String,
    pub version: String,
    pub title: String,
    /// The group, always printed: the publisher is visible by law
    /// (`##DISC-PUBLISHER-VISIBLE`), because a title alone can be made
    /// to look like anybody's.
    pub publisher: String,
    pub lang: String,
    pub rank: Rank,
}

/// The reverse edges of a whole catalog, folded once.
#[derive(Debug, Clone, Default)]
pub struct Shelves {
    documentation: BTreeMap<String, Vec<Row>>,
    translations: BTreeMap<String, Vec<Row>>,
    dependants: BTreeMap<String, Vec<Row>>,
}

impl Shelves {
    /// The documentation of one coordinate, ranked and then ordered:
    /// primary, official, community, and by coordinate inside each.
    pub fn documentation_of(&self, coordinate: &str) -> &[Row] {
        self.documentation
            .get(coordinate)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// The adaptations of one documentation coordinate.
    pub fn translations_of(&self, coordinate: &str) -> &[Row] {
        self.translations
            .get(coordinate)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// What depends on one coordinate.
    pub fn dependants_of(&self, coordinate: &str) -> &[Row] {
        self.dependants
            .get(coordinate)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Every language one documentation is readable in: its own, and the
    /// language of every adaptation of it. Computed from the `translates`
    /// edges because the source stores no list of its translations
    /// (`##LOC-NO-TRANSLATIONS-TABLE`).
    pub fn languages_of(&self, coordinate: &str, own: &str) -> Vec<String> {
        let mut out = vec![own.to_string()];
        for row in self.translations_of(coordinate) {
            if !out.contains(&row.lang) {
                out.push(row.lang.clone());
            }
        }
        out
    }
}

/// Fold a catalog into its reverse edges.
pub fn fold(facts: &[Facts]) -> Shelves {
    let by_coordinate: BTreeMap<String, &Facts> =
        facts.iter().map(|f| (f.coordinate(), f)).collect();
    let mut shelves = Shelves::default();

    for one in facts {
        for subject in &one.documents {
            let rank = documentation_rank(one, subject, by_coordinate.get(subject).copied());
            shelves
                .documentation
                .entry(subject.clone())
                .or_default()
                .push(row(one, rank));
        }
        if let Some(source) = &one.translates {
            let rank = translation_rank(one, source);
            shelves
                .translations
                .entry(source.clone())
                .or_default()
                .push(row(one, rank));
        }
        for required in &one.requires {
            shelves
                .dependants
                .entry(required.clone())
                .or_default()
                .push(row(one, Rank::Community));
        }
    }

    for rows in shelves
        .documentation
        .values_mut()
        .chain(shelves.translations.values_mut())
        .chain(shelves.dependants.values_mut())
    {
        // Ranked first, then by coordinate: the order is one of the
        // three signals that have to agree with the mark and the word.
        rows.sort_by(|a, b| {
            a.rank
                .cmp(&b.rank)
                .then_with(|| a.coordinate.cmp(&b.coordinate))
                .then_with(|| a.version.cmp(&b.version))
        });
        rows.dedup();
    }
    shelves
}

fn row(facts: &Facts, rank: Rank) -> Row {
    Row {
        coordinate: facts.coordinate(),
        version: facts.version.clone(),
        title: facts.title.clone(),
        publisher: facts.group.clone(),
        lang: facts.lang.clone(),
        rank,
    }
}

/// Where one documentation package stands for one subject.
///
/// The subject decides, which is what makes the answer stable: three
/// contributors cannot argue over a word the subject assigns. A subject
/// the catalog does not carry cannot have decided anything, so the
/// convention is all that is left — and the convention is exactly what a
/// subject which said nothing would have got.
fn documentation_rank(doc: &Facts, subject: &str, known: Option<&Facts>) -> Rank {
    if let Some(declared) = known.and_then(|s| s.documentation.as_ref()) {
        let (primary, official) = declared;
        let coordinate = doc.coordinate();
        if primary.as_deref() == Some(coordinate.as_str()) {
            return Rank::Primary;
        }
        if official.iter().any(|o| o == &coordinate) {
            return Rank::Official;
        }
        // A declared `[documentation]` replaces the convention
        // ENTIRELY: naming one package makes `<name>-docs` community
        // like anybody else's.
        return Rank::Community;
    }
    match convention(subject, doc) {
        true => Rank::Primary,
        false => Rank::Community,
    }
}

/// The default convention: `<name>-docs` in the subject's own group.
fn convention(subject: &str, doc: &Facts) -> bool {
    let Some((group, name)) = subject.rsplit_once('/') else {
        return false;
    };
    doc.group == group && doc.name == format!("{name}-docs")
}

/// Where one adaptation stands for its source.
///
/// The mark here means «named by the author of this documentation», not
/// «approved by the subject» — a different claim from the one on the
/// documentation shelf, and the interface says so in words
/// (`##LOC-OFFICIAL-TRANSLATION`).
fn translation_rank(adaptation: &Facts, source: &str) -> Rank {
    let Some((group, name)) = source.rsplit_once('/') else {
        return Rank::Community;
    };
    let expected = format!("{name}-{}", adaptation.lang);
    match adaptation.group == group && adaptation.name == expected {
        true => Rank::Official,
        false => Rank::Community,
    }
}

#[cfg(test)]
mod tests;
