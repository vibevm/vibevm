//! Exhaustive, read-only projection of the retained extension registry.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use vibe_core::machine_json_path;
use vibe_core::manifest::{
    ExtensionAppliesTo, ExtensionConfig, ExtensionHandler, ExtensionIrLevel, ExtensionPass,
    ExtensionPassKind, ExtensionWhen,
};
use vibe_lifecycle::{
    ContributionTier, EffectiveManifestKind, ExtensionProvider, RegistryState, RegistryView,
    SelectorSubject as DomainSelectorSubject,
};
use vibe_wire::generated::extensions_report::{
    AppliesTo, ExtensionEntry, ExtensionsReport, Handler, HandlerAgent, HandlerBinary,
    HandlerBuiltin, HandlerNative, HandlerScript, IrLevel, JsonMap, ManifestKind,
    NativeObservation, Order, PackageKind as ReportPackageKind, Pass, PassKind, Project, Provider,
    ProviderSource, SelectorSubject, SelectorSubjectKind, State, Tier,
};

use crate::cli::ExtensionsArgs;
use crate::output;

use vibe_orchestrator as world;

pub fn run(ctx: &output::Context, args: ExtensionsArgs) -> Result<()> {
    let loaded = world::inspect(&args.path)?;
    let views = loaded
        .registry
        .exhaustive(DomainSelectorSubject::unscoped());
    let declarations = views
        .iter()
        .enumerate()
        .map(|(sequence, view)| project_entry(sequence, *view))
        .collect::<Result<Vec<_>>>()?;
    let effective_count = declarations.iter().filter(|row| row.effective).count();
    let report = ExtensionsReport {
        command: "extensions".into(),
        count: count_u32(declarations.len(), "declaration count")?,
        declarations,
        effective_count: count_u32(effective_count, "effective declaration count")?,
        notices: loaded
            .registry
            .notices()
            .iter()
            .map(ToString::to_string)
            .collect(),
        ok: true,
        project: Project {
            effective_stack: loaded.effective_stack.as_ref().map(ToString::to_string),
            identity: loaded.host_identity.to_string(),
            manifest_kind: manifest_kind(loaded.manifest_kind),
            root: loaded.project.root,
            version: loaded.project.version,
        },
        selector_subject: SelectorSubject {
            kind: SelectorSubjectKind::Unscoped,
            package: None,
            path: None,
        },
    };

    if ctx.is_json() {
        return ctx.emit_json(&report);
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "{} extension declaration(s), {} effective",
            report.count, report.effective_count
        ));
        return Ok(());
    }
    ctx.heading("Extensions");
    for row in &report.declarations {
        ctx.step(&format!(
            "[{}] {} — point={} handler={} provider={}@{} tier={} state={}",
            row.sequence,
            row.key,
            row.point,
            handler_kind(&row.handler),
            row.provider.identity,
            row.provider.version,
            tier_text(&row.tier),
            state_text(&row.state),
        ));
    }
    ctx.summary(&format!(
        "{} extension declaration(s), {} effective",
        report.count, report.effective_count
    ));
    Ok(())
}

fn project_entry(sequence: usize, view: RegistryView<'_>) -> Result<ExtensionEntry> {
    let row = view.row;
    let declaration = row.declaration();
    Ok(ExtensionEntry {
        activated: row.is_activated(),
        applies_to: declaration.applies_to.as_ref().map(applies_to),
        authored_auto: declaration.auto,
        authored_config: config(declaration.config.as_ref())?,
        auto: row.active_by_default(),
        compiler_internals: declaration.compiler_internals.unwrap_or(false),
        disabled: row.is_disabled(),
        effective: view.is_effective(),
        effective_config: config(row.effective_config())?,
        handler: handler(&declaration.handler),
        id: declaration.id.clone(),
        inputs: declaration.inputs.clone(),
        key: row.key().to_string(),
        native: matches!(declaration.handler, ExtensionHandler::Native { .. }).then_some(
            NativeObservation {
                artifact_path: None,
                build_state: "unavailable".into(),
                content_hash: None,
            },
        ),
        natural_tier: tier(row.natural_tier()),
        order: Order {
            activation: optional_ordinal(row.activation_ordinal(), "activation ordinal")?,
            declaration: count_u32(row.declaration_ordinal(), "declaration ordinal")?,
            provider: optional_ordinal(row.provider_ordinal(), "provider ordinal")?,
        },
        pass: declaration.pass.as_ref().map(extension_pass),
        point: declaration.point.to_string(),
        provider: provider(row.provider()),
        selector_matches: view.selector_matches,
        sequence: count_u32(sequence, "sequence")?,
        state: state(view.state()),
        tier: tier(row.effective_tier()),
        when: when(declaration.when.as_ref())?,
    })
}

