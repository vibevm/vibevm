//! `formats/EPOCHS.toml` — the epochs flags, read loudly.
//!
//! Two booleans decide the whole change-native regime (PROP-044 §4.7
//! `##M-BREAK-WINDOW`, §7 `##STABILITY-IS-A-FLAG` and
//! `##THE-PUBLIC-SWITCH`): `public` — has the first public presentation
//! happened, a fact only the owner can declare because it is technically
//! underivable from any event — and `break_window_open` — are changes
//! under `schemas/**` and `formats/**` permitted at all. The next phase
//! step, `cargo xtask wire-diff`, reads its entire behaviour out of
//! them: pre-publication it reports, a closed window forbids, an open
//! public window demands a break note. The reader cannot land before the
//! thing it reads — hence this module first.
//!
//! It is the `load_format_registry` genre (`codegen/format_id.rs`):
//! TOML in the tree → a typed value → a loud refusal on every crooked
//! input. A missing file is a broken checkout, not a pre-publication
//! default to guess — the doctrine `Vocabularies::load` states for the
//! vocabulary home (`codegen/vocabulary.rs`). An unknown key is a typo
//! caught at the edit — the same argument that makes the hand-written
//! manifest strict, PROP-044 §6.2 `##FMT-MANIFEST`. A missing or
//! non-boolean key is a flag with no default to guess: a default the
//! author never saw is exactly the accident this file exists to make
//! impossible. No reading of these flags may go around this loader — a
//! second parser could disagree with the first, and the disagreement
//! would surface as a gate forbidding what a report just promised.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

/// The epochs home, relative to the repo root. `formats/` is the house
/// of data about formats (`REGISTRY.toml`, `vocabularies.json`,
/// `hash_recipes/`, `breaks/`); the two flags govern every format at
/// once, which is why they sit one level above the registry rather than
/// inside it as records.
pub(crate) fn epochs_path(root: &Path) -> PathBuf {
    root.join("formats").join("EPOCHS.toml")
}

/// The epochs flags, typed. Both fields are owner decisions recorded in
/// the file — the loader fills them, nothing computes them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Epochs {
    /// The public switch (PROP-044 §7 `##THE-PUBLIC-SWITCH`): `false`
    /// until the owner declares the first public presentation. While
    /// `false`, breaking is free and unmigrated — no codemods, no
    /// sunset calendars, no mandatory break notes.
    pub(crate) public: bool,
    /// The break window (PROP-044 §4.7 `##M-BREAK-WINDOW`): closed
    /// means changes under `schemas/**` and `formats/**` are rejected;
    /// open means they are allowed — and at `public = true` each
    /// wire-visible one requires a break note under `formats/breaks/`.
    pub(crate) break_window_open: bool,
}

/// The exactly two legal keys, in file order. The unknown-key refusal
/// names both, so a typo dies naming what to write instead.
const LEGAL_KEYS: [&str; 2] = ["public", "break_window_open"];

impl Epochs {
    /// Read and validate `formats/EPOCHS.toml`. Every crooked input —
    /// missing file, unknown key, missing key, non-boolean value —
    /// refuses here, carrying the rule, one phrase of why, and the fix;
    /// the file is committed state a human edits, and this is the only
    /// place positioned to answer a typo at the edit.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let path = epochs_path(root);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => bail!(
                "formats/EPOCHS.toml is missing (looked at {}). The epochs \
                 file is committed state, so a missing file is a broken \
                 checkout, not a pre-publication default to guess — \
                 `public = false` is a ratified owner decision, and an \
                 absent file would let every reader invent its own.\n\
                 Fix: restore formats/EPOCHS.toml from version control, \
                 then re-run the command.",
                path.display()
            ),
            Err(err) => {
                return Err(anyhow::Error::from(err))
                    .with_context(|| format!("reading {}", path.display()));
            }
        };
        let parsed: toml::Value =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        // A TOML document always parses to a table of top-level keys; the
        // require_bool calls below cannot proceed without one.
        let table = parsed.as_table().with_context(|| {
            format!(
                "`{}` did not parse to a table of keys — the epochs file is \
                 two top-level booleans, nothing else",
                path.display()
            )
        })?;

        // The typo check runs BEFORE the key checks: a file whose author
        // wrote `publik` in place of `public` deserves the sharper
        // diagnosis — the stranger is named — not a "missing `public`"
        // that sends them hunting for a deletion that never happened.
        for key in table.keys() {
            if !LEGAL_KEYS.contains(&key.as_str()) {
                bail!(
                    "formats/EPOCHS.toml: unknown key `{key}` — the epochs \
                     file carries exactly two keys, `public` and \
                     `break_window_open`, and a file a human edits by hand \
                     must catch the typo at the edit (PROP-044 §6.2 \
                     `##FMT-MANIFEST`), not carry it as a dead flag.\n\
                     Fix: rename `{key}` to the key you meant — `public` or \
                     `break_window_open` — then re-run the command."
                );
            }
        }

        let public = require_bool(table, "public")?;
        let break_window_open = require_bool(table, "break_window_open")?;
        Ok(Self {
            public,
            break_window_open,
        })
    }
}

