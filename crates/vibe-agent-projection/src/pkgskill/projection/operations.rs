//! Command-facing projection operations over the shared receipt owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");
use super::*;
use specmark::spec;

impl DeclaredSkillProjection {
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#CMD-SKILL-INSTALL")]
    pub fn install(&self, dry_run: bool) -> Result<Vec<PackageSkillReport>> {
        let mut reports = Vec::new();
        let mut processed = std::collections::BTreeSet::new();
        for task in &self.tasks {
            if task
                .agent
                .skills_root(task.scope, Some(&self.project_root))?
                .is_none()
            {
                reports.push(crate::pkgskill::skipped(
                    &task.name,
                    task.agent,
                    task.scope.as_str(),
                ));
                continue;
            }
            if task.scope == Scope::Project {
                if !processed.insert((task.provider.clone(), task.name.clone())) {
                    continue;
                }
                let aliases = self
                    .tasks
                    .iter()
                    .filter(|candidate| {
                        candidate.scope == Scope::Project
                            && candidate.provider == task.provider
                            && candidate.name == task.name
                    })
                    .map(|candidate| candidate.agent)
                    .collect::<Vec<_>>();
                let binding = collect_project_skill_bindings(&self.project_root)?
                    .into_iter()
                    .find(|b| {
                        b.skill.provider.identity() == task.provider
                            && b.skill.decl.name == task.name
                    })
                    .ok_or_else(|| anyhow::anyhow!("selected project skill vanished"))?;
                reports.extend(crate::pkgskill::receipt::project_subset(
                    &self.project_root,
                    &binding,
                    &aliases,
                    dry_run,
                )?);
                continue;
            }
            let desired = crate::pkgskill::snapshot_source(&task.source, &task.include)?;
            reports.push(crate::pkgskill::install_managed_skill_snapshot(
                task.agent,
                task.scope,
                Some(&self.project_root),
                Some(&task.provider),
                &task.name,
                &desired,
                dry_run,
            )?);
        }
        Ok(reports)
    }

    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
    pub fn activation_intent(
        &self,
        intent: &crate::activation::ActivationIntent,
        retain: bool,
    ) -> Result<()> {
        let mut checked = std::collections::BTreeSet::new();
        for task in &self.tasks {
            if task.scope == Scope::Project {
                if !checked.insert((task.provider.clone(), task.name.clone())) {
                    continue;
                }
                let aliases = self
                    .tasks
                    .iter()
                    .filter(|candidate| {
                        candidate.scope == Scope::Project
                            && candidate.provider == task.provider
                            && candidate.name == task.name
                    })
                    .map(|candidate| candidate.agent)
                    .collect::<Vec<_>>();
                crate::pkgskill::receipt::project_group_intent(
                    &self.project_root,
                    &task.provider,
                    &task.name,
                    &aliases,
                    intent,
                    false,
                )?;
            } else {
                crate::pkgskill::admit_managed_skill_activation(
                    task.agent,
                    task.scope,
                    Some(&self.project_root),
                    &task.provider,
                    &task.name,
                    intent,
                )?;
            }
        }
        if retain {
            let mut retained = Vec::new();
            let mut processed = std::collections::BTreeSet::new();
            for task in &self.tasks {
                let result = if task.scope == Scope::Project {
                    if !processed.insert((task.provider.clone(), task.name.clone())) {
                        continue;
                    }
                    let aliases = self
                        .tasks
                        .iter()
                        .filter(|candidate| {
                            candidate.scope == Scope::Project
                                && candidate.provider == task.provider
                                && candidate.name == task.name
                        })
                        .map(|candidate| candidate.agent)
                        .collect::<Vec<_>>();
                    crate::pkgskill::receipt::project_group_intent(
                        &self.project_root,
                        &task.provider,
                        &task.name,
                        &aliases,
                        intent,
                        true,
                    )
                } else {
                    crate::pkgskill::retain_managed_skill_activation(
                        task.agent,
                        task.scope,
                        Some(&self.project_root),
                        &task.provider,
                        &task.name,
                        intent,
                    )
                };
                result.map_err(|e|e.context(format!("retained intents [{}]; current retention unconfirmed; projection not attempted",retained.join(", "))))?;
                retained.push(format!(
                    "{}/{}:{}:{}",
                    task.provider,
                    task.name,
                    task.agent.as_str(),
                    task.scope.as_str()
                ));
            }
        }
        Ok(())
    }

    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
    pub fn activation(
        &self,
        request: Option<&crate::activation::ActivationIntent>,
        dry_run: bool,
    ) -> Result<Vec<crate::pkgskill::SkillActivationStatus>> {
        let mut rows = Vec::new();
        let mut processed = std::collections::BTreeSet::new();
        for task in &self.tasks {
            if task.scope == Scope::Project {
                let binding = collect_project_skill_bindings(&self.project_root)?
                    .into_iter()
                    .find(|b| {
                        b.skill.provider.identity() == task.provider
                            && b.skill.decl.name == task.name
                    })
                    .ok_or_else(|| anyhow::anyhow!("selected project skill vanished"))?;
                if let Some(request) = request {
                    if !processed.insert((task.provider.clone(), task.name.clone())) {
                        continue;
                    }
                    let aliases = self
                        .tasks
                        .iter()
                        .filter(|candidate| {
                            candidate.scope == Scope::Project
                                && candidate.provider == task.provider
                                && candidate.name == task.name
                        })
                        .map(|candidate| candidate.agent)
                        .collect::<Vec<_>>();
                    rows.extend(crate::pkgskill::receipt::project_group_activation(
                        &self.project_root,
                        &binding,
                        &aliases,
                        request,
                        dry_run,
                    )?);
                } else {
                    rows.push(crate::pkgskill::receipt::project_status(
                        &self.project_root,
                        &binding,
                        task.agent,
                    )?);
                }
                continue;
            }
            let row = if let Some(request) = request {
                let desired = crate::pkgskill::snapshot_source(&task.source, &task.include)?;
                crate::pkgskill::change_managed_skill_activation(
                    crate::pkgskill::SkillActivationTarget {
                        agent: task.agent,
                        scope: task.scope,
                        project: Some(&self.project_root),
                        provider: &task.provider,
                        name: &task.name,
                    },
                    &desired,
                    request,
                    dry_run,
                )?
            } else {
                crate::pkgskill::managed_skill_status(
                    task.agent,
                    task.scope,
                    Some(&self.project_root),
                    &task.provider,
                    &task.name,
                )?
            };
            rows.push(row);
        }
        Ok(rows)
    }

    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-018#CMD-SKILL-UNINSTALL")]
    pub fn uninstall(&self, dry_run: bool) -> Result<Vec<PackageSkillReport>> {
        let mut reports = Vec::with_capacity(self.tasks.len());
        let mut processed = std::collections::BTreeSet::new();
        for task in &self.tasks {
            if task
                .agent
                .skills_root(task.scope, Some(&self.project_root))?
                .is_none()
            {
                reports.push(crate::pkgskill::skipped(
                    &task.name,
                    task.agent,
                    task.scope.as_str(),
                ));
                continue;
            }
            if task.scope == Scope::Project {
                if !processed.insert((task.provider.clone(), task.name.clone())) {
                    continue;
                }
                let aliases = self
                    .tasks
                    .iter()
                    .filter(|candidate| {
                        candidate.scope == Scope::Project
                            && candidate.provider == task.provider
                            && candidate.name == task.name
                    })
                    .map(|candidate| candidate.agent)
                    .collect::<Vec<_>>();
                let binding = collect_project_skill_bindings(&self.project_root)?
                    .into_iter()
                    .find(|b| {
                        b.skill.provider.identity() == task.provider
                            && b.skill.decl.name == task.name
                    })
                    .ok_or_else(|| anyhow::anyhow!("selected project skill vanished"))?;
                reports.extend(crate::pkgskill::receipt::project_remove(
                    &self.project_root,
                    &binding,
                    &aliases,
                    dry_run,
                )?);
                continue;
            }
            reports.push(uninstall_package_skill(
                task.agent,
                task.scope,
                Some(&self.project_root),
                &task.name,
                dry_run,
            )?);
        }
        Ok(reports)
    }

