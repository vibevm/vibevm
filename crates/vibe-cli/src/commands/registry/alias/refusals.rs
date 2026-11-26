//! Operation-specific alias refusals; transport and table validation stay independent.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name");

use vibe_index::aliases::{AliasFault, AliasTable};

const SHORT: &str = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name";

pub(super) fn missing(registry: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "registry_not_found: no registry named {registry} is configured \
         (violates {SHORT}; fix: `vibe registry list`)"
    )
}

pub(super) fn unsupported(registry: &str, kind: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "alias_table_unsupported: registry {registry} is a {kind} registry and carries no alias table \
         (violates {SHORT}; fix: vibe-index alias): \
         only localdir and monorepo registries own _index/aliases.toml"
    )
}

pub(super) fn not_issued(registry: &str, alias: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "alias_not_issued: registry {registry} issues no short name {alias} \
         (violates {SHORT}; fix: `vibe registry alias {registry} list`)"
    )
}

pub(super) fn exists(registry: &str, alias: &str, held: &str, target: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "alias_exists: {alias} already means {held} in registry {registry} \
         (violates {SHORT}; fix: `vibe registry alias {registry} add {alias} {target} --replace`): \
         one spelling has one meaning; use --replace deliberately or choose another spelling"
    )
}

pub(super) fn faults(registry: &str, faults: &[AliasFault], table: &AliasTable) -> anyhow::Error {
    let Some(AliasFault::Grammar { alias, .. }) = faults
        .iter()
        .find(|fault| matches!(fault, AliasFault::Grammar { .. }))
    else {
        return anyhow::anyhow!(
            "the alias table of registry `{registry}` would not stand:\n  {}",
            faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  "),
        );
    };
    // The target is the row's actual target, never a guessed replacement package.
    let target = table
        .get(alias)
        .map(vibe_index::aliases::spell)
        .unwrap_or_default();
    let others = faults
        .iter()
        .filter(|fault| fault.alias() != alias)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    let others = if others.is_empty() {
        String::new()
    } else {
        format!("; {others}")
    };
    anyhow::anyhow!(
        "invalid_alias: {alias} is not a legal package name \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref; \
         fix: `vibe registry alias {registry} add bad-name {target}`): \
         an alias uses lowercase letters, digits and inner single hyphens{others}"
    )
}
