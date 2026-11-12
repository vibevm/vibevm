//! The registry's alias table — the short names the REGISTRY OWNER has
//! ISSUED for packages it carries (PROP-008 §2.6 `#short-name`).
//!
//! `vibe install -g qwen` reaches `org.vibevm.qwen/qwen-code` because
//! the registry that carries that package says `qwen` means it. The
//! saying is done in ONE place, `<data-dir>/aliases.toml`, authored by
//! whoever builds and publishes that registry's index — and nowhere
//! else. A package cannot declare its own short name: the owner's
//! ruling of 2026-11-11 is that package-declared aliases would be «an
//! uncontrolled war for short names», because a short spelling is a
//! registry-wide namespace and a package author owns only its own
//! `(group, name)` corner of it.
//!
//! **This table is now the whole of the short-name namespace.** On
//! 2026-11-12 the owner closed the other half of it: «если мы разрешим
//! назначать короткие имена держателям пакетов, то всё сломается,
//! потому что все будут бороться за эти короткие имена. Это то же самое
//! что с доменами в интернете - мы не можем дать людям самим выбирать
//! любое имя.» A package's own `name` is no longer a short name by
//! itself, so `node` resolves only because this table issues `node =
//! "org.vibevm.node/node"`. Two consequences land here. A row whose
//! spelling equals its target's own name is **legal, and the ordinary
//! case** — the rule that refused it existed only to protect a
//! candidacy that no longer exists. And the table's rows are the only
//! thing a resolving client can act on, which is why the writer's gate
//! below is the gate that matters.
//!
//! **The table is an INPUT, not a projection.** It sits beside the
//! journal among the things a catalog is computed FROM — the two
//! inputs of a layout registry are already the tree and the journal
//! ([`crate::layout`]), and this is a third. It is deliberately not a
//! journal fact: a layout registry's journal is committed and replayed
//! by every client that finds the persisted index stale, and
//! [`crate::journal::replay`] refuses a record whose `kind` it does not
//! know. An alias arm would therefore have made every older vibe fail
//! outright on a registry that uses one, where an unread file costs
//! them nothing at all.
//!
//! **What the table produces** is the `alias` member of
//! `by-name/<alias>.json` — the one GET short-name resolution already
//! makes per registry now answers "that spelling is this package"
//! beside "these packages carry that name". The candidate list in the
//! same file stays exactly what it was, so a reader that has never
//! heard of aliases sees an empty candidate set and says the name is
//! not there, which is the truthful answer for a reader that cannot
//! follow the pointer.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use vibe_core::PackageRef;

use crate::error::{Error, Result};
use crate::index::Index;
use crate::types::AliasTarget;

/// The table's file name inside an index data directory — so
/// `<registry>/_index/aliases.toml` for a layout registry.
pub const FILENAME: &str = "aliases.toml";

/// The schema number the parser accepts. A higher one is refused by
/// name rather than half-read: the table decides which package an
/// operator's `vibe install qwen` reaches, and guessing at a document
/// written by a newer tool is the one thing it must not do.
pub const SCHEMA: u32 = 1;

/// The header every rendered table carries.
const HEADER: &str = "\
# vibe registry alias table 1 — the short names this registry issues.
#
# Authored by the REGISTRY OWNER. A package cannot declare its own
# alias: a short spelling is a registry-wide namespace, and package-
# declared aliases would be an uncontrolled race for the good ones.
# A bare `vibe install <name>` resolves through these rows and through
# nothing else — a package is not reachable by its own name unless a
# row here issues it, which `name = \"group/name\"` does.
# Edit with `vibe-index alias <data-dir> add|remove|list`, or, for a
# monorepo/localdir registry, `vibe registry alias <registry> …`.
";

/// The alias table of the index data directory `data_dir`.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILENAME)
}

/// One registry's alias rows, in alias order.
///
/// Empty is the overwhelmingly common state and costs nothing: an
/// absent file reads as an empty table, an empty table writes no file,
/// and a registry that never aliases anything is byte-for-byte the
/// registry it was before this feature existed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AliasTable {
    rows: BTreeMap<String, AliasTarget>,
}

impl AliasTable {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn get(&self, alias: &str) -> Option<&AliasTarget> {
        self.rows.get(alias)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &AliasTarget)> {
        self.rows.iter()
    }

    /// Insert or replace one row. Returns the target it displaced.
    pub fn insert(&mut self, alias: impl Into<String>, target: AliasTarget) -> Option<AliasTarget> {
        self.rows.insert(alias.into(), target)
    }

    pub fn remove(&mut self, alias: &str) -> Option<AliasTarget> {
        self.rows.remove(alias)
    }

