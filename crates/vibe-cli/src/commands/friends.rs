//! `vibe friends` — the sealed-circle report for one provider (PROP-050
//! ##ALLOW-FRIENDS-EXHAUSTIVE): open / sealed / the named circle, who
//! actually befriends it, which grants its allow-friends rejects, and
//! whether it stands in the root's friend closure. The exhaustive closure
//! the sealed design exists to make computable.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use vibe_core::visibility::{AllowFriendsState, friends, load_installed_world};

use crate::output;

/// `vibe friends <group>/<name>` — who may befriend this provider, who
/// actually does, and which grants its allow-friends rejects.
#[derive(Debug, Args)]
pub struct FriendsArgs {
    /// The provider coordinate to report on, in `<group>/<name>` form.
    pub coordinate: String,

    /// Project root with `vibe.toml`. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

/// Run `vibe friends`: load the installed world and print the circle
/// report for one provider. An unknown coordinate is the answer (exit
/// zero), matching `vibe why`; only input/world-read errors fail the run.
pub fn run(_ctx: &output::Context, args: FriendsArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.path)?;
    let world = load_installed_world(&root).map_err(anyhow::Error::msg)?;
    let Some(report) = friends(&world, &args.coordinate) else {
        _ctx.summary(&format!(
            "{} — unknown coordinate: nothing in this world declares it; \
             try `vibe tree` for the installed inventory",
            args.coordinate
        ));
        return Ok(());
    };
    let state = match &report.state {
        AllowFriendsState::Open => "open (allow-friends unset — anyone's grant works)".to_string(),
        AllowFriendsState::Sealed => "sealed (allow-friends = [] — nobody enters)".to_string(),
        AllowFriendsState::Circle(entries) => {
            format!("circle: {}", entries.join(", "))
        }
    };
    _ctx.summary(&format!("{} — {state}", args.coordinate));
    let actual = report
        .actual_friends
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    _ctx.summary(&format!("  actual friends: {}", actual_or_none(&actual)));
    let rejected = report
        .rejected
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    _ctx.summary(&format!("  rejected grants: {}", actual_or_none(&rejected)));
    _ctx.summary(&format!(
        "  in root closure: {}",
        if report.in_root_closure { "yes" } else { "no" }
    ));
    Ok(())
}

/// An empty joined list reads as "none" rather than an empty stretch.
fn actual_or_none(joined: &str) -> &str {
    if joined.is_empty() { "none" } else { joined }
}

#[cfg(test)]
mod tests {
    use super::{FriendsArgs, actual_or_none};

    #[test]
    fn args_carry_coordinate_and_path() {
        let args = FriendsArgs {
            coordinate: "org.x/g".into(),
            path: "some/root".into(),
        };
        assert_eq!(args.coordinate, "org.x/g");
        assert_eq!(args.path.to_string_lossy(), "some/root");
    }

    #[test]
    fn empty_lists_render_as_none() {
        assert_eq!(actual_or_none(""), "none");
        assert_eq!(actual_or_none("a, b"), "a, b");
    }
}