fn provider(source: &ExtensionProvider) -> Provider {
    match source {
        ExtensionProvider::Dependency(provider) => Provider {
            content_hash: Some(provider.content_hash.to_string()),
            identity: provider.id.to_string(),
            kind: Some(package_kind(provider.kind)),
            root: Some(machine_json_path(&provider.root)),
            source: ProviderSource::Dependency,
            version: provider.version.clone(),
        },
        ExtensionProvider::Host(provider) => Provider {
            content_hash: provider.content_hash.as_ref().map(ToString::to_string),
            identity: provider.identity.to_string(),
            kind: provider.kind.map(package_kind),
            root: Some(machine_json_path(&provider.root)),
            source: ProviderSource::Host,
            version: provider.version.clone(),
        },
    }
}

fn handler(source: &ExtensionHandler) -> Handler {
    match source {
        ExtensionHandler::Builtin { name } => {
            Handler::Builtin(Box::new(HandlerBuiltin { name: name.clone() }))
        }
        ExtensionHandler::Script { base } => Handler::Script(Box::new(HandlerScript {
            base: machine_json_path(base),
        })),
        ExtensionHandler::Binary { name } => {
            Handler::Binary(Box::new(HandlerBinary { name: name.clone() }))
        }
        ExtensionHandler::Native {
            crate_dir,
            prebuilt,
        } => Handler::Native(Box::new(HandlerNative {
            crate_dir: crate_dir.as_ref().map(|path| machine_json_path(path)),
            prebuilt: prebuilt.as_ref().map(|paths| {
                paths
                    .iter()
                    .map(|(platform, path)| (platform.clone(), machine_json_path(path)))
                    .collect()
            }),
        })),
        ExtensionHandler::Agent { prompt } => Handler::Agent(Box::new(HandlerAgent {
            prompt: prompt.clone(),
        })),
    }
}

fn applies_to(source: &ExtensionAppliesTo) -> AppliesTo {
    AppliesTo {
        packages: source.packages.clone(),
        paths: source.paths.clone(),
    }
}

fn extension_pass(source: &ExtensionPass) -> Pass {
    Pass {
        after: source.after.clone(),
        artifact: source.artifact.clone(),
        before: source.before.clone(),
        formats: source.formats.clone(),
        from: source.from.map(ir_level),
        kind: pass_kind(source.kind),
        level: source.level.map(ir_level),
        replace: source.replace.clone(),
        to: source.to.map(ir_level),
    }
}

fn config(source: Option<&ExtensionConfig>) -> Result<Option<JsonMap>> {
    source.map(|value| json_table(value.as_table())).transpose()
}

fn when(source: Option<&ExtensionWhen>) -> Result<Option<JsonMap>> {
    source.map(|value| json_table(value.as_table())).transpose()
}

fn json_table(source: &toml::Table) -> Result<JsonMap> {
    source
        .iter()
        .map(|(key, value)| {
            serde_json::to_value(value)
                .with_context(|| format!("encoding extension field `{key}` as JSON"))
                .map(|value| (key.clone(), Some(value)))
        })
        .collect::<Result<BTreeMap<_, _>>>()
}

