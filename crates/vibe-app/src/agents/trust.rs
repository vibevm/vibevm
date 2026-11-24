//! Resolving the effective trust of one run from the two files that carry
//! it (PROP-064 `##consent`, PROP-065 `##ordinary-trust`).
//!
//! The mechanism, the patterns and the law that a deny wins are
//! `vibe_core::trust`, which reads nothing. This module is the one place
//! that turns the files into it: the base list vibe ships, then the
//! `[trust]` table of the applied site profile, then `trusted_groups` /
//! `untrusted_groups` of `<settings>/agents.toml`. Every consumer — an
//! agent step, a dependency's `slot:` contribution and `vibe agent list` — reads the value this returns and no second list.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust");

use std::path::Path;

use specmark::spec;
use vibe_core::site_profile::{SITE_PROFILE_FILE, installed_at};
use vibe_core::trust::{TrustError, TrustLists, TrustPolicy};

use crate::settings::SettingsRoot;

use super::{AgentError, AgentsConfig};

/// The effective trust of this machine: base ++ site ++ user, minus every
/// group the site's or the user's deny list covers.
///
/// `config` is the already-read `agents.toml`, or `None` when the file does
/// not exist or could not be read — the base and site layers still apply,
/// because an unreadable personal file is not a statement about trust.
///
/// ```
/// let dir = tempfile::tempdir().unwrap();
/// let settings = vibe_app::SettingsRoot::new(dir.path());
///
/// // With no file of either kind, what vibe ships is the whole answer.
/// let shipped = vibe_app::effective_trust(&settings, None).unwrap();
/// assert!(shipped.trusts("org.vibevm.ai-native"));
/// assert!(!shipped.trusts("org.example"));
///
/// // The user adds a group and refuses one subtree of the base.
/// let config: vibe_app::AgentsConfig = toml::from_str(
///     "trusted_groups = ['org.example']\nuntrusted_groups = ['org.vibevm.ai-native']\n",
/// )
/// .unwrap();
/// let policy = vibe_app::effective_trust(&settings, Some(&config)).unwrap();
/// assert!(policy.trusts("org.example"));
/// assert!(policy.trusts("org.vibevm.node"));
/// assert!(!policy.trusts("org.vibevm.ai-native"));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust")]
pub fn effective_trust(
    settings: &SettingsRoot,
    config: Option<&AgentsConfig>,
) -> Result<TrustPolicy, AgentError> {
    let site = site_lists(&settings.path().join(SITE_PROFILE_FILE))?;
    let user = config
        .map(|config| TrustLists::new(&config.trusted_groups, &config.untrusted_groups))
        .unwrap_or_default();
    TrustPolicy::effective(&site, &user).map_err(|error| {
        let TrustError::Pattern { entry, reason, .. } = &error;
        let list = config.and_then(|config| {
            [
                ("trusted_groups", &config.trusted_groups),
                ("untrusted_groups", &config.untrusted_groups),
            ]
            .into_iter()
            .find_map(|(list, entries)| {
                entries
                    .iter()
                    .any(|written| written.trim() == entry.trim())
                    .then_some(list)
            })
        });
        let file = settings.agents_path().display().to_string();
        match list {
            Some(list) => AgentError::UserTrustEntry {
                entry: entry.trim().to_owned(),
                list,
                file,
                why: if entry.trim() == "**" {
                    "a subtree (org.example.*) already covers every depth below its root".into()
                } else {
                    reason.replace('`', "")
                },
            },
            None => AgentError::Trust {
                file,
                reason: error.to_string(),
            },
        }
    })
}

