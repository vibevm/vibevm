//! Tests for `[slot.<slot>.extras.<ecosystem>]` — the user's own per-slot,
//! per-ecosystem extras list (PROP-075 `#SLOT-EXTRAS`).
//!
//! A test module of its own rather than more of `tests.rs`: that file's
//! subject is where the file is and how its existing sections layer, and
//! this one's is a single new table and its editors.

use super::*;
use tempfile::tempdir;

/// The user's own per-slot, per-ecosystem extras list (PROP-075
/// `#SLOT-EXTRAS`). The table had to exist here before a user could write
/// it at all: `UserConfig` is `deny_unknown_fields`, so a file carrying
/// `[slot.node.extras.npm]` was a parse error on **every** command, which
/// is why this is a `vibe-core` change and not only a `vibe-app` one.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_slot_extras_table_parses_round_trips_and_keeps_its_own_order() {
    let body = "[slot.node.extras.npm]\n\
                add = [\"typescript\", \"@scope/pkg@^1.2\", \"; rm -rf $HOME\"]\n\
                drop = [\"corepack\"]\n\
                [slot.java.extras.maven]\nadd = [\"org.example:plugin:1.0\"]\n";
    let cfg: UserConfig = toml::from_str(body).unwrap();
    let npm = cfg.extras_of("node", "npm").unwrap();
    // The order is the file's, and an item is opaque — vibe does not make
    // a shell's sense of it.
    assert_eq!(
        npm.add,
        ["typescript", "@scope/pkg@^1.2", "; rm -rf $HOME"],
        "the user's own order, verbatim"
    );
    assert_eq!(npm.drop, ["corepack"]);
    assert!(cfg.extras_of("java", "maven").unwrap().drop.is_empty());
    // A slot or ecosystem the file never mentions is absent, never an error.
    assert!(cfg.extras_of("node", "cargo").is_none());
    assert!(cfg.extras_of("python", "npm").is_none());
    let rendered = toml::to_string_pretty(&cfg).unwrap();
    assert!(rendered.contains("[slot.node.extras.npm]"), "{rendered}");
    assert_eq!(toml::from_str::<UserConfig>(&rendered).unwrap(), cfg);
}

/// A file with no `[slot]` table at all is the whole of today's world, and
/// it must stay byte-identical through the struct: this is the regression
/// the `vibe-app` → user-config seam of P7c is measured against.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_config_with_no_slot_table_is_unchanged_and_an_unknown_slot_key_still_refuses() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[net]\noffline = true\n").unwrap();
    let cfg = UserConfig::load_from(&path).unwrap();
    assert!(cfg.slot.is_empty());
    assert!(cfg.extras_of("node", "npm").is_none());
    // The sections `UserConfig` already carried are untouched.
    assert!(cfg.net.offline);
    assert_eq!(cfg, UserConfig::load_from(&path).unwrap());
    // `deny_unknown_fields` still bites inside the new tables, so a typo
    // in the user's own list is reported rather than silently dropped.
    assert!(
        toml::from_str::<UserConfig>("[slot.node.extras.npm]\nadds = [\"x\"]\n").is_err(),
        "a mis-spelled member of the extras table must refuse"
    );
    assert!(
        toml::from_str::<UserConfig>("[slot.node]\nextra = {}\n").is_err(),
        "a mis-spelled member of the slot table must refuse"
    );
}

/// The settings editors (`vibe app extras add|drop|import`) append, keep
/// the user's order and never duplicate an exact string.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_extras_editors_append_in_place_and_an_import_reads_one_item_per_line() {
    let mut cfg = UserConfig::default();
    assert_eq!(
        cfg.extras_mut("node", "npm")
            .add_items(["typescript".to_owned(), "pnpm@9".to_owned()]),
        2
    );
    assert_eq!(
        cfg.extras_mut("node", "npm")
            .add_items(["pnpm@9".to_owned()]),
        0
    );
    assert_eq!(
        cfg.extras_mut("node", "npm")
            .drop_ids(["corepack".to_owned()]),
        1
    );
    assert_eq!(
        cfg.extras_of("node", "npm").unwrap().add,
        ["typescript", "pnpm@9"]
    );
    assert_eq!(cfg.extras_of("node", "npm").unwrap().drop, ["corepack"]);
    // `$NVM_DIR/default-packages`, as a file: one item per line, blank
    // lines and `#` comments ignored, everything else verbatim.
    assert_eq!(
        read_item_lines("# my globals\n\n  typescript  \n@scope/pkg@^1.2\npkg#tag\n\n"),
        ["typescript", "@scope/pkg@^1.2", "pkg#tag"]
    );
    assert!(read_item_lines("# nothing but comments\n\n").is_empty());
}

/// Saving keeps the file's comments and unchanged bytes, so a user whose
/// `config.toml` is hand-written does not lose it to `extras add`.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn saving_an_edited_extras_list_preserves_the_rest_of_the_users_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# my own notes\n[net]\noffline = true\n").unwrap();
    let mut cfg = UserConfig::load_from(&path).unwrap();
    cfg.extras_mut("node", "npm")
        .add_items(["typescript".to_owned()]);
    cfg.save_to(&path).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("# my own notes"), "{body}");
    assert!(body.contains("typescript"), "{body}");
    let back = UserConfig::load_from(&path).unwrap();
    assert!(back.net.offline);
    assert_eq!(back.extras_of("node", "npm").unwrap().add, ["typescript"]);
}
