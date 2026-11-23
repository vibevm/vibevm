//! Exact subject review for the one governed ordinary update route.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#DEFAULT-CONSENT");
use anyhow::{Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use vibe_core::manifest::{ConfigurationIntentDecl, Lockfile, PackageConfiguration};

pub(crate) mod set;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Proposal {
    pub schema: u32,
    pub review_id: String,
    pub package: String,
    pub from_version: String,
    pub to_version: String,
    pub consumer: String,
    pub group: String,
    pub from_member: String,
    pub to_member: String,
    pub added_dependencies: Vec<String>,
    pub removed_dependencies: Vec<String>,
}
#[derive(Debug)]
pub(crate) struct ReviewError {
    pub proposal: Box<Proposal>,
    state: &'static str,
    id: String,
    package: String,
    from_version: String,
    to_version: String,
    consumer: String,
    group: String,
    from_member: String,
    to_member: String,
    added: Vec<String>,
    removed: Vec<String>,
}
impl ReviewError {
    /// The refusal as `refusal_of` reads it: the code, the subject as the
    /// headline, the spec and the exact response command in the tail, and
    /// the proposal's detail as the cause after it.
    fn message(&self) -> String {
        let listed = |items: &[String]| {
            if items.is_empty() {
                "nothing".to_owned()
            } else {
                items.join(", ")
            }
        };
        let headline = if self.state == "stale" {
            format!(
                "{} {} → {}: the answered package configuration review is not the current one",
                self.package, self.from_version, self.to_version
            )
        } else {
            format!(
                "{} {} → {} needs a package configuration review",
                self.package, self.from_version, self.to_version
            )
        };
        format!(
            "{}: {headline} (violates spec://org.vibevm.core/vibevm/common/PROP-069#DEFAULT-CONSENT; \
             fix: `vibe update --package-configuration-review {}`): group {} member {} → {} for \
             consumer {}; added {}; removed {}",
            self.code(),
            self.id,
            self.group,
            self.from_member,
            self.to_member,
            self.consumer,
            listed(&self.added),
            listed(&self.removed),
        )
    }

    pub fn code(&self) -> &'static str {
        if self.state == "stale" {
            "package_configuration_review_stale"
        } else {
            "package_configuration_review_required"
        }
    }
    fn new(proposal: Proposal, stale: bool) -> Self {
        Self {
            state: if stale { "stale" } else { "required" },
            id: proposal.review_id.clone(),
            package: proposal.package.clone(),
            from_version: proposal.from_version.clone(),
            to_version: proposal.to_version.clone(),
            consumer: proposal.consumer.clone(),
            group: proposal.group.clone(),
            from_member: proposal.from_member.clone(),
            to_member: proposal.to_member.clone(),
            added: proposal.added_dependencies.clone(),
            removed: proposal.removed_dependencies.clone(),
            proposal: Box::new(proposal),
        }
    }
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}
impl std::error::Error for ReviewError {}

#[derive(Debug, thiserror::Error)]
#[error("package configuration review response is stale or inapplicable")]
pub(crate) struct InapplicableReview;