fn manifest_kind(kind: EffectiveManifestKind) -> ManifestKind {
    match kind {
        EffectiveManifestKind::Project => ManifestKind::Project,
        EffectiveManifestKind::VirtualWorkspace => ManifestKind::Workspace,
        EffectiveManifestKind::Package(kind) => match kind {
            vibe_core::PackageKind::Flow => ManifestKind::Flow,
            vibe_core::PackageKind::Feat => ManifestKind::Feat,
            vibe_core::PackageKind::Stack => ManifestKind::Stack,
            vibe_core::PackageKind::Tool => ManifestKind::Tool,
            vibe_core::PackageKind::Mcp => ManifestKind::Mcp,
            vibe_core::PackageKind::Lang => ManifestKind::Lang,
        },
    }
}

fn tier(value: ContributionTier) -> Tier {
    match value {
        ContributionTier::Preset => Tier::Preset,
        ContributionTier::Dependency => Tier::Dependency,
        ContributionTier::HostDeclaration => Tier::HostDeclaration,
        ContributionTier::HostActivation => Tier::HostActivation,
    }
}

fn state(value: RegistryState) -> State {
    match value {
        RegistryState::Disabled => State::Disabled,
        RegistryState::Inactive => State::Inactive,
        RegistryState::SelectorMismatch => State::SelectorMismatch,
        RegistryState::Effective => State::Effective,
    }
}

fn package_kind(value: vibe_core::PackageKind) -> ReportPackageKind {
    match value {
        vibe_core::PackageKind::Flow => ReportPackageKind::Flow,
        vibe_core::PackageKind::Feat => ReportPackageKind::Feat,
        vibe_core::PackageKind::Stack => ReportPackageKind::Stack,
        vibe_core::PackageKind::Tool => ReportPackageKind::Tool,
        vibe_core::PackageKind::Mcp => ReportPackageKind::Mcp,
        vibe_core::PackageKind::Lang => ReportPackageKind::Lang,
    }
}

fn pass_kind(value: ExtensionPassKind) -> PassKind {
    match value {
        ExtensionPassKind::Transform => PassKind::Transform,
        ExtensionPassKind::Lowering => PassKind::Lowering,
        ExtensionPassKind::Frontend => PassKind::Frontend,
        ExtensionPassKind::Backend => PassKind::Backend,
    }
}

fn ir_level(value: ExtensionIrLevel) -> IrLevel {
    match value {
        ExtensionIrLevel::Source => IrLevel::Source,
        ExtensionIrLevel::Document => IrLevel::Document,
        ExtensionIrLevel::Closure => IrLevel::Closure,
        ExtensionIrLevel::Lane => IrLevel::Lane,
        ExtensionIrLevel::Emitted => IrLevel::Emitted,
    }
}

fn optional_ordinal(value: Option<usize>, label: &str) -> Result<Option<u32>> {
    value.map(|value| count_u32(value, label)).transpose()
}

fn count_u32(value: usize, label: &str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("{label} does not fit the epoch-1 report"))
}

fn handler_kind(handler: &Handler) -> &'static str {
    match handler {
        Handler::Agent(_) => "agent",
        Handler::Binary(_) => "binary",
        Handler::Builtin(_) => "builtin",
        Handler::Native(_) => "native",
        Handler::Script(_) => "script",
    }
}

fn tier_text(tier: &Tier) -> &'static str {
    match tier {
        Tier::Dependency => "dependency",
        Tier::HostActivation => "host-activation",
        Tier::HostDeclaration => "host-declaration",
        Tier::Preset => "preset",
    }
}

fn state_text(state: &State) -> &'static str {
    match state {
        State::Disabled => "disabled",
        State::Effective => "effective",
        State::Inactive => "inactive",
        State::SelectorMismatch => "selector-mismatch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_and_empty_config_remain_distinct() {
        assert!(config(None).unwrap().is_none());
        let empty = ExtensionConfig::from_table(toml::Table::new());
        assert!(config(Some(&empty)).unwrap().unwrap().is_empty());
    }
}