/// The site layer: the `[trust]` table of the applied profile, or nothing.
///
/// A stored profile that no longer validates is an error here for the same
/// reason it is one everywhere else — ignoring it would silently drop a
/// deny entry an administrator wrote, which is the one failure mode a trust
/// mechanism must not have.
fn site_lists(path: &Path) -> Result<TrustLists, AgentError> {
    match installed_at(path) {
        Ok(Some(installed)) => Ok(installed.profile.trust),
        Ok(None) => Ok(TrustLists::default()),
        Err(error) => Err(AgentError::Trust {
            file: path.display().to_string(),
            reason: error.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three layers in one file-backed run, and the deny that wins over
    /// the base — the owner's worked example, read off disk rather than
    /// built in memory.
    #[test]
    fn the_site_profile_is_the_middle_layer_and_its_deny_covers_the_base() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        std::fs::write(
            dir.path().join(SITE_PROFILE_FILE),
            "schema = 1\nname = 'corp'\n[trust]\ntrusted_groups = ['org.corp.*']\n\
             untrusted_groups = ['org.vibevm.ai-native']\n",
        )
        .unwrap();
        let policy = effective_trust(&settings, None).unwrap();
        assert!(policy.trusts("org.corp.tools"), "the site's own trust");
        assert!(
            policy.trusts("org.vibevm.node"),
            "the base, less one subtree"
        );
        assert!(
            !policy.trusts("org.vibevm.ai-native"),
            "the site refuses one group vibe trusts out of the box",
        );

        // The user cannot take back what the site denied: a deny wins over
        // every trusted entry of every layer, including this one.
        let config: AgentsConfig =
            toml::from_str("trusted_groups = ['org.vibevm.ai-native']\n").unwrap();
        let policy = effective_trust(&settings, Some(&config)).unwrap();
        assert!(!policy.trusts("org.vibevm.ai-native"));
    }

    /// Registry refusals identify the list and actual repair path without
    /// nesting the core refusal inside the application refusal.
    #[test]
    fn registry_trust_refusal_names_list() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let file = settings.agents_path().display().to_string();
        for list in ["trusted_groups", "untrusted_groups"] {
            for (written, entry, why) in [
                (
                    "**",
                    "**",
                    "a subtree (org.example.*) already covers every depth below its root",
                ),
                (
                    "  **  ",
                    "**",
                    "a subtree (org.example.*) already covers every depth below its root",
                ),
                (
                    " org.*.tools ",
                    "org.*.tools",
                    "a wildcard is only ever the whole last label of a subtree",
                ),
                (
                    " org.** ",
                    "org.**",
                    "** is not a pattern — a subtree already covers every depth below its root",
                ),
                ("   ", "", "it is empty"),
            ] {
                let config: AgentsConfig =
                    toml::from_str(&format!("{list} = ['org.example.*', '{written}']\n")).unwrap();
                let error = effective_trust(&settings, Some(&config)).unwrap_err();
                assert!(matches!(
                    &error,
                    AgentError::UserTrustEntry {
                        entry: actual_entry, list: actual_list, file: actual_file, why: actual_why,
                    } if actual_entry == entry && *actual_list == list
                        && actual_file == &file && actual_why == why
                ));
                assert_eq!(error.code(), "trusted_groups_invalid");
                assert_eq!(
                    error.to_string(),
                    format!(
                        "{entry} in {list} is neither a group nor a subtree \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust; \
                         fix: replace {entry} in {file} with a group (org.example) or a subtree (org.example.*)): {why}"
                    )
                );
            }
        }
        // Both lists are invalid: the policy still validates trusted first.
        let config: AgentsConfig =
            toml::from_str("trusted_groups = [' ** ']\nuntrusted_groups = ['org.*.tools', '**']\n")
                .unwrap();
        assert!(matches!(
            effective_trust(&settings, Some(&config)).unwrap_err(),
            AgentError::UserTrustEntry { list: "trusted_groups", entry, .. } if entry == "**"
        ));
    }

    /// A bad entry in either file refuses with the file named, rather than
    /// being dropped into a trust answer nobody wrote.
    #[test]
    fn a_bad_entry_is_refused_with_the_file_that_carries_it() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let config: AgentsConfig = toml::from_str("trusted_groups = ['org.*.tools']\n").unwrap();
        let error = effective_trust(&settings, Some(&config))
            .unwrap_err()
            .to_string();
        assert!(error.contains("agents.toml"), "{error}");
        assert!(error.contains("org.*.tools"), "{error}");

        std::fs::write(
            dir.path().join(SITE_PROFILE_FILE),
            "schema = 1\n[trust]\nuntrusted_groups = ['**']\n",
        )
        .unwrap();
        let path = dir.path().join(SITE_PROFILE_FILE);
        let reason = installed_at(&path).unwrap_err().to_string();
        for config in [None, Some(&config)] {
            let error = effective_trust(&settings, config).unwrap_err();
            assert!(matches!(&error, AgentError::Trust { .. }));
            assert_eq!(error.code(), "trusted_groups_invalid");
            let error = error.to_string();
            assert_eq!(
                error,
                format!(
                    "`{}` carries a trust entry vibe cannot read: {reason} \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust; \
                     fix: correct the entry — each one is a group (`org.example`) or a subtree \
                     (`org.example.*`) in `trusted_groups` / `untrusted_groups`)",
                    path.display()
                )
            );
            assert!(error.contains(SITE_PROFILE_FILE), "{error}");
            assert!(error.contains("trust.untrusted_groups[0]"), "{error}");
        }
    }
}
