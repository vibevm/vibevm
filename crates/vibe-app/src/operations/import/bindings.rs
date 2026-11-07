//! Finite import meaning carried through ordinary dependency planning/execution.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-AUTO-BINDINGS");

use crate::configuration::{ConfigurationError, PortableTransfer};
use crate::operations::pending;
use crate::{
    AppError, ImportSecrets, InstallChoice, InstallRequest, Installation, Mark, OperationContext,
    ResolvedPackage,
};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{InstallerOptionDecl, RequirementScope};
use vibe_wire::generated::{apps_manifest as legacy, shared as w};

#[path = "bindings/context.rs"]
mod context;

/// Host-created finite import context. Values are ephemeral; its fingerprint
/// detects retained-context drift and does not authenticate editable storage.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION")]
pub struct ImportScope<'a> {
    target: Option<w::ApplicationImportTarget>,
    roots: Vec<String>,
    bindings: BTreeMap<String, w::ApplicationPortableBinding>,
    active: BTreeSet<String>,
    secret_requirements: BTreeMap<String, Vec<String>>,
    secrets: &'a ImportSecrets,
    fingerprint: String,
}
impl std::fmt::Debug for ImportScope<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportScope")
            .field("target", &self.target)
            .field("roots", &self.roots)
            .field("keys", &self.bindings.keys().collect::<Vec<_>>())
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}
fn invalid(key: &str, reason: impl Into<String>) -> AppError {
    ConfigurationError::Invalid {
        option: key.into(),
        reason: reason.into(),
    }
    .into()
}
fn set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}
impl<'a> ImportScope<'a> {
    /// A retained cohort may query only its captured resolver route. Caller
    /// closures remain the caller's responsibility; no function identity is claimed.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-FINALIZATION"
    )]
    pub(crate) fn admit_lookup(&self, ctx: &OperationContext<'_>) -> Result<(), AppError> {
        let Some(target) = &self.target else {
            return Ok(());
        };
        let owner = super::load(ctx.settings, &target.operation)?;
        if owner.target() != target || owner.context().lookup != pending::lookup(ctx)? {
            return Err(super::store::failure(
                ctx.settings,
                &target.operation,
                "ordinary target-only lookup requires the retained resolver route; restore that context before continuing",
            ));
        }
        Ok(())
    }
    pub(crate) fn released_refusal(
        &self,
        settings: &crate::SettingsRoot,
        state: &crate::steps::StepsFile,
    ) -> Result<bool, AppError> {
        let Some(target) = &self.target else {
            return Ok(false);
        };
        let owner = super::load(settings, &target.operation)?;
        if owner.target() != target {
            return Err(invalid("import", "refused-attempt owner changed"));
        }
        owner.released_refusal(state)
    }
    pub(crate) fn binding(&self, key: &str) -> Option<&w::ApplicationPortableBinding> {
        self.bindings.get(key).filter(|_| self.active.contains(key))
    }
    pub(crate) fn answers(&self, key: &str) -> &[(String, String)] {
        if self.active.contains(key) {
            self.secrets.for_key(key)
        } else {
            &[]
        }
    }
    pub(crate) fn required_secrets(&self, key: &str) -> &[String] {
        self.secret_requirements
            .get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub(crate) fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub(crate) fn completed_binding(
        &self,
        settings: &crate::SettingsRoot,
        key: &str,
        index: &crate::Index,
    ) -> Result<bool, AppError> {
        match &self.target {
            Some(target) => {
                super::completed_binding(settings, target, key, index, self.fingerprint())
            }
            None => Ok(false),
        }
    }

    pub(crate) fn dependency(
        &self,
        owner: &str,
        spelling: &str,
        scope: RequirementScope,
        step: Option<&str>,
    ) -> Result<Option<&w::ApplicationPortableBinding>, AppError> {
        let Some(parent) = self.binding(owner) else {
            return Ok(None);
        };
        let requested = crate::qualified_ref(spelling)?;
        let group = requested.group.as_ref().map(ToString::to_string);
        let mut found_coordinate = false;
        let mut selected = BTreeSet::new();
        for edge in &parent.application.requires {
            let key = pending::coordinate(&edge.key)?;
            if group.as_deref() != Some(key.group.as_str()) || requested.name.as_str() != key.name {
                continue;
            }
            found_coordinate = true;
            let same_scope = matches!(
                (&edge.scope, scope),
                (w::PortableRequirementScope::Build, RequirementScope::Build)
                    | (
                        w::PortableRequirementScope::Runtime,
                        RequirementScope::Runtime
                    )
            );
            if same_scope
                && edge.step.as_deref() == step
                && semver::Version::parse(&key.version).is_ok_and(|v| requested.version.matches(&v))
            {
                selected.insert(edge.key.as_str());
            }
        }
        if !found_coordinate {
            return Ok(None);
        }
        if selected.len() != 1 {
            return Err(invalid(
                owner,
                "current requirement has no unique compatible imported exact binding at this scope/step; review the target configuration",
            ));
        }
        let key = selected
            .into_iter()
            .next()
            .ok_or_else(|| invalid(owner, "missing imported dependency selection"))?;
        self.binding(key)
            .map(Some)
            .ok_or_else(|| invalid(owner, "required imported binding is missing"))
    }

    pub(crate) fn admit(
        &self,
        ctx: &OperationContext<'_>,
        binding: &w::ApplicationPortableBinding,
    ) -> Result<(ResolvedPackage, InstallerOptionDecl), AppError> {
        let key = &binding.application.key;
        if let Some(target) = &self.target
            && pending::actual_home(ctx.settings)? != target.home
        {
            return Err(invalid(key, "import belongs to another home"));
        }
        let mut resolved = pending::read_source(ctx, &binding.source, key)?;
        resolved.source_evidence = if let Some(target) = &self.target {
            super::load(ctx.settings, &target.operation)?
                .source_evidence()
                .get(key)
                .cloned()
        } else {
            None
        };
        let option = resolved
            .options
            .iter()
            .find(|o| {
                o.id == binding.installer
                    && o.platform == binding.platform
                    && o.arch == binding.arch
            })
            .cloned()
            .ok_or_else(|| invalid(key, "prepared dependency installer is unavailable"))?;
        if crate::portable::import_definition(&resolved, &option)? != binding.definition {
            return Err(invalid(key, "target changed; review a fresh import"));
        }
        let meaning = option
            .configuration
            .as_ref()
            .map(|d| d.semantic_fingerprint())
            .transpose()
            .map_err(|_| invalid(key, "invalid target configuration meaning"))?;
        if meaning != binding.configuration_definition {
            return Err(invalid(key, "prepared target basis changed"));
        }
        Ok((resolved, option))
    }

    pub(crate) fn request(
        &self,
        ctx: &OperationContext<'_>,
        binding: &w::ApplicationPortableBinding,
        option: &InstallerOptionDecl,
        accept_license: bool,
    ) -> Result<InstallRequest, AppError> {
        let key = &binding.application.key;
        let declarations = option
            .configuration
            .as_ref()
            .map_or(option.parameters.as_slice(), |d| d.parameters.as_slice());
        let mut answers = self.answers(key).to_vec();
        if answers
            .iter()
            .any(|(id, _)| !declarations.iter().any(|p| &p.id == id && p.secret))
        {
            return Err(invalid(key, "scoped answer is unknown or public"));
        }
        let mut request = InstallRequest {
            choice: InstallChoice {
                installer: Some(binding.installer.clone()),
                platform: Some(binding.platform.clone()),
                arch: Some(binding.arch.clone()),
            },
            hidden: binding.application.hidden,
            mark: match binding.application.mark {
                w::PortableApplicationMark::Manual => Mark::Manual,
                w::PortableApplicationMark::Auto => Mark::Auto,
            },
            accept_license,
            ..Default::default()
        };
        match &binding.application.configuration {
            w::PortableConfigurationState::Configured(source) => {
                request.transfer = Some(PortableTransfer {
                    history: source.history.clone(),
                    target: binding.target_input.clone(),
                    required_secrets: self.required_secrets(key).to_vec(),
                });
                request.parameters = answers;
            }
            w::PortableConfigurationState::LegacyEffective(source) => {
                if option.configuration.is_some() {
                    return Err(invalid(key, "legacy input needs explicit reconfiguration"));
                }
                let parameters = crate::portable::legacy_target_public(
                    key,
                    source,
                    &binding.target_input,
                    option,
                )?;
                let mut missing = Vec::new();
                for id in self.required_secrets(key) {
                    let parameter = declarations
                        .iter()
                        .find(|p| &p.id == id && p.secret)
                        .ok_or_else(|| {
                            invalid(
                                key,
                                "known secret obligation is no longer a declared secret",
                            )
                        })?;
                    if answers.iter().any(|(supplied, _)| supplied == id) {
                        continue;
                    }
                    if let Some(prompt) = ctx.parameter_prompt
                        && let Some(value) = prompt(parameter, None)
                            .map_err(|_| invalid(key, "secret prompt failed"))?
                    {
                        answers.push((id.clone(), value));
                        continue;
                    }
                    missing.push(id.clone());
                }
                if !missing.is_empty() {
                    let Some(target) = &self.target else {
                        return Err(crate::StepError::ParameterInvalid {
                            option: option.id.clone(),
                            parameter: missing.join(", "),
                            reason: "full import requires scoped secret resupply before execution"
                                .into(),
                        }
                        .into());
                    };
                    return Err(ConfigurationError::ImportInputRequired {
                        target: target.clone(),
                        key: key.clone(),
                        parameters: missing,
                    }
                    .into());
                }
                let row = legacy::Application {
                    key: key.clone(),
                    installer: binding.installer.clone(),
                    features: source.features.clone(),
                    parameters,
                    license: binding
                        .application
                        .license_accepted
                        .then_some(legacy::ApplicationLicense::Accepted),
                    mark: legacy::ApplicationMark::Auto,
                    held: binding.application.held,
                    hidden: binding.application.hidden,
                };
                let input = crate::portable::answers(&row, request.choice.clone(), Some(option));
                request.features.enable = input.enable;
                request.features.disable = input.disable;
                request.parameters = input.parameters;
                request.parameters.extend(answers);
            }
        }
        Ok(request)
    }

    pub(crate) fn input_error(&self, key: &str, error: AppError) -> AppError {
        if let AppError::Configuration(ConfigurationError::Unresolved { evaluation, .. }) = &error {
            let ids: Option<Vec<_>> = evaluation
                .questions
                .iter()
                .map(|q| match q {
                    w::ConfigurationQuestion::SecretResupply(p) => Some(p.id.clone()),
                    w::ConfigurationQuestion::Parameter(p) if p.secret => Some(p.id.clone()),
                    _ => None,
                })
                .collect();
            if let Some(target) = &self.target
                && evaluation.problems.is_empty()
                && let Some(parameters) = ids.filter(|ids| !ids.is_empty())
            {
                return ConfigurationError::ImportInputRequired {
                    target: target.clone(),
                    key: key.into(),
                    parameters,
                }
                .into();
            }
        }
        error
    }

    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-INSTALLED-REUSE"
    )]
    pub(crate) fn compatible(
        &self,
        ctx: &OperationContext<'_>,
        binding: &w::ApplicationPortableBinding,
        option: &InstallerOptionDecl,
        installed: &Installation,
    ) -> Result<(), AppError> {
        let key = &binding.application.key;
        if !self.answers(key).is_empty() {
            return Err(invalid(
                key,
                "a new scoped secret answer cannot be compared with an installed opaque value; use explicit modify/reconfiguration or an explicit root skip",
            ));
        }
        let keep_manual = binding.application.mark == w::PortableApplicationMark::Auto
            && installed.lifecycle.mark == Mark::Manual;
        if installed.package.key() != *key
            || installed.option != binding.installer
            || installed.platform != binding.platform
            || installed.arch != binding.arch
            || installed.pending.is_some()
            || !keep_manual
                && (installed.lifecycle.hidden != binding.application.hidden
                    || installed.lifecycle.held != binding.application.held)
            || binding.application.mark == w::PortableApplicationMark::Manual
                && installed.lifecycle.mark != Mark::Manual
        {
            return Err(invalid(
                key,
                "installed selection or ownership policy differs from imported binding; reconcile explicitly before reuse",
            ));
        }
        match (&binding.application.configuration, &installed.configuration) {
            (w::PortableConfigurationState::Configured(source), Some(current)) => {
                let definition = option
                    .configuration
                    .as_ref()
                    .ok_or_else(|| invalid(key, "prepared common definition is unavailable"))?;
                let wanted = crate::configuration::transfer::prepare_required(
                    crate::configuration::transfer::TransferRequest {
                        definition,
                        history: &source.history,
                        target: &binding.target_input,
                        ephemeral: &[],
                        facts: ctx.facts,
                        prompt: None,
                    },
                    self.required_secrets(key),
                )
                .map_err(|_| invalid(key, "cannot reassess installed binding"))?;
                let desired = &wanted.record.evaluation;
                if !desired.problems.is_empty()
                    || desired.questions.iter().any(|q| match q {
                        w::ConfigurationQuestion::SecretResupply(_) => false,
                        w::ConfigurationQuestion::Parameter(p) => !p.secret,
                        _ => true,
                    })
                {
                    return Err(invalid(key, "public input or review required before reuse"));
                }
                let actual = &current.evaluation;
                let features = actual
                    .feature_result
                    .as_ref()
                    .zip(desired.feature_result.as_ref())
                    .is_some_and(|(a, b)| {
                        set(&a.selected) == set(&b.selected) && a.implied == b.implied
                    });
                let public = |parameters: &[w::ConfigurationParameterResult]| {
                    parameters
                        .iter()
                        .filter_map(|p| match p {
                            w::ConfigurationParameterResult::Public(p) => {
                                Some((p.id.clone(), p.clone()))
                            }
                            _ => None,
                        })
                        .collect::<BTreeMap<_, _>>()
                };
                if actual.resolved.is_none()
                    || actual.definition != desired.definition
                    || !same_input(&actual.input, &desired.input)
                    || !features
                    || set(&actual.reliance) != set(&desired.reliance)
                    || public(&actual.parameters) != public(&desired.parameters)
                {
                    return Err(invalid(key, "common meaning differs; reconfigure"));
                }
                for p in &desired.parameters {
                    if let w::ConfigurationParameterResult::Secret(wanted) = p {
                        let required = self.required_secrets(key).contains(&wanted.id);
                        let compatible=actual.parameters.iter().any(|p|matches!(p,w::ConfigurationParameterResult::Secret(current)if current.id==wanted.id&&current.provided==required&&(required||current.origin==wanted.origin)));
                        if !compatible {
                            return Err(invalid(key, "secret provision does not match import"));
                        }
                    }
                }
            }
            (w::PortableConfigurationState::LegacyEffective(source), None) => {
                if option.configuration.is_some()
                    || set(&installed.features) != set(&source.features)
                    || installed.parameters
                        != crate::portable::legacy_target_public(
                            key,
                            source,
                            &binding.target_input,
                            option,
                        )?
                {
                    return Err(invalid(key, "legacy effective input differs; reconfigure"));
                }
                for (id, origin) in &source.secret_origins {
                    if installed.secret_origins.get(id) != Some(origin) {
                        return Err(invalid(key, "legacy secret provenance differs"));
                    }
                }
                for id in self.required_secrets(key) {
                    if !matches!(
                        installed.secret_origins.get(id),
                        Some(w::ConfigurationParameterOrigin::Explicit(_))
                    ) {
                        return Err(invalid(key, "legacy secret provision is insufficient"));
                    }
                }
            }
            _ => {
                return Err(invalid(key, "configuration kind differs; reconfigure"));
            }
        }
        Ok(())
    }
}

fn same_input(a: &w::ConfigurationInput, b: &w::ConfigurationInput) -> bool {
    a.intent.policy == b.intent.policy
        && a.intent.profile == b.intent.profile
        && a.intent.default_features == b.intent.default_features
        && set(&a.intent.features) == set(&b.intent.features)
        && set(&a.disabled_features) == set(&b.disabled_features)
        && a.intent.parameters == b.intent.parameters
        && a.intent.selections.len() == b.intent.selections.len()
        && a.intent.selections.iter().all(|(id, left)| {
            b.intent.selections.get(id).is_some_and(|right| {
                left.intent == right.intent
                    && match (&left.members, &right.members) {
                        (None, None) => true,
                        (Some(a), Some(b)) => set(a) == set(b),
                        _ => false,
                    }
            })
        })
}