struct Binding(Sha256);
impl Binding {
    fn new() -> Self {
        let mut d = Sha256::new();
        d.update(b"vibevm.package-default-review/v1\0");
        Self(d)
    }
    fn count(&mut self, n: usize) {
        self.0.update((n as u64).to_be_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.count(v.len());
        self.0.update(v);
    }
    fn text(&mut self, v: &str) {
        self.bytes(v.as_bytes());
    }
    fn flag(&mut self, v: bool) {
        self.0.update([u8::from(v)]);
    }
    fn optional(&mut self, v: Option<&str>) {
        self.flag(v.is_some());
        if let Some(v) = v {
            self.text(v);
        }
    }
    fn strings(&mut self, v: &[String]) {
        self.count(v.len());
        for s in v {
            self.text(s);
        }
    }
    fn path(&mut self, path: &std::path::Path) -> Result<()> {
        let canonical = path.canonicalize()?;
        if canonical != path {
            bail!("package review requires canonical admitted paths");
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            self.text("unix-path-bytes");
            self.bytes(canonical.as_os_str().as_bytes());
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            self.text("windows-path-utf16le");
            let bytes: Vec<_> = canonical
                .as_os_str()
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect();
            self.bytes(&bytes);
        }
        #[cfg(not(any(unix, windows)))]
        {
            bail!("package review path encoding is unsupported");
        }
        Ok(())
    }
    fn intent(&mut self, input: &ConfigurationIntentDecl) -> Result<()> {
        if !input.parameters.is_empty() {
            bail!("package review parameter input is unsupported");
        }
        self.text(match input.policy {
            vibe_core::configuration::ChoicePolicy::RequireSelection => "require-selection",
            vibe_core::configuration::ChoicePolicy::UseDeclaredDefaults => "use-declared-defaults",
        });
        self.optional(input.profile.as_deref());
        self.flag(input.default_features);
        self.strings(&input.features);
        self.count(input.selections.len());
        for (group, selection) in &input.selections {
            self.text(group);
            self.text(match selection.intent {
                vibe_core::configuration::MergeIntent::Exact => "exact",
                vibe_core::configuration::MergeIntent::Additive => "additive",
            });
            self.flag(selection.members.is_some());
            if let Some(members) = &selection.members {
                self.strings(members);
            }
        }
        self.count(0);
        Ok(())
    }
    fn choices(&mut self, record: &PackageConfiguration) {
        self.flag(record.automatic_basis.is_some());
        if let Some(basis) = &record.automatic_basis {
            self.text(&basis.definition_fingerprint);
            self.count(basis.consumer_choices.len());
            for (consumer, groups) in &basis.consumer_choices {
                self.text(consumer);
                self.count(groups.len());
                for (group, member) in groups {
                    self.text(group);
                    self.text(member);
                }
            }
        }
        self.strings(&record.active_dependencies);
    }
    fn finish(self) -> String {
        format!("package-default-review/1:sha256:{:x}", self.0.finalize())
    }
}

pub(super) struct Subject<'a> {
    pub workspace: &'a std::path::Path,
    pub selected: &'a std::path::Path,
    pub manifest: &'a vibe_core::manifest::Manifest,
}

