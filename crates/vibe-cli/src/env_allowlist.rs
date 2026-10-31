//! Which names a user-config `[env]` table may promote into the process
//! environment. The rule that decides is pure and lives here; the
//! promotion itself, and its recorded deviation, stay at the composition
//! root in `main`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use std::collections::BTreeMap;

/// The environment-variable name prefixes a user-config `[env]` table
/// is allowed to set. Closed on purpose; [`promote_user_config_env`]
/// carries the reasoning, and widening it is an edit here.
pub(crate) const PROMOTABLE_ENV_PREFIXES: [&str; 2] = ["VIBE_", "VIBEVM_"];

/// Whether `name` is inside the promotion allowlist.
///
/// `VIBEVM_` needs its own entry rather than falling out of `VIBE_`:
/// `VIBEVM_HOME` does not start with `VIBE_` — the fifth character is
/// `V`, not the separator.
fn is_promotable_env_name(name: &str) -> bool {
    PROMOTABLE_ENV_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Split a `[env]` table into the entries the allowlist admits (as
/// name/value pairs) and the names it turns away.
///
/// Pure, and separate from the mutation, so the rule that decides what
/// reaches the process can be tested without a test that mutates the
/// process environment to check it.
pub(crate) fn partition_env_promotions(
    env: &BTreeMap<String, String>,
) -> (Vec<(&str, &str)>, Vec<&str>) {
    let mut admitted = Vec::new();
    let mut rejected = Vec::new();
    for (name, value) in env {
        if is_promotable_env_name(name) {
            admitted.push((name.as_str(), value.as_str()));
        } else {
            rejected.push(name.as_str());
        }
    }
    (admitted, rejected)
}

#[cfg(test)]
#[path = "env_allowlist/tests.rs"]
mod tests;
