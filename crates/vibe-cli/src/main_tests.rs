//! The composition root's own reds — the `[env]` promotion allowlist.
//!
//! Their own cell because `main.rs` is the dispatch AND the one
//! sanctioned ambient-env root, and a policy's proofs are a second
//! responsibility from the policy itself.
use super::*;

fn env_table(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// The whole point, stated as the smallest case that shows it: a
/// user config declaring a database URL next to a vibevm variable
/// hands the process the second one and never the first. Asserted
/// on the decision rather than on the live environment, because a
/// test that promotes into its own process env would be mutating
/// global state to observe a rule that is pure.
#[test]
fn allowlist_admits_vibe_names_and_refuses_the_rest() {
    let env = env_table(&[
        ("DATABASE_URL", "postgres://admin:hunter2@db.internal/prod"),
        ("VIBE_THING", "promoted-ok"),
    ]);

    let (admitted, rejected) = partition_env_promotions(&env);

    assert_eq!(admitted, vec![("VIBE_THING", "promoted-ok")]);
    assert_eq!(rejected, vec!["DATABASE_URL"]);
}

#[test]
fn allowlist_covers_both_prefixes() {
    for name in [
        "VIBE_LOG",
        "VIBE_REGISTRY_CACHE",
        "VIBE_NO_DEFAULT_REGISTRY",
        // `VIBEVM_*` is a second prefix, not a special case of the
        // first: `VIBEVM_HOME` does not start with `VIBE_`.
        "VIBEVM_HOME",
        "VIBEVM_PUBLISH_TOKEN_GITHUB",
    ] {
        assert!(is_promotable_env_name(name), "{name} must be promotable");
    }
}

#[test]
fn allowlist_refuses_everything_outside_the_namespace() {
    for name in [
        // The names this rule exists for.
        "DATABASE_URL",
        "AWS_SECRET_ACCESS_KEY",
        "KUBECONFIG",
        "PATH",
        "LD_PRELOAD",
        "HOME",
        // Near-misses: prefix means prefix, and it means the
        // separator too.
        "VIBE",
        "VIBEX_LOG",
        "MY_VIBE_LOG",
        // Case-sensitive on purpose — see `promote_user_config_env`.
        "vibe_thing",
    ] {
        assert!(!is_promotable_env_name(name), "{name} must be refused");
    }
}

/// An empty or absent `[env]` is unchanged and silent: nothing to
/// promote, and nothing to warn about.
#[test]
fn an_empty_table_admits_and_refuses_nothing() {
    let empty = BTreeMap::new();
    let (admitted, rejected) = partition_env_promotions(&empty);
    assert!(admitted.is_empty());
    assert!(rejected.is_empty());
}
