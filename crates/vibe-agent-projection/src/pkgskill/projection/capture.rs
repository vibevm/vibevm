//! Finite observed raw skill closures. Captures are not write authority.
use super::*;
use vibe_safefs::{Project, StableFileSnapshot};

const FILE_CAP: usize = 8 * 1024 * 1024;
const TOTAL_CAP: usize = 64 * 1024 * 1024;
const ENTRY_CAP: usize = 4096;

#[derive(Debug, thiserror::Error)]
#[error("source-capture-stale: selected skill observation changed")]
pub struct SourceCaptureStale;

#[derive(Debug)]
struct RawClosure {
    root: PathBuf,
    correlation: String,
    source: PathBuf,
    include: Vec<String>,
    files: BTreeMap<String, StableFileSnapshot>,
}

impl RawClosure {
    fn observe(root: &Path, source: &Path, include: &[String]) -> Result<Self> {
        super::super::receipt::ensure_no_follow_walk(root, source, false)?;
        let project = Project::open(root)?;
        let correlation = project.root_correlation_token()?;
        let mut pending = vec![(source.to_path_buf(), 0usize)];
        let mut files = BTreeMap::new();
        let mut remaining = TOTAL_CAP;
        let mut entries = 0;
        while let Some((path, depth)) = pending.pop() {
            entries += 1;
            anyhow::ensure!(entries <= ENTRY_CAP && depth <= 64, "skill capture bound");
            super::super::receipt::ensure_no_follow_walk(root, &path, false)?;
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                for child in fs::read_dir(&path)? {
                    pending.push((child?.path(), depth + 1));
                    anyhow::ensure!(pending.len() + entries <= ENTRY_CAP, "skill capture bound");
                }
            } else {
                anyhow::ensure!(metadata.is_file(), "non-regular skill source");
                let relative = if path == source {
                    path.file_name()
                        .context("skill source filename")?
                        .to_str()
                        .context("skill UTF-8 filename")?
                        .to_owned()
                } else {
                    path.strip_prefix(source)?
                        .to_str()
                        .context("skill UTF-8 path")?
                        .replace('\\', "/")
                };
                if path != source
                    && !include.is_empty()
                    && !include
                        .iter()
                        .any(|pattern| super::super::glob_match(pattern, &relative))
                {
                    continue;
                }
                let rooted = path
                    .strip_prefix(root)?
                    .to_str()
                    .context("skill UTF-8 path")?
                    .replace('\\', "/");
                let snapshot = project
                    .read_file_snapshot_bounded(&rooted, FILE_CAP.min(remaining))?
                    .context("selected skill disappeared")?;
                remaining = remaining
                    .checked_sub(snapshot.bytes.len())
                    .context("skill byte bound")?;
                files.insert(relative, snapshot);
            }
        }
        super::super::receipt::judge_selection(files.keys().map(String::as_str))
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        Ok(Self {
            root: root.to_owned(),
            correlation,
            source: source.to_owned(),
            include: include.to_vec(),
            files,
        })
    }

    fn revalidate(&self) -> Result<()> {
        let current = Self::observe(&self.root, &self.source, &self.include)
            .map_err(|_| SourceCaptureStale)?;
        anyhow::ensure!(
            self.correlation == current.correlation && self.files == current.files,
            SourceCaptureStale
        );
        Ok(())
    }

    fn bytes(&self) -> BTreeMap<String, Vec<u8>> {
        self.files
            .iter()
            .map(|(path, file)| (path.clone(), file.bytes.clone()))
            .collect()
    }
}

/// In-memory retained payload and original source observations. The upper
/// selected-world owner must additionally bind membership/lock continuity.
#[derive(Debug)]
pub struct PreparedProjectSkills {
    bindings: Vec<ProjectSkillBinding>,
    closures: Vec<RawClosure>,
}

impl PreparedProjectSkills {
    pub fn plan(&self) -> Result<Vec<ProjectSkillBinding>> {
        for closure in &self.closures {
            closure.revalidate()?;
        }
        Ok(self.bindings.clone())
    }
}

pub fn prepare_project_skill_capture(
    project: &Path,
    providers: Vec<ProjectSkillProviderInput>,
    offline: bool,
) -> Result<PreparedProjectSkills> {
    prepare_with(project, providers, offline, |provider, name, policy| {
        provider.embedded_root(name, policy)
    })
}

pub(super) fn prepare_with(
    project: &Path,
    providers: Vec<ProjectSkillProviderInput>,
    offline: bool,
    resolve: impl FnMut(&DeclaredSkillProvider, &str, bool) -> Result<PathBuf>,
) -> Result<PreparedProjectSkills> {
    prepare_with_ports(project, providers, offline, resolve, composed_skill_tree)
}
pub(super) fn prepare_with_ports(
    project: &Path,
    providers: Vec<ProjectSkillProviderInput>,
    offline: bool,
    mut resolve: impl FnMut(&DeclaredSkillProvider, &str, bool) -> Result<PathBuf>,
    mut compose: impl FnMut(&str, &str, &BTreeMap<String, Vec<u8>>) -> Result<PathBuf>,
) -> Result<PreparedProjectSkills> {
    let mut closures = Vec::new();
    let mut skills = Vec::new();
    for input in providers {
        let root = input.provider.root();
        closures.push(RawClosure::observe(root, &root.join("vibe.toml"), &[])?);
        for decl in input.declarations {
            skills.push(DeclaredSkill {
                source: root.join(&decl.path),
                source_root: root.to_owned(),
                origin: input.provider.identity(),
                provider: input.provider.clone(),
                decl,
            });
        }
    }
    let bindings = binding::lower_skills_with(project, skills, Some(offline), |skill, policy| {
        let original = skill.clone();
        let mut roots = BTreeMap::new();
        hydrate_declared_skill_using(
            skill,
            policy,
            |provider, name, policy| {
                let root = resolve(provider, name, policy)?;
                roots.insert(name.to_owned(), root.clone());
                Ok(root)
            },
            &mut compose,
        )?;
        let source_root = match &original.decl.source {
            Some(name) => roots.get(name).context("prepared embedded root")?,
            None => &original.source_root,
        };
        let source = RawClosure::observe(
            source_root,
            &source_root.join(&original.decl.path),
            &original.decl.include,
        )?;
        let mut desired = source.bytes();
        closures.push(source);
        for resource in &original.decl.resources {
            let root = roots
                .get(&resource.embedded_source)
                .context("prepared resource root")?;
            let capture = RawClosure::observe(root, &root.join(&resource.path), &resource.include)?;
            for (path, bytes) in capture.bytes() {
                let path = format!(
                    "{}/{}",
                    resource.target.to_string_lossy().replace('\\', "/"),
                    path
                );
                anyhow::ensure!(desired.insert(path, bytes).is_none(), "resource collision");
            }
            closures.push(capture);
        }
        let composed = RawClosure::observe(&skill.source_root, &skill.source, &skill.decl.include)?;
        anyhow::ensure!(composed.bytes() == desired, SourceCaptureStale);
        closures.push(composed);
        Ok(())
    })?;
    for binding in &bindings {
        let observed = closures
            .iter()
            .rev()
            .find(|closure| {
                closure.root == binding.skill.source_root && closure.source == binding.skill.source
            })
            .context("prepared binding observation")?;
        anyhow::ensure!(
            binding.selected_files.as_ref() == Some(&observed.bytes()),
            SourceCaptureStale
        );
    }
    let capture = PreparedProjectSkills { bindings, closures };
    capture.plan()?;
    Ok(capture)
}