pub(super) fn gate(
    originals: &vibe_resolver::configuration::PackageConfigurationInputs,
    subject: Subject<'_>,
    prior_bytes: &[u8],
    prior: &Lockfile,
    updated: &[super::scoped::Resolved],
    response: Option<&str>,
) -> Result<()> {
    let Subject {
        workspace,
        selected,
        manifest,
    } = subject;
    if updated.iter().any(|(cached, _, _, _, next)| {
        let key = format!("{}/{}", cached.resolved.group, cached.resolved.name);
        prior
            .configurations
            .get(&key)
            .is_some_and(|old| old.closure_fallback_basis.is_some())
            && next.as_ref().is_none_or(|new| {
                prior.configurations.get(&key).is_none_or(|old| {
                    old.closure_fallback_basis != new.closure_fallback_basis
                        || old.declarations != new.declarations
                        || old.consumers != new.consumers
                })
            })
    }) {
        bail!(
            "closure fallback renewal is not connected; scalar and member-set review tokens cannot authorize it"
        );
    }
    if set::applies(prior, updated) {
        return set::gate(
            originals,
            Subject {
                workspace,
                selected,
                manifest,
            },
            prior_bytes,
            prior,
            updated,
            response,
        );
    }
    let mut changed = Vec::new();
    for (cached, _, _, _, record) in updated {
        let key = format!("{}/{}", cached.resolved.group, cached.resolved.name);
        if let (Some(old), Some(new)) = (prior.configurations.get(&key), record)
            && old.automatic_basis.is_some()
            && (old.automatic_basis != new.automatic_basis || old.declarations != new.declarations)
        {
            changed.push((cached, old, new));
        }
    }
    if changed.is_empty() {
        if response.is_some() {
            return Err(InapplicableReview.into());
        }
        return Ok(());
    }
    if changed.len() != 1 {
        bail!("package review requires exactly one affected package");
    }
    let (cached, old, new) = changed[0];
    if old.consumers != new.consumers {
        bail!("package review original consumer inputs changed");
    }
    let old_basis = old
        .automatic_basis
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing historical basis"))?;
    let new_basis = new
        .automatic_basis
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("removed automatic basis is not reviewable"))?;
    if !old_basis.consumer_feature_defaults.is_empty()
        || !new_basis.consumer_feature_defaults.is_empty()
    {
        bail!("feature-default renewal is not connected; choice review tokens cannot authorize it");
    }
    if !old_basis.consumer_dormant_choices.is_empty() || !new_basis.consumer_dormant_choices.is_empty()
        || old.definition.groups.iter().chain(&new.definition.groups).any(|g|matches!(g,vibe_core::manifest::ConfigurationGroupDecl::Choice{guards,..} if !guards.is_empty())) {
        bail!("guarded automatic default renewal is not connected; legacy scalar review tokens cannot authorize it");
    }
    if !old_basis.consumer_choice_sets.is_empty() || !new_basis.consumer_choice_sets.is_empty() {
        bail!(
            "multi-member automatic default renewal is not connected; scalar review tokens cannot authorize it"
        );
    }
    let mut differences = Vec::new();
    for (consumer, groups) in &old_basis.consumer_choices {
        for (group, member) in groups {
            if let Some(next) = new_basis
                .consumer_choices
                .get(consumer)
                .and_then(|g| g.get(group))
            {
                if member != next {
                    differences.push((
                        consumer.clone(),
                        group.clone(),
                        member.clone(),
                        next.clone(),
                    ));
                }
            } else {
                bail!("removed automatic choice requires a wider review");
            }
        }
    }
    if differences.len() != 1
        || old_basis.consumer_choices.len() != new_basis.consumer_choices.len()
        || old_basis.consumer_choices.iter().any(|(key, groups)| {
            new_basis
                .consumer_choices
                .get(key)
                .is_none_or(|new| groups.keys().ne(new.keys()))
        })
    {
        bail!("package review requires one changed consumer/group");
    }
    let (consumer, group, from_member, to_member) = differences.remove(0);
    let package = format!("{}/{}", cached.resolved.group, cached.resolved.name);
    let before = prior
        .find(&cached.resolved.group, &cached.resolved.name)
        .ok_or_else(|| anyhow::anyhow!("missing prior package"))?;
    let added: Vec<_> = new
        .active_dependencies
        .iter()
        .filter(|d| !old.active_dependencies.contains(d))
        .cloned()
        .collect();
    let removed: Vec<_> = old
        .active_dependencies
        .iter()
        .filter(|d| !new.active_dependencies.contains(d))
        .cloned()
        .collect();
    let mut binding = Binding::new();
    binding.text("update-all");
    binding.path(workspace)?;
    binding.path(selected)?;
    binding.bytes(prior_bytes);
    binding.count(originals.len());
    for (package, consumers) in originals {
        binding.text(package);
        binding.count(consumers.len());
        for (consumer, intent) in consumers {
            binding.text(consumer);
            binding.flag(intent.is_some());
            if let Some(intent) = intent {
                binding.intent(intent)?;
            }
        }
    }

    // Complete original consumer maps of every proposed configured package.
    let records: BTreeMap<_, _> = updated
        .iter()
        .filter_map(|(cached, _, _, _, record)| {
            record.as_ref().map(|r| {
                (
                    format!("{}/{}", cached.resolved.group, cached.resolved.name),
                    r,
                )
            })
        })
        .collect();
    binding.count(records.len());
    for (key, record) in records {
        binding.text(&key);
        binding.count(record.consumers.len());
        for (consumer, intent) in &record.consumers {
            binding.text(consumer);
            binding.intent(intent)?;
        }
        binding.choices(record);
    }
    binding.choices(old);
    binding.text(&before.version.to_string());
    let candidates: BTreeMap<_, _> = updated
        .iter()
        .map(|row| {
            (
                (
                    row.0.resolved.group.to_string(),
                    row.0.resolved.name.to_string(),
                ),
                row,
            )
        })
        .collect();
    binding.count(candidates.len());
    for ((g, n), (cached, deps, _, _, _)) in candidates {
        binding.text(&g);
        binding.text(&n);
        binding.text(&cached.resolved.version.to_string());
        binding.text(&cached.source_uri);
        binding.text(&cached.content_hash);
        let mut deps: Vec<_> = deps.iter().map(ToString::to_string).collect();
        deps.sort();
        binding.strings(&deps);
    }
    binding.text(&consumer);
    binding.text(&group);
    binding.text(&from_member);
    binding.text(&to_member);
    binding.strings(&added);
    binding.strings(&removed);
    let proposal = Proposal {
        schema: 1,
        review_id: binding.finish(),
        package,
        from_version: before.version.to_string(),
        to_version: cached.resolved.version.to_string(),
        consumer,
        group,
        from_member,
        to_member,
        added_dependencies: added,
        removed_dependencies: removed,
    };
    if response == Some(proposal.review_id.as_str()) {
        return Ok(());
    }
    Err(ReviewError::new(proposal, response.is_some()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_refusal_has_code_fix_and_why() {
        let proposal = Proposal {
            schema: 1,
            review_id: "package-default-review/1:sha256:abc".into(),
            package: "org.example/lib".into(),
            from_version: "1.0.0".into(),
            to_version: "2.0.0".into(),
            consumer: "./vibe.toml".into(),
            group: "storage".into(),
            from_member: "sqlite".into(),
            to_member: "postgres".into(),
            added_dependencies: vec!["org.example/pg".into()],
            removed_dependencies: vec!["org.example/sqlite".into()],
        };
        let error = anyhow::Error::from(ReviewError::new(proposal, false));
        let refusal = crate::output::refusal_of(&error);
        assert_eq!(
            refusal.code.as_deref(),
            Some("package_configuration_review_required")
        );
        assert_eq!(
            refusal.headline,
            "org.example/lib 1.0.0 → 2.0.0 needs a package configuration review"
        );
        assert_eq!(
            refusal.fix,
            ["vibe update --package-configuration-review package-default-review/1:sha256:abc"]
        );
        assert_eq!(
            refusal.see.as_deref(),
            Some("spec://org.vibevm.core/vibevm/common/PROP-069#DEFAULT-CONSENT")
        );
        let why = refusal.why.unwrap_or_default();
        assert!(why.contains("sqlite → postgres"), "{why}");
        assert!(why.contains("added org.example/pg"), "{why}");
    }

    #[test]
    fn framed_boundaries_and_options_are_distinct() {
        let mut a = Binding::new();
        a.text("ab");
        a.text("c");
        let mut b = Binding::new();
        b.text("a");
        b.text("bc");
        assert_ne!(a.finish(), b.finish());
        let mut a = Binding::new();
        a.optional(None);
        let mut b = Binding::new();
        b.optional(Some(""));
        assert_ne!(a.finish(), b.finish());
    }
    #[test]
    fn framed_identity_golden() {
        let mut b = Binding::new();
        b.text("golden");
        b.count(2);
        b.optional(Some(""));
        assert_eq!(
            b.finish(),
            "package-default-review/1:sha256:c21c37eedbc368ed513d20b923cbd0a0c6e3e27cebc982d9767a43cd5b36a863"
        );
    }
}