/// One boolean flag out of the epochs table — present, and boolean.
/// A missing key and a wrongly-typed one refuse separately: absence is
/// a flag somebody deleted (no default exists to guess), a string where
/// a boolean belongs is a quoting slip (`public = "false"` reads as a
/// decision to a human and as data to the machine).
fn require_bool(table: &toml::value::Table, key: &str) -> Result<bool> {
    match table.get(key) {
        None => bail!(
            "formats/EPOCHS.toml: `{key}` is missing — both flags are owner \
             decisions with no default to guess: a missing `public` must not \
             read as «not public yet» any more than a missing \
             `break_window_open` may read as «closed».\n\
             Fix: re-add the line (today's ratified state: `public = false`, \
             `break_window_open = true`), then re-run the command."
        ),
        Some(value) => value.as_bool().ok_or_else(|| {
            anyhow!(
                "formats/EPOCHS.toml: `{key}` must be a boolean — found {}. \
                 The flags are read by gate machinery that refuses to guess, \
                 and a quoted `\"false\"` is a string, not a decision.\n\
                 Fix: write `{key} = false` or `{key} = true` (no quotes), \
                 then re-run the command.",
                toml_kind(value)
            )
        }),
    }
}

/// The TOML kind of a value, for refusal texts — naming what was found
/// beats making the reader reconstruct it from a parse error.
fn toml_kind(value: &toml::Value) -> &'static str {
    match value {
        toml::Value::String(_) => "a string",
        toml::Value::Integer(_) => "an integer",
        toml::Value::Float(_) => "a float",
        toml::Value::Boolean(_) => "a boolean",
        toml::Value::Datetime(_) => "a datetime",
        toml::Value::Array(_) => "an array",
        toml::Value::Table(_) => "a table",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::{Context, Result};
    use tempfile::TempDir;

    use super::Epochs;

    /// The ratified file, verbatim — the fixture every refusal below
    /// breaks exactly one thing in.
    const GOOD: &str = "public = false\nbreak_window_open = true\n";

    /// A scratch repo root whose `formats/EPOCHS.toml` is `text` — or
    /// absent entirely when `text` is `None`.
    fn scratch_root(text: Option<&str>) -> Result<TempDir> {
        let root = tempfile::tempdir().context("creating the scratch root")?;
        let formats = root.path().join("formats");
        std::fs::create_dir_all(&formats).context("creating the scratch formats dir")?;
        if let Some(text) = text {
            std::fs::write(formats.join("EPOCHS.toml"), text)
                .context("writing the scratch epochs file")?;
        }
        Ok(root)
    }

    /// §3.3a — a correct file loads both flags as written. The flipped
    /// copy proves the loader reads the file rather than echoing the
    /// ratified state it was built beside.
    #[test]
    fn a_correct_file_loads_both_flags() -> Result<()> {
        let root = scratch_root(Some(GOOD))?;
        let epochs = Epochs::load(root.path())?;
        assert!(!epochs.public, "GOOD carries public = false");
        assert!(
            epochs.break_window_open,
            "GOOD carries break_window_open = true"
        );

        let flipped = scratch_root(Some("public = true\nbreak_window_open = false\n"))?;
        let epochs = Epochs::load(flipped.path())?;
        assert!(epochs.public, "the loader reads the file, not the calendar");
        assert!(!epochs.break_window_open);
        Ok(())
    }

    /// §3.3b — a missing file is a broken checkout, refused with the
    /// doctrine and the restore recipe, never a guessed default.
    #[test]
    fn a_missing_file_is_a_broken_checkout_not_a_default() -> Result<()> {
        let root = scratch_root(None)?;
        let err = Epochs::load(root.path()).expect_err("a missing epochs file must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("broken checkout"),
            "the refusal states the doctrine: {msg}"
        );
        assert!(
            msg.contains("restore formats/EPOCHS.toml"),
            "the refusal carries the fix: {msg}"
        );
        assert!(
            msg.contains("EPOCHS.toml"),
            "the refusal names the file it probed: {msg}"
        );
        Ok(())
    }

    /// §3.3c — an unknown key is refused naming the stranger and BOTH
    /// legal keys, so a typo dies at the edit.
    #[test]
    fn an_unknown_key_is_refused_naming_it_and_both_legal_keys() -> Result<()> {
        let root = scratch_root(Some("publik = false\nbreak_window_open = true\n"))?;
        let err = Epochs::load(root.path()).expect_err("an unknown key must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("`publik`"),
            "the refusal names the unknown key: {msg}"
        );
        assert!(
            msg.contains("`public`") && msg.contains("`break_window_open`"),
            "the refusal names both legal keys: {msg}"
        );
        assert!(
            msg.contains("rename `publik`"),
            "the refusal carries the fix: {msg}"
        );
        Ok(())
    }

    /// §3.3d — a non-boolean value is refused naming the key, the kind
    /// found, and the unquoted shape to write.
    #[test]
    fn a_non_boolean_value_is_refused_naming_the_kind() -> Result<()> {
        let root = scratch_root(Some("public = \"false\"\nbreak_window_open = true\n"))?;
        let err = Epochs::load(root.path()).expect_err("a string flag value must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("`public` must be a boolean"),
            "the refusal names the key and the expected kind: {msg}"
        );
        assert!(
            msg.contains("found a string"),
            "the refusal names what was found: {msg}"
        );
        assert!(
            msg.contains("no quotes"),
            "the refusal carries the fix: {msg}"
        );
        Ok(())
    }

    /// §3.3e — a missing key is refused naming it and stating that no
    /// default exists to guess.
    #[test]
    fn a_missing_key_is_refused_naming_it() -> Result<()> {
        let root = scratch_root(Some("break_window_open = true\n"))?;
        let err = Epochs::load(root.path()).expect_err("a missing key must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("`public` is missing"),
            "the refusal names the missing key: {msg}"
        );
        assert!(
            msg.contains("no default to guess"),
            "the refusal states why there is no default: {msg}"
        );
        assert!(
            msg.contains("re-add the line"),
            "the refusal carries the fix: {msg}"
        );
        Ok(())
    }

    /// §3.3f — the REAL tree's `formats/EPOCHS.toml` loads and carries
    /// today's ratified state: `public = false`, window open. This is
    /// the machine watchdog over the public switch — the flag only the
    /// owner flips flips visibly, and an unnoticed shift turns this test
    /// red before any gate can act on the new state.
    #[test]
    fn the_real_tree_file_carries_todays_ratified_state() -> Result<()> {
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set under cargo");
        let root = PathBuf::from(manifest_dir)
            .parent()
            .expect("xtask manifest dir has a parent")
            .to_path_buf();
        let epochs = Epochs::load(&root)?;
        assert!(
            !epochs.public,
            "the public switch has moved — flipping it is an owner-only act \
             (PROP-044 §7 `##THE-PUBLIC-SWITCH`); if this shift is yours, \
             this test going red IS the announcement"
        );
        assert!(
            epochs.break_window_open,
            "the break window has closed — a closed window must be an owner \
             decision, not an accident this test swallows"
        );
        Ok(())
    }
}