    /// Read the table of `data_dir`. An absent file is an empty table —
    /// the state of every registry that has never aliased anything.
    pub fn read(data_dir: &Path) -> Result<Self> {
        let file = path(data_dir);
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AliasTable::default());
            }
            Err(error) => {
                return Err(Error::Io {
                    path: file,
                    message: error.to_string(),
                });
            }
        };
        Self::parse(&text).map_err(|reason| {
            Error::Malformed(format!("alias table `{}`: {reason}", file.display()))
        })
    }

    /// Parse the table's text. The failure is a plain sentence naming
    /// the row, because the operator who wrote it is the one reading.
    pub fn parse(text: &str) -> std::result::Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Doc {
            schema: u32,
            #[serde(default)]
            aliases: BTreeMap<String, String>,
        }
        let doc: Doc = toml::from_str(text).map_err(|e| e.to_string())?;
        if doc.schema != SCHEMA {
            return Err(format!(
                "`schema = {}` is not the {SCHEMA} this vibe reads — the table decides which \
                 package a short name reaches, so a document written by a newer tool is \
                 refused rather than half-understood; upgrade vibe",
                doc.schema
            ));
        }
        let mut rows = BTreeMap::new();
        for (alias, target) in doc.aliases {
            let target = parse_target(&alias, &target)?;
            rows.insert(alias, target);
        }
        Ok(AliasTable { rows })
    }

    /// Render the table — deterministic, one row per line, alias order.
    pub fn render(&self) -> String {
        let mut text = format!("{HEADER}\nschema = {SCHEMA}\n\n[aliases]\n");
        for (alias, target) in &self.rows {
            text.push_str(&format!("{alias} = \"{}/{}\"\n", target.group, target.name));
        }
        text
    }

    /// Write the table to `data_dir`, or remove the file when the table
    /// is empty: a registry that has dropped its last alias must leave
    /// behind the byte-exact tree of one that never had any, or the
    /// listing digest below would carry a ghost for ever.
    pub fn write(&self, data_dir: &Path) -> Result<()> {
        let file = path(data_dir);
        if self.rows.is_empty() {
            return match fs::remove_file(&file) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(Error::Io {
                    path: file,
                    message: error.to_string(),
                }),
            };
        }
        crate::index::atomic_write(&file, self.render().as_bytes())
    }
}

fn parse_target(alias: &str, spelling: &str) -> std::result::Result<AliasTarget, String> {
    let pkgref = PackageRef::parse(spelling)
        .map_err(|e| format!("alias `{alias}` names `{spelling}`, which is not a pkgref: {e}"))?;
    let Some(group) = pkgref.group.clone() else {
        return Err(format!(
            "alias `{alias}` names `{spelling}`, which carries no group — an alias target is \
             always fully qualified `<group>/<name>`, because the whole point of the alias is \
             to supply the group the operator did not type"
        ));
    };
    if !matches!(pkgref.version, vibe_core::VersionSpec::Latest) {
        return Err(format!(
            "alias `{alias}` names `{spelling}`, which pins a version — an alias is a spelling \
             of a PACKAGE, and version selection stays the operator's (`vibe install \
             {alias}@<version>` still works)"
        ));
    }
    Ok(AliasTarget {
        group,
        name: pkgref.name.to_string(),
    })
}

/// Why one alias row cannot stand.
///
/// Every variant is a refusal a WRITER makes before the row reaches the
/// file, and a note a READER makes when it finds the row already there
/// — see [`check`] and [`attach`] for why those two differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasFault {
    /// The alias is not a legal package name. An alias is typed where a
    /// package name is typed, so it obeys the same grammar.
    Grammar { alias: String, reason: String },
    /// This registry's index carries no such package.
    TargetAbsent { alias: String, target: String },
    /// The target's own name is itself an alias here, pointing at a
    /// DIFFERENT package.
    AliasOfAlias { alias: String, target: String },
    /// The alias is a name this registry has buried. A retired name
    /// answers with its tombstone, and re-pointing it would make the
    /// retirement silently reversible by a different hand.
    Buried { alias: String },
}

impl AliasFault {
    pub fn alias(&self) -> &str {
        match self {
            AliasFault::Grammar { alias, .. }
            | AliasFault::TargetAbsent { alias, .. }
            | AliasFault::AliasOfAlias { alias, .. }
            | AliasFault::Buried { alias } => alias,
        }
    }
}

impl fmt::Display for AliasFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AliasFault::Grammar { alias, reason } => write!(
                f,
                "`{alias}` is not a legal package name ({reason}) — an alias is typed exactly \
                 where a package name is typed, so it obeys the same grammar: lowercase \
                 letters, digits and inner single hyphens"
            ),
            AliasFault::TargetAbsent { alias, target } => write!(
                f,
                "`{alias}` names `{target}`, which this registry's index does not carry — an \
                 alias points INTO the registry that offers it, never across to another one, \
                 so publish the package here first or alias something this registry has"
            ),
            AliasFault::AliasOfAlias { alias, target } => write!(
                f,
                "`{alias}` names `{target}`, whose own name is an alias here for a different \
                 package — an alias resolves in exactly one step, so that every spelling has \
                 one readable answer and no table can ever loop; point it at the package \
                 directly"
            ),
            AliasFault::Buried { alias } => write!(
                f,
                "`{alias}` is a name this registry has buried — a retired name answers with \
                 its tombstone and the successor it names, and re-pointing it by another hand \
                 would undo that retirement without a word; choose a fresh spelling"
            ),
        }
    }
}

