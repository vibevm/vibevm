//! The effective compile-trace request for ONE command.
//!
//! Two facts decide it, and they are equipotent:
//!
//! ```text
//! effective request = --trace-compile OR selected_manifest.[compile].trace
//! ```
//!
//! Neither can switch the other off, which is why this is an `||` and not a
//! precedence ladder: a flag is a one-shot ask and a manifest key is a standing
//! one, and an operator who wrote both meant both.
//!
//! **Only the SELECTED node's manifest decides.** A dependency that traces its
//! own builds cannot switch tracing on for the project that installed it — the
//! host would then be paying for an observer it never asked for, keyed off a
//! file it does not own. The read is deliberately role-blind
//! ([`Manifest::compile_trace_enabled`] is the seam) because a package-rooted
//! checkout is still a consumer.
//!
//! **This function is PURE.** It takes the manifest the command already has;
//! it opens no path, and there is nothing here that could turn a parse failure
//! into `false`. That is deliberate: a helper that read the file itself would
//! be a *second* read of a file the command reads anyway, and two reads are two
//! answers — the second one racing an edit, a mount, or the command's own
//! `--git` rewrite. The single snapshot, and the rule that a stored parse
//! failure is still owed to the boundary that historically reported it, live in
//! [`crate::commands::install::SelectedManifest`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use vibe_core::manifest::Manifest;

/// Whether THIS command records a compile trace, given its selected manifest.
pub(crate) fn effective_request(flag: bool, selected: &Manifest) -> bool {
    flag || selected.compile_trace_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(body: &str) -> Manifest {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(Manifest::FILENAME);
        std::fs::write(&path, body).unwrap();
        Manifest::read(&path).unwrap()
    }

    const PROJECT: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";

    #[test]
    fn the_flag_alone_activates() {
        let plain = manifest(PROJECT);
        assert!(effective_request(true, &plain));
        assert!(!effective_request(false, &plain));
    }

    /// The truth table, stated as one table so a future edit that turns the
    /// `||` into a precedence ladder is red on the row it breaks.
    #[test]
    fn cli_or_selected_manifest_is_the_whole_rule() {
        let off = manifest(PROJECT);
        let on = manifest(&format!("{PROJECT}\n[compile]\ntrace = true\n"));
        for (flag, declared, expected) in [
            (false, false, false),
            (false, true, true),
            (true, false, true),
            // Neither input can switch the other off.
            (true, true, true),
        ] {
            let selected = if declared { &on } else { &off };
            assert_eq!(
                effective_request(flag, selected),
                expected,
                "flag={flag} declared={declared}"
            );
        }
    }

    /// Roles are equipotent: a package-rooted checkout is still a consumer.
    #[test]
    fn a_package_root_requests_exactly_like_a_project_root() {
        let package = manifest(
            "[package]\ngroup = \"org.demo\"\nname = \"thing\"\nversion = \"0.1.0\"\n\
             kind = \"flow\"\n\n[compile]\ntrace = true\n",
        );
        assert!(effective_request(false, &package));
    }
}
