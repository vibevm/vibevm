//! Manifest wire conversion retains all source-policy and plane validation.
use super::*;

impl TryFrom<ManifestWire> for Manifest {
    type Error = String;

    fn try_from(wire: ManifestWire) -> Result<Self, Self::Error> {
        if wire.translations.is_some() {
            return Err(
                "[translations] is not a manifest table — a source stores no list of its \
                 translations, and which adaptations exist is computed from the [translates] \
                 edges of the translation packages themselves \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-NO-TRANSLATIONS-TABLE; \
                  fix: delete the table — each translation declares [translates] pointing here)"
                    .to_string(),
            );
        }
        if let Some(meta) = &wire.visibility {
            validate_visibility(meta)?;
        }
        let (overrides, override_table) = match wire.override_wire {
            Some(OverrideWire::Registry(entries)) => (entries, None),
            Some(OverrideWire::Visibility(table)) => {
                table.targets()?;
                (Vec::new(), Some(table))
            }
            None => (Vec::new(), None),
        };
        let extensions = wire
            .extensions
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;
        let mechanisms = wire
            .mechanisms
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;
        let mechanism_routes: MechanismRoutes = wire.mechanism_routes.try_into()?;
        let artifacts = wire.artifacts.map(TryInto::try_into).transpose()?;
        let deploy = wire.deploy.map(TryInto::try_into).transpose()?;
        Ok(Self {
            project: wire.project,
            package: wire.package,
            template: wire.template,
            system_prompt: wire.system_prompt,
            application: wire.application,
            installer_options: wire.installer_options,
            workspace: wire.workspace,
            origin: wire.origin,
            requires: wire.requires,
            requires_any: wire.requires_any,
            provides: wire.provides,
            obsoletes: wire.obsoletes,
            conflicts: wire.conflicts,
            recommends: wire.recommends,
            suggests: wire.suggests,
            embedded_sources: wire.embedded_sources,
            skills: wire.skills,
            binaries: wire.binaries,
            mcp_servers: wire.mcp_servers,
            documents: wire.documents,
            documentation: wire.documentation,
            translates: wire.translates,
            media: wire.media,
            navigation: wire.navigation,
            glossary: wire.glossary,
            hooks: wire.hooks,
            extensions,
            run_commands: wire.run_commands,
            extension_controls: wire.extension_controls.into(),
            mechanism_decls: mechanisms,
            mechanism_routes,
            artifacts,
            deploy,
            remap: wire.remap,
            policy: wire.policy,
            compatibility: wire.compatibility,
            boot_snippet: wire.boot_snippet,
            features: wire.features,
            configuration: wire.configuration,
            conditional_deps: wire.conditional_deps,
            active: wire.active,
            llm: wire.llm,
            registries: wire.registries,
            source_policy: wire.source_policy,
            mirrors: wire.mirrors,
            overrides,
            visibility: wire.visibility,
            override_table,
            i18n: wire.i18n,
            boot: wire.boot,
            compile: wire.compile,
        })
    }
}

impl TryFrom<Manifest> for ManifestWire {
    type Error = String;

    fn try_from(manifest: Manifest) -> Result<Self, Self::Error> {
        manifest.validate().map_err(|error| error.to_string())?;
        let has_project = manifest.project.is_some();
        let has_package = manifest.package.is_some();
        validate_extension_declarations(&manifest.extensions, has_project, has_package)?;
        // The identical whole-document validator `Manifest::validate` runs.
        // A document that cannot be read back is never written.
        validate_plane(&manifest)?;
        let override_wire = match (manifest.overrides.is_empty(), manifest.override_table) {
            (false, Some(_)) => {
                return Err("manifest cannot serialize registry [[override]] and visibility [override] together".into());
            }
            (false, None) => Some(OverrideWire::Registry(manifest.overrides)),
            (true, Some(table)) => Some(OverrideWire::Visibility(table)),
            (true, None) => None,
        };
        let extensions = manifest
            .extensions
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;
        let mechanisms = manifest
            .mechanism_decls
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;
        let mechanism_routes = manifest.mechanism_routes.try_into()?;
        let artifacts = manifest.artifacts.map(TryInto::try_into).transpose()?;
        let deploy: Option<DeploySectionWire> =
            manifest.deploy.map(TryInto::try_into).transpose()?;
        Ok(Self {
            project: manifest.project,
            package: manifest.package,
            template: manifest.template,
            system_prompt: manifest.system_prompt,
            application: manifest.application,
            installer_options: manifest.installer_options,
            workspace: manifest.workspace,
            origin: manifest.origin,
            requires: manifest.requires,
            requires_any: manifest.requires_any,
            provides: manifest.provides,
            obsoletes: manifest.obsoletes,
            conflicts: manifest.conflicts,
            recommends: manifest.recommends,
            suggests: manifest.suggests,
            embedded_sources: manifest.embedded_sources,
            skills: manifest.skills,
            binaries: manifest.binaries,
            mcp_servers: manifest.mcp_servers,
            documents: manifest.documents,
            documentation: manifest.documentation,
            translates: manifest.translates,
            media: manifest.media,
            navigation: manifest.navigation,
            glossary: manifest.glossary,
            translations: None,
            hooks: manifest.hooks,
            extensions,
            run_commands: manifest.run_commands,
            extension_controls: manifest.extension_controls.into(),
            mechanisms,
            mechanism_routes,
            artifacts,
            deploy,
            remap: manifest.remap,
            policy: manifest.policy,
            compatibility: manifest.compatibility,
            boot_snippet: manifest.boot_snippet,
            features: manifest.features,
            configuration: manifest.configuration,
            conditional_deps: manifest.conditional_deps,
            active: manifest.active,
            llm: manifest.llm,
            registries: manifest.registries,
            source_policy: manifest.source_policy,
            mirrors: manifest.mirrors,
            override_wire,
            visibility: manifest.visibility,
            i18n: manifest.i18n,
            boot: manifest.boot,
            compile: manifest.compile,
        })
    }
}