    #[cfg(test)]
    pub(super) fn install_with<F>(
        &self,
        dry_run: bool,
        mut project_one: F,
    ) -> Result<Vec<PackageSkillReport>>
    where
        F: FnMut(
            Agent,
            Scope,
            Option<&Path>,
            &str,
            &Path,
            &[String],
            bool,
        ) -> Result<PackageSkillReport, PackageSkillError>,
    {
        let mut reports = Vec::with_capacity(self.tasks.len());
        for task in &self.tasks {
            reports.push(project_one(
                task.agent,
                task.scope,
                Some(&self.project_root),
                &task.name,
                &task.source,
                &task.include,
                dry_run,
            )?);
        }
        Ok(reports)
    }
}

/// Inspect activation from inventory and receipts only; never hydrate or fetch skill sources.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn declared_skill_activation_status(
    project_root: &Path,
    filter: &DeclaredSkillFilter<'_>,
    scope: Scope,
) -> Result<Vec<crate::pkgskill::SkillActivationStatus>> {
    let agents = match filter.agent {
        Some(value) => Agent::parse_filter(value)?,
        None => Agent::ALL.to_vec(),
    };
    let mut rows = Vec::new();
    for skill in collect_declared_skills(project_root)? {
        if filter
            .provider
            .is_some_and(|p| p != skill.provider.identity())
            || (!filter.names.is_empty() && !filter.names.contains(&skill.decl.name))
        {
            continue;
        }
        for agent in skill_agents(&skill.decl, &agents)? {
            for concrete in scope.expand() {
                if concrete == Scope::Project {
                    let binding = ProjectSkillBinding {
                        activation: Default::default(),
                        skill: skill.clone(),
                        targets: Vec::new(),
                        source_snapshot: "missing".into(),
                        selected_files: None,
                    };
                    rows.push(crate::pkgskill::receipt::project_status(
                        project_root,
                        &binding,
                        agent,
                    )?);
                } else {
                    rows.push(crate::pkgskill::managed_skill_status(
                        agent,
                        concrete,
                        Some(project_root),
                        &skill.provider.identity(),
                        &skill.decl.name,
                    )?);
                }
            }
        }
    }
    if rows.is_empty() {
        bail!("no selected managed skill declaration");
    }
    Ok(rows)
}
