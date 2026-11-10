//! `vibe app extras add|drop|import`: the editors of the user's **own**
//! list, `[slot.<slot>.extras.<ecosystem>]` in `~/.vibe/config.toml`
//! (PROP-075 `#SLOT-EXTRAS`).
//!
//! These three install nothing and touch no installation. They are the
//! only verbs of the family that write a file, and the file is the user's,
//! so it is read with `load_own` — never the site-layered view — and saved
//! through the comment-preserving writer: a hand-written `config.toml` must
//! survive `extras add`.
//!
//! Items are opaque. An item reaches the file exactly as it was typed, and
//! an `import` reads one item per line with `#` comments and blank lines
//! ignored — the format of `$NVM_DIR/default-packages`, and the format an
//! adapter's own `list` verb writes.
//!
//! **Which ecosystem.** The fact gives these verbs no required
//! `--ecosystem`, and the nvm case (`vibe app extras import node
//! ~/.nvm/default-packages`) reads badly with one, so the id is inferred
//! when it is unambiguous: the user's own file first — a second `add` to a
//! list that already exists needs no package tree at all — then what the
//! slot's current version declares. Several adapters and no flag is a
//! refusal naming them, never a guess.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

use anyhow::{Context, Result, bail};
use vibe_app::SettingsRoot;
use vibe_core::user_config::{UserConfig, read_item_lines};

use crate::cli::ExtrasTargetArgs;
use crate::output;
use crate::output::language::{Role, paint};

/// `vibe app extras add SLOT ITEM…`.
#[allow(clippy::too_many_arguments)]
pub(super) fn add(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    items: &[String],
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let ecosystem = one_ecosystem(ctx, settings, slot, target, embedded_root, offline)?;
    edit(ctx, slot, &ecosystem, "add", |extras| {
        extras.add_items(items.iter().cloned())
    })
}

/// `vibe app extras drop SLOT ID…`.
#[allow(clippy::too_many_arguments)]
pub(super) fn drop(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    ids: &[String],
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let ecosystem = one_ecosystem(ctx, settings, slot, target, embedded_root, offline)?;
    edit(ctx, slot, &ecosystem, "drop", |extras| {
        extras.drop_ids(ids.iter().cloned())
    })
}

/// `vibe app extras import SLOT FILE`.
#[allow(clippy::too_many_arguments)]
pub(super) fn import(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    file: &std::path::Path,
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let ecosystem = one_ecosystem(ctx, settings, slot, target, embedded_root, offline)?;
    let body = std::fs::read_to_string(file)
        .with_context(|| format!("reading the item list `{}`", file.display()))?;
    let items = read_item_lines(&body);
    if items.is_empty() {
        ctx.summary(&format!(
            "vibe app extras import: `{}` names no item, so nothing changed",
            file.display()
        ));
        return Ok(None);
    }
    edit(ctx, slot, &ecosystem, "add", |extras| {
        extras.add_items(items.iter().cloned())
    })
}

/// Load the user's own file, apply `change`, save it back and report what
/// moved. The file's comments and every byte the change does not touch
/// survive, because `UserConfig::save_to` renders over the existing text.
fn edit(
    ctx: &output::Context,
    slot: &str,
    ecosystem: &str,
    list: &str,
    change: impl FnOnce(&mut vibe_core::user_config::SlotExtras) -> usize,
) -> Result<Option<i32>> {
    let Some(path) = UserConfig::default_path() else {
        bail!(
            "no user settings directory could be resolved, so there is no `config.toml` to edit; \
             set $VIBE_SETTINGS or $VIBEVM_USER_CONFIG"
        );
    };
    let mut config = UserConfig::load_own()?;
    let added = change(config.extras_mut(slot, ecosystem));
    config.save_to(&path)?;
    let extras = config
        .extras_of(slot, ecosystem)
        .map(|extras| match list {
            "drop" => extras.drop.len(),
            _ => extras.add.len(),
        })
        .unwrap_or_default();
    ctx.summary(&format!(
        "vibe app extras {list}: {added} new in `[slot.{slot}.extras.{ecosystem}] {list}` ({extras} in all), in {}",
        paint(Role::Accent, &path.display().to_string())
    ));
    if list == "add" {
        ctx.summary(&format!(
            "  the version you have now is unchanged; run {} to put the list into it",
            paint(Role::Accent, &format!("vibe app extras apply {slot}"))
        ));
    }
    Ok(None)
}

/// The one ecosystem these verbs edit: the named one, else the only one
/// the user's own file already has for this slot, else the only one the
/// slot's current version declares. Ambiguity is a refusal, never a guess.
fn one_ecosystem(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<String> {
    if let Some(id) = &target.ecosystem {
        return Ok(id.clone());
    }
    // The user's own file first: a second edit of a list that already
    // exists must not need a package tree, a registry or a network.
    let config = UserConfig::load_own()?;
    if let Some(table) = config.slot.get(slot)
        && table.extras.len() == 1
        && let Some(id) = table.extras.keys().next()
    {
        return Ok(id.clone());
    }
    let declared = declared(ctx, settings, slot, target, embedded_root, offline);
    match declared.as_deref() {
        Ok([only]) => Ok(only.clone()),
        Ok([]) => bail!(
            "the slot `{slot}` declares no ecosystem adapter, so there is no extras list to \
             edit; `vibe app describe <package>` names the adapters a package has"
        ),
        Ok(many) => bail!(
            "the slot `{slot}` declares several ecosystems ({}); name one with `--ecosystem ID`",
            many.join(", ")
        ),
        Err(error) => bail!(
            "the slot `{slot}`'s ecosystems could not be read ({error}); name one with \
             `--ecosystem ID`"
        ),
    }
}

/// The adapter ids the slot's current version declares, read off the
/// listing with **no** observation: nothing is run to answer this.
fn declared(
    ctx: &output::Context,
    settings: &SettingsRoot,
    slot: &str,
    target: &ExtrasTargetArgs,
    embedded_root: Option<&std::path::Path>,
    offline: bool,
) -> Result<Vec<String>> {
    let operation = super::Operation::new(ctx, settings, target, embedded_root, offline)?;
    let document = operation.around(settings, offline, |cx| {
        vibe_app::extras_listing(cx, slot, None, None, false, Default::default())
    })?;
    Ok(document.ecosystems.into_iter().map(|row| row.id).collect())
}