/// Every fault in `table` against the catalog `index` describes —
/// the WRITER's gate, run before a table reaches disk.
///
/// A writer refuses the whole table rather than dropping the bad row:
/// the operator is standing right there, the file is theirs, and a
/// publish that quietly discarded one of their rows would be found out
/// much later, by someone else, as a short name that simply does not
/// work.
pub fn check(index: &Index, table: &AliasTable) -> Vec<AliasFault> {
    table
        .iter()
        .filter_map(|(alias, target)| fault(index, table, alias, target))
        .collect()
}

fn fault(
    index: &Index,
    table: &AliasTable,
    alias: &str,
    target: &AliasTarget,
) -> Option<AliasFault> {
    if let Err(reason) = bare_name_grammar(alias) {
        return Some(AliasFault::Grammar {
            alias: alias.to_owned(),
            reason,
        });
    }
    if index.tombstones.contains_key(alias) {
        return Some(AliasFault::Buried {
            alias: alias.to_owned(),
        });
    }
    // An alias of an alias is still refused, but the test is now about
    // the TARGET, not about the spelling: a row whose target's name is
    // another row's key is only a second step when the two rows mean
    // different packages. `node = "org.vibevm.node/node"` matches
    // itself here, and `qwen = "org.vibevm.qwen/qwen-code"` beside
    // `qwen-code = "org.vibevm.qwen/qwen-code"` matches a row that
    // means the very same package — neither adds a step, and refusing
    // them would make the issued-name law (§2.6
    // `##ISSUED-NAMES-ONLY`) unusable for exactly the rows it exists
    // to allow.
    if table.get(&target.name).is_some_and(|other| other != target) {
        return Some(AliasFault::AliasOfAlias {
            alias: alias.to_owned(),
            target: spell(target),
        });
    }
    if index.get(&target.group, &target.name).is_none() {
        return Some(AliasFault::TargetAbsent {
            alias: alias.to_owned(),
            target: spell(target),
        });
    }
    None
}

/// `<group>/<name>` — the one spelling of a target used in every
/// message, every row and every rendered table.
pub fn spell(target: &AliasTarget) -> String {
    format!("{}/{}", target.group, target.name)
}

/// The alias grammar, which IS the package-name grammar: a spelling
/// that parses as a bare pkgref and comes back unchanged.
fn bare_name_grammar(alias: &str) -> std::result::Result<(), String> {
    let pkgref = PackageRef::parse(alias).map_err(|e| e.to_string())?;
    let plain = matches!(pkgref.version, vibe_core::VersionSpec::Latest);
    match pkgref.group.is_some() || pkgref.kind.is_some() || !plain || pkgref.name != alias {
        true => Err("it carries a group, a kind or a version".into()),
        false => Ok(()),
    }
}

/// Attach `table` to `index`, keeping the rows that stand — the
/// READER's gate.
///
/// A reader drops a faulty row and answers anyway, where [`check`]
/// refuses. The asymmetry is deliberate and it is the same one the
/// layout projection already makes about a version directory it cannot
/// read: the file belongs to somebody else, a reader that bricked
/// itself over a stranger's typo would make one bad row take down a
/// whole registry, and the row it drops is one that could not have
/// resolved anyway. The dropped rows are returned so a surface that
/// wants to say so can.
pub fn attach(index: &mut Index, table: AliasTable) -> Vec<AliasFault> {
    let faults = check(index, &table);
    let dropped: Vec<&str> = faults.iter().map(AliasFault::alias).collect();
    index.aliases = table
        .iter()
        .filter(|(alias, _)| !dropped.contains(&alias.as_str()))
        .map(|(alias, target)| (alias.clone(), target.clone()))
        .collect();
    for fault in &faults {
        tracing::warn!("alias row dropped: {fault}");
    }
    faults
}

/// Read `data_dir`'s table and attach it to `index`.
pub fn attach_from(index: &mut Index, data_dir: &Path) -> Result<Vec<AliasFault>> {
    Ok(attach(index, AliasTable::read(data_dir)?))
}

/// Fold `records` into a catalog and attach `data_dir`'s alias table —
/// the one way a catalog is built from everything its registry
/// authored.
///
/// The journal alone is no longer the whole input, and every surface
/// that builds a catalog goes through here rather than calling the
/// fold directly, so that "the catalog is what the journal plus the
/// alias table say" holds for the writer, the reader, the server and
/// the rebuild proof identically. A faulty row is dropped with a WARN
/// here, by [`attach`]'s reader rule; the writer's refusal happens
/// where the row is authored, not where a catalog is projected.
pub fn projected(
    records: impl IntoIterator<Item = crate::journal::JournalRecord>,
    data_dir: &Path,
) -> Result<Index> {
    let mut index = crate::journal::project(records)?;
    attach_from(&mut index, data_dir)?;
    Ok(index)
}

#[cfg(test)]
#[path = "aliases/tests.rs"]
mod tests;
