//! Role and package-kind validation for the unified manifest document.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

use crate::error::{Error, Result};
use crate::manifest::extension::validate_extension_declarations;
use crate::manifest::package::{
    ABSTRACT_LIMIT, MCP_ARG_VARS, coordinate_form_is_valid, media_path_is_inside_package,
    pinned_path_form_is_valid, section_id_form_is_valid, validate_visibility,
    version_constraint_is_valid,
};
use crate::manifest::plane::validate_plane;

use super::Manifest;

impl Manifest {
    /// Enforce the role rules: `[project]` ⊕ `[package]`; at least one role
    /// section present; package-role sections require `[package]`.
    pub fn validate(&self) -> Result<()> {
        let has_project = self.project.is_some();
        let has_package = self.package.is_some();
        let has_workspace = self.workspace.is_some();

        if has_project && has_package {
            return Err(Error::InvalidManifest {
                reason: "[project] and [package] are mutually exclusive — a node is \
                         either a plain project or a publishable package, not both"
                    .to_string(),
            });
        }
        if !has_project && !has_package && !has_workspace {
            return Err(Error::InvalidManifest {
                reason: "manifest declares no role — it must carry [project], [package], \
                         or [workspace]"
                    .to_string(),
            });
        }

        if let Some(table) = &self.override_table {
            table
                .targets()
                .map_err(|reason| Error::InvalidManifest { reason })?;
        }
        if let Some(meta) = &self.visibility {
            validate_visibility(meta).map_err(|reason| Error::InvalidManifest { reason })?;
        }

        if !has_package {
            let mut offenders: Vec<&str> = Vec::new();
            if self.application.is_some() {
                offenders.push("[application]");
            }
            if self.application_source.is_some() {
                offenders.push("[application_source]");
            }
            if self.boot_snippet.is_some() {
                offenders.push("[boot_snippet]");
            }
            if !self.provides.is_empty() {
                offenders.push("[provides]");
            }
            if !self.requires_any.is_empty() {
                offenders.push("[[requires_any]]");
            }
            if !self.obsoletes.is_empty() {
                offenders.push("[obsoletes]");
            }
            if !self.conflicts.is_empty() {
                offenders.push("[conflicts]");
            }
            if !self.recommends.is_empty() {
                offenders.push("[recommends]");
            }
            if !self.suggests.is_empty() {
                offenders.push("[suggests]");
            }
            if !self.embedded_sources.is_empty() {
                offenders.push("[[embedded_source]]");
            }
            if !self.skills.is_empty() {
                offenders.push("[[skill]]");
            }
            if !self.binaries.is_empty() {
                offenders.push("[[binary]]");
            }
            if !self.mcp_servers.is_empty() {
                offenders.push("[[mcp_server]]");
            }
            if !self.hooks.is_empty() {
                offenders.push("[hooks]");
            }
            if !self.compatibility.is_empty() {
                offenders.push("[compatibility]");
            }
            if !self.features.is_empty() {
                offenders.push("[features]");
            }
            if !self.conditional_deps.is_empty() {
                offenders.push("[target]");
            }
            if !offenders.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "package-role section(s) {} present without a [package] table",
                        offenders.join(", ")
                    ),
                });
            }
        }

        if let Some(application) = &self.application {
            application
                .validate()
                .map_err(|reason| Error::InvalidManifest { reason })?;
        }
        if let Some(source) = &self.application_source {
            source
                .validate()
                .map_err(|reason| Error::InvalidManifest { reason })?;
        }
        if self.application.is_some() && self.application_source.is_some() {
            return Err(Error::InvalidManifest {
                reason: "[application] and [application_source] are mutually exclusive".into(),
            });
        }

        if has_package && !self.run_commands.is_empty() {
            return Err(Error::InvalidManifest {
                reason: "[[command]] is host-owned and cannot be declared by a publishable package"
                    .into(),
            });
        }
        let mut command_ids = std::collections::BTreeSet::new();
        for command in &self.run_commands {
            command
                .validate()
                .map_err(|reason| Error::InvalidManifest { reason })?;
            if !command_ids.insert(command.id.as_str()) {
                return Err(Error::InvalidManifest {
                    reason: format!("duplicate [[command]] id `{}`", command.id),
                });
            }
        }

        let mut embedded_source_names = std::collections::BTreeSet::new();
        for source in &self.embedded_sources {
            source
                .validate()
                .map_err(|reason| Error::InvalidManifest { reason })?;
            if !embedded_source_names.insert(source.name.as_str()) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "duplicate [[embedded_source]] name `{}` — source names are manifest-local identities",
                        source.name
                    ),
                });
            }
        }
        if !self.embedded_sources.is_empty()
            && self
                .package
                .as_ref()
                .is_some_and(|package| package.materialization.is_in_place())
        {
            return Err(Error::InvalidManifest {
                reason: "[[embedded_source]] requires snapshot/copy materialization in v1; the bridge package slot and external immutable cache have separate ownership"
                    .into(),
            });
        }

        for skill in &self.skills {
            skill
                .validate()
                .map_err(|reason| Error::InvalidManifest { reason })?;
            if let Some(source) = &skill.source
                && !embedded_source_names.contains(source.as_str())
            {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[skill]] `{}` references undeclared [[embedded_source]] `{source}`",
                        skill.name
                    ),
                });
            }
            for resource in &skill.resources {
                if !embedded_source_names.contains(resource.embedded_source.as_str()) {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[[skill.resource]] in `{}` references undeclared [[embedded_source]] `{}`",
                            skill.name, resource.embedded_source
                        ),
                    });
                }
            }
        }

        // Preserve the established diagnostic order: the legacy role,
        // visibility, and package-role checks above still win. Extension
        // declarations are judged next; the existing MCP-kind check stays
        // last. The mechanism/artifact/deploy grammar follows the extension
        // declarations.
        validate_extension_declarations(&self.extensions, has_project, has_package)
            .map_err(|reason| Error::InvalidManifest { reason })?;
        // The build/package/deploy plane has exactly one validator, shared
        // verbatim with `TryFrom<Manifest> for ManifestWire`, so a document
        // that will not parse can never be serialised either.
        validate_plane(self).map_err(|reason| Error::InvalidManifest { reason })?;
        self.validate_mcp_kind()?;
        self.validate_documentation()?;
        Ok(())
    }

    /// The `mcp`-kind laws (PROP-027; VIBEVM-SPEC §4.1): `[[mcp_server]]`
    /// is legal only in `mcp`-kind packages and mandatory there; every
    /// declared server names a `[[binary]]` in the same manifest, server
    /// names are unique, launch args substitute only the closed variable
    /// set; and every package requirement is an exact `=X.Y.Z` pin, so
    /// the served engines and the consumer's gates resolve to one
    /// version set.
    fn validate_mcp_kind(&self) -> Result<()> {
        use crate::package_ref::PackageKind;

        let kind = self.package.as_ref().map(|p| p.kind);
        if kind != Some(PackageKind::Mcp) {
            if !self.mcp_servers.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[mcp_server]] is legal only in `mcp`-kind packages (this manifest is {}) \
                         — the kind IS the taxonomy \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: set [package] kind = \"mcp\", or drop the [[mcp_server]] table)",
                        kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
                    ),
                });
            }
            return Ok(());
        }

        if self.mcp_servers.is_empty() {
            return Err(Error::InvalidManifest {
                reason: "an `mcp`-kind package must declare at least one [[mcp_server]] — \
                         the kind promises a server \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: declare the server, or pick the kind that matches the content)"
                    .to_string(),
            });
        }

        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for s in &self.mcp_servers {
            if !seen.insert(s.name.as_str()) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "duplicate [[mcp_server]] name `{}` \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: server names are the agent-visible identity — make them unique)",
                        s.name
                    ),
                });
            }
            if !self.binaries.iter().any(|b| b.name == s.binary) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[mcp_server]] `{}` names binary `{}` but no [[binary]] declares it \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: the server IS a PROP-025 binary — declare it in [[binary]])",
                        s.name, s.binary
                    ),
                });
            }
            let unknown = s.unknown_arg_vars();
            if !unknown.is_empty() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[mcp_server]] `{}` args carry unknown substitution variable(s) {} \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#manifest; \
                          fix: only {} substitute at registration time)",
                        s.name,
                        unknown.join(", "),
                        MCP_ARG_VARS.join(", "),
                    ),
                });
            }
        }

        for r in &self.requires.packages {
            if !r.version.is_exact_pin() {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "`mcp`-kind packages pin every package requirement exactly, and \
                         `{r}` does not — the served engines and the consumer's gates must \
                         resolve to ONE version set \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#exact-pin; \
                          fix: require `=X.Y.Z`, and bump it in lockstep with the served package)",
                    ),
                });
            }
        }
        Ok(())
    }

    /// The documentation laws of PROP-057: what a `doc` package owes,
    /// what only a `doc` package may say, and the two keys that read
    /// only so the refusal can name the thing that works.
    ///
    /// Every edge here is a coordinate, never a resolved package. The
    /// subject of the core manual is the host's own project coordinate,
    /// which no registry can hand back, so the form is what is checked
    /// and selectability is left to the gate (`##REL-HOST-SUBJECT`).
    fn validate_documentation(&self) -> Result<()> {
        use crate::package_ref::PackageKind;

        let kind = self.package.as_ref().map(|p| p.kind);
        let is_doc = kind == Some(PackageKind::Doc);

        // The two keys an author will reach for and must not find.
        // `[translations]` is refused at the wire boundary, where the
        // table physically arrives; `lang` is refused here, because it
        // rides on `[package]`, the one table both halves share.
        if let Some(meta) = &self.package
            && meta.lang.is_some()
        {
            return Err(Error::InvalidManifest {
                reason: "`lang` is not a [package] field — a package's language is \
                         `[i18n].canonical` (PROP-003 §2.7), and two fields for one fact \
                         is how they drift apart \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-LANGUAGE-FIELD; \
                          fix: write `canonical = \"<BCP-47 tag>\"` under [i18n] instead)"
                    .to_string(),
            });
        }

        // `authorship` — a statement about prose, so only a package that
        // carries prose may make it. A `flow` or a `stack` has no
        // document whose hand a reader could ask about, and a field that
        // meant nothing where it was legal would be read as meaning
        // something.
        if let Some(meta) = &self.package
            && let Some(authorship) = meta.authorship
            && !is_doc
        {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "`authorship = \"{}\"` is legal only in `doc`-kind packages (this manifest \
                     is kind = \"{}\") — it says who wrote the PROSE a documentation carries, \
                     and it is never an attribution of the commits or of the repository \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-AUTHORSHIP; \
                      fix: drop `authorship`, or set [package] kind = \"doc\" if this package \
                      really is documentation)",
                    authorship.as_str(),
                    meta.kind,
                ),
            });
        }

        // `[[documents]]` — required in documentation, meaningless
        // anywhere else: declaring a subject is what makes a package
        // documentation, so the kind and the table cannot disagree.
        if is_doc && self.documents.is_empty() {
            return Err(Error::InvalidManifest {
                reason: "a `doc`-kind package must declare at least one [[documents]] subject — \
                         documentation that documents nothing has no place to be shown \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED; \
                          fix: add [[documents]] with the subject's `package` and a `version` \
                          constraint)"
                    .to_string(),
            });
        }
        if !is_doc && !self.documents.is_empty() {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[[documents]] is legal only in `doc`-kind packages (this manifest is {}) \
                     — the table IS the claim to be documentation \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-DOCUMENT; \
                      fix: set [package] kind = \"doc\", or drop the [[documents]] table)",
                    kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
                ),
            });
        }
        for subject in &self.documents {
            check_coordinate("[[documents]].package", &subject.package)?;
            check_constraint("[[documents]].version", &subject.version)?;
        }

        // `[translates]` — an adaptation is a package of its own, so
        // only a documentation package can be one.
        if let Some(translates) = &self.translates {
            if !is_doc {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[translates] is legal only in `doc`-kind packages (this manifest is {}) \
                         — a translation of documentation is itself documentation \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-PACKAGE-PER-LANGUAGE; \
                          fix: set [package] kind = \"doc\", or drop the [translates] table)",
                        kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
                    ),
                });
            }
            check_coordinate("[translates].package", &translates.package)?;
            check_constraint("[translates].version", &translates.version)?;
        }

        // `[navigation]` — a statement about a page tree, and only
        // documentation has one. The paths are checked for FORM here;
        // whether a pinned page exists is a question about a tree, and
        // `vibe check` is where a tree is read.
        if let Some(navigation) = &self.navigation {
            if !is_doc {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[navigation] is legal only in `doc`-kind packages (this manifest is {}) \
                         — it names pages and sections, and only documentation has a page tree \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                          fix: set [package] kind = \"doc\", or drop the [navigation] table)",
                        kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
                    ),
                });
            }
            for path in &navigation.pinned {
                if !pinned_path_form_is_valid(path) {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[navigation].pinned `{path}` is not a document path — a pin names a \
                             page under the spec root with forward slashes and WITHOUT its \
                             extension, because one document is served as three projections \
                             (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                              fix: write the path alone, e.g. `start/what-vibevm-is`)"
                        ),
                    });
                }
            }
            for section in &navigation.sections {
                if !section_id_form_is_valid(&section.id) {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[[navigation.section]].id `{}` is not a folder — a section is one \
                             top-level folder of the page tree, named as the page paths spell it \
                             (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                              fix: write the first segment alone, e.g. `start`)",
                            section.id
                        ),
                    });
                }
                if section.title.trim().is_empty() {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[[navigation.section]] `{}` carries an empty `title` — the title is \
                             the whole reason a section is named, and an empty one would show a \
                             blank heading over the pages \
                             (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                              fix: give the section its name in this package's own language)",
                            section.id
                        ),
                    });
                }
            }
        }

        // `[documentation]` — the subject's pointer, legal in any kind,
        // and deliberately unversioned.
        if let Some(documentation) = &self.documentation {
            if let Some(primary) = &documentation.primary {
                check_coordinate("[documentation].primary", primary)?;
                if documentation.official.iter().any(|c| c == primary) {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[documentation] names `{primary}` as both `primary` and `official` \
                             — `primary` is already official, and the repetition would show the \
                             same package twice \
                             (violates spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED; \
                              fix: remove it from the `official` list)"
                        ),
                    });
                }
            }
            for coordinate in &documentation.official {
                check_coordinate("[documentation].official", coordinate)?;
            }
        }

        // `[media]` — paths inside this package's own tree. The gate
        // opens the files; this only refuses a shape that could point
        // outside the package at all.
        if let Some(media) = &self.media {
            for (field, path) in media.declared() {
                if !media_path_is_inside_package(path) {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "[media].{field} `{}` is not a path inside this package — images are \
                             source files of the package tree, never absolute paths, parent \
                             escapes or foreign addresses \
                             (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE; \
                              fix: commit the image under the package and name it relatively, \
                              e.g. `media/icon.png`)",
                            path.display()
                        ),
                    });
                }
            }
        }

        // The card. Required of documentation because the index and the
        // shelf must show a name and a paragraph without downloading
        // the package.
        let Some(meta) = &self.package else {
            return Ok(());
        };
        if is_doc && meta.title.as_ref().is_none_or(|t| t.trim().is_empty()) {
            return Err(Error::InvalidManifest {
                reason: "a `doc`-kind package must carry a `title` — the coordinate is its \
                         identity, but the title is what a reader sees on a shelf, in the \
                         language selector and in the page heading \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-TITLE; \
                          fix: add `title = \"…\"` under [package])"
                    .to_string(),
            });
        }
        match &meta.abstract_text {
            None => {
                if is_doc {
                    return Err(Error::InvalidManifest {
                        reason: "a `doc`-kind package must carry an `abstract` — four answers, \
                                 not a slogan: what it covers, for whom, what it assumes known, \
                                 what it leaves out \
                                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DESCRIPTION-AND-ABSTRACT; \
                                  fix: add `abstract = \"\"\"…\"\"\"` under [package]; `description` \
                                  stays the one-line subtitle)"
                            .to_string(),
                    });
                }
            }
            Some(text) => {
                if is_doc && text.trim().is_empty() {
                    return Err(Error::InvalidManifest {
                        reason: "a `doc`-kind package's `abstract` is empty — four answers, not \
                                 an empty string \
                                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DESCRIPTION-AND-ABSTRACT; \
                                  fix: say what it covers, for whom, what it assumes known, and \
                                  what it leaves out)"
                            .to_string(),
                    });
                }
                let length = text.chars().count();
                if length > ABSTRACT_LIMIT {
                    return Err(Error::InvalidManifest {
                        reason: format!(
                            "`abstract` is {length} characters, over the {ABSTRACT_LIMIT} the card \
                             allows — the abstract is read on a shelf card, and past that length \
                             it stops being one \
                             (violates spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DESCRIPTION-AND-ABSTRACT; \
                              fix: keep the four answers and move the rest onto the entry page)"
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

/// One subject/documentation coordinate: `<group>/<name>`, no version,
/// no `kind:` prefix (PROP-057 `##REL-DOCUMENTATION-UNVERSIONED`).
fn check_coordinate(field: &str, value: &str) -> Result<()> {
    if coordinate_form_is_valid(value) {
        return Ok(());
    }
    Err(Error::InvalidManifest {
        reason: format!(
            "{field} `{value}` is not a coordinate — a documentation edge names \
             `<group>/<name>` and nothing else: no version, no `kind:` prefix \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED; \
              fix: write the coordinate alone, e.g. `org.vibevm.core/vibevm`)"
        ),
    })
}

/// One subject-version constraint — the same grammar a dependency's
/// version takes, so one documentation version can serve a range.
fn check_constraint(field: &str, value: &str) -> Result<()> {
    if version_constraint_is_valid(value) {
        return Ok(());
    }
    Err(Error::InvalidManifest {
        reason: format!(
            "{field} `{value}` is not a semver constraint — the edge carries a RANGE, because \
             one documentation version serves many subject versions \
             (violates spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED; \
              fix: write a constraint such as `^1.0`, `>=1, <2` or `=1.2.3`)"
        ),
    })
}
