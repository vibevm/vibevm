//! Exact, source-supplied realization evidence. This pure owner hashes typed
//! selections and captured bytes; it does not authenticate where a caller
//! obtained those bytes or grant a target write.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY");

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use specmark::spec;

use super::*;
use crate::PackageRef;
use crate::manifest::TargetOs;

const LOCK_DOMAIN: &[u8] = b"vibevm/environment-lock/e1";
const GENERATION_DOMAIN: &[u8] = b"vibevm/environment-generation/e1";

/// One file of an owner-supplied complete shippable-tree inventory. This
/// pure cell checks recipe-1 bytes against the selected content hash, but
/// cannot prove that an external owner omitted no files from this input.
pub struct CapturedFile<'a> {
    pub relative_path: String,
    pub bytes: &'a [u8],
}

/// Captured tree input from the selected Vibe package owner. The source
/// owner must prove completeness and authenticity before supplying it.
pub struct PackageCapture<'a> {
    pub capture: &'a Reference,
    pub owner: &'a Reference,
    pub files: Vec<CapturedFile<'a>>,
}

/// Exact captured lock/closure bytes of a native or external backend.
pub struct ExternalCapture<'a> {
    pub backend: &'a Reference,
    pub closure: &'a Reference,
    pub owner: &'a Reference,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactLockDigest(String);
impl ExactLockDigest {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactGenerationDigest(String);
impl ExactGenerationDigest {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

struct Frames(Sha256);
impl Frames {
    fn new(domain: &[u8]) -> Self {
        let mut hash = Sha256::new();
        hash.update(domain);
        Self(hash)
    }
    fn bytes(&mut self, tag: &'static str, value: &[u8]) {
        self.0
            .update(u16::try_from(tag.len()).unwrap().to_be_bytes());
        self.0.update(tag.as_bytes());
        self.0
            .update(u64::try_from(value.len()).unwrap().to_be_bytes());
        self.0.update(value);
    }
    fn text(&mut self, tag: &'static str, value: &str) -> Result<(), EnvironmentError> {
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
            return Err(invalid(
                "exact.text",
                "identity text must be nonblank canonical UTF8",
            ));
        }
        self.bytes(tag, value.as_bytes());
        Ok(())
    }
    fn count(&mut self, tag: &'static str, count: usize) {
        self.bytes(tag, &u64::try_from(count).unwrap().to_be_bytes());
    }
    fn presence(&mut self, tag: &'static str, present: bool) {
        self.bytes(tag, &[u8::from(present)]);
    }
    fn finish(self) -> String {
        format!("sha256:{:x}", self.0.finalize())
    }
}

fn reference(
    hash: &mut Frames,
    tag: &'static str,
    value: &Reference,
) -> Result<(), EnvironmentError> {
    hash.text(tag, value.as_str())
}

fn package_ref(hash: &mut Frames, package: &PackageRef) -> Result<(), EnvironmentError> {
    hash.presence("package.kind.present", package.kind.is_some());
    if let Some(kind) = package.kind {
        hash.text("package.kind", kind.as_str())?;
    }
    hash.presence("package.group.present", package.group.is_some());
    if let Some(group) = &package.group {
        hash.text("package.group", group.as_str())?;
    }
    hash.text("package.name", package.name.as_str())?;
    match &package.version {
        crate::VersionSpec::Latest => hash.text("package.version.kind", "latest")?,
        crate::VersionSpec::Req(req) => {
            hash.text("package.version.kind", "requirement")?;
            hash.text("package.version.value", &req.to_string())?;
        }
    }
    Ok(())
}

fn os_name(os: TargetOs) -> &'static str {
    match os {
        TargetOs::Windows => "windows",
        TargetOs::Linux => "linux",
        TargetOs::Macos => "macos",
    }
}
fn requirement<T>(
    hash: &mut Frames,
    tag: &'static str,
    value: &Requirement<T>,
    exact: impl Fn(&T) -> &str,
) -> Result<(), EnvironmentError> {
    match value {
        Requirement::Any => hash.text(tag, "any"),
        Requirement::NotApplicable => hash.text(tag, "not_applicable"),
        Requirement::Exact(value) => {
            hash.text(tag, "exact")?;
            hash.text("requirement.value", exact(value))
        }
    }
}
fn constraints(hash: &mut Frames, value: &PlatformConstraints) -> Result<(), EnvironmentError> {
    requirement(hash, "target.os", value.os_requirement(), |os| os_name(*os))?;
    requirement(
        hash,
        "target.arch",
        value.architecture_requirement(),
        Reference::as_str,
    )?;
    requirement(
        hash,
        "target.abi",
        value.abi_requirement(),
        Reference::as_str,
    )?;
    requirement(
        hash,
        "target.libc",
        value.libc_requirement(),
        Reference::as_str,
    )
}
fn fact<T>(
    hash: &mut Frames,
    tag: &'static str,
    value: &Fact<T>,
    exact: impl Fn(&T) -> &str,
) -> Result<(), EnvironmentError> {
    match value {
        Fact::Unknown => hash.text(tag, "unknown"),
        Fact::NotApplicable => hash.text(tag, "not_applicable"),
        Fact::Known(value) => {
            hash.text(tag, "known")?;
            hash.text("fact.value", exact(value))
        }
    }
}
fn platform(hash: &mut Frames, value: &PlatformFacts) -> Result<(), EnvironmentError> {
    if matches!(value.os(), Fact::Unknown)
        || matches!(value.architecture(), Fact::Unknown)
        || matches!(value.abi(), Fact::Unknown)
        || matches!(value.libc(), Fact::Unknown)
    {
        return Err(invalid(
            "exact.platform",
            "required runtime platform fact is unknown",
        ));
    }
    fact(hash, "platform.os", value.os(), |os| os_name(*os))?;
    fact(
        hash,
        "platform.arch",
        value.architecture(),
        Reference::as_str,
    )?;
    fact(hash, "platform.abi", value.abi(), Reference::as_str)?;
    fact(hash, "platform.libc", value.libc(), Reference::as_str)
}
fn role(role: DependencyRole) -> &'static str {
    match role {
        DependencyRole::Build => "build",
        DependencyRole::Runtime => "runtime",
        DependencyRole::BuildAndRuntime => "build_and_runtime",
    }
}
fn tree_recipe_one(files: &[CapturedFile<'_>]) -> Result<String, EnvironmentError> {
    let mut ordered = BTreeMap::new();
    for file in files {
        let normalized = file.relative_path.replace('\\', "/");
        if normalized.is_empty()
            || normalized
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || normalized.chars().any(char::is_control)
            || ordered.insert(normalized, file.bytes).is_some()
        {
            return Err(invalid(
                "exact.capture",
                "invalid or colliding captured relative file path",
            ));
        }
    }
    let mut hash = Sha256::new();
    for (path, bytes) in ordered {
        hash.update(path.as_bytes());
        hash.update([0]);
        hash.update(bytes);
        hash.update([0]);
    }
    Ok(format!("sha256-tree/1:{:x}", hash.finalize()))
}

/// Hash a complete typed definition and exact selected closure. All package
/// and external members require captured-byte evidence; URL/version text is
/// never evidence. `EnvironmentLock::id` remains an A.1 opaque reference.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY",
    r = 1
)]
pub fn exact_lock_digest(
    definition: &EnvironmentDefinition,
    lock: &EnvironmentLock,
    packages: &[PackageCapture<'_>],
    external: &[ExternalCapture<'_>],
) -> Result<ExactLockDigest, EnvironmentError> {
    if lock.closure().definition != *definition.id()
        || lock.closure().target_constraints != definition.composition().target_constraints
    {
        return Err(invalid(
            "exact.lock",
            "definition and selected lock disagree",
        ));
    }
    let mut captures = BTreeMap::new();
    for entry in packages {
        if captures.insert(entry.capture.as_str(), entry).is_some() {
            return Err(invalid(
                "exact.packages",
                "duplicate captured package identity",
            ));
        }
    }
    let mut closures = BTreeMap::new();
    for entry in external {
        if closures
            .insert((entry.backend.as_str(), entry.closure.as_str()), entry)
            .is_some()
        {
            return Err(invalid(
                "exact.external",
                "duplicate captured external closure",
            ));
        }
    }
    if captures.len() != lock.closure().packages.len()
        || closures.len() != lock.closure().external_backends.len()
    {
        return Err(invalid(
            "exact.closure",
            "required captured closure member is missing or extra",
        ));
    }
    let mut hash = Frames::new(LOCK_DOMAIN);
    reference(
        &mut hash,
        "definition.reference",
        definition.id().reference(),
    )?;
    reference(&mut hash, "definition.producer", definition.id().producer())?;
    let composition = definition.composition();
    let mut roots = composition.roots.iter().collect::<Vec<_>>();
    roots.sort_by_key(|row| row.name.as_str());
    hash.count("definition.roots.count", roots.len());
    for row in roots {
        reference(&mut hash, "root.name", &row.name)?;
        package_ref(&mut hash, &row.package)?;
    }
    let mut features = composition.features.iter().collect::<Vec<_>>();
    features.sort_by_key(|value| value.as_str());
    hash.count("definition.features.count", features.len());
    for value in features {
        reference(&mut hash, "feature", value)?;
    }
    let mut choices = composition.choices.iter().collect::<Vec<_>>();
    choices.sort_by_key(|row| row.name.as_str());
    hash.count("definition.choices.count", choices.len());
    for row in choices {
        reference(&mut hash, "choice.name", &row.name)?;
        reference(&mut hash, "choice.value", &row.value)?;
    }
    constraints(&mut hash, &composition.target_constraints)?;
    let mut bindings = composition.runtime_bindings.iter().collect::<Vec<_>>();
    bindings.sort_by_key(|row| row.name.as_str());
    hash.count("definition.runtime.count", bindings.len());
    for row in bindings {
        reference(&mut hash, "runtime.name", &row.name)?;
        platform(&mut hash, &row.platform)?;
    }
    for (tag, exports) in [
        ("definition.tools.count", &composition.tool_exports),
        ("definition.services.count", &composition.service_exports),
    ] {
        let mut exports = exports.iter().collect::<Vec<_>>();
        exports.sort_by_key(|row| row.name.as_str());
        hash.count(tag, exports.len());
        for row in exports {
            reference(&mut hash, "export.name", &row.name)?;
            reference(&mut hash, "export.package_root", &row.package_root)?;
            reference(&mut hash, "export.runtime_binding", &row.runtime_binding)?;
        }
    }
    let mut selected = lock.closure().packages.iter().collect::<Vec<_>>();
    selected.sort_by_key(|row| (role(row.facts().role), row.facts().capture.as_str()));
    let mut semantic = BTreeSet::new();
    hash.count("lock.packages.count", selected.len());
    for row in selected {
        let row = row.facts();
        if !semantic.insert((
            row.package.group.as_ref().map(|g| g.as_str()),
            row.package.name.as_str(),
            row.version.to_string(),
        )) {
            return Err(invalid(
                "exact.packages",
                "duplicate selected semantic package",
            ));
        }
        let evidence = captures
            .get(row.capture.as_str())
            .ok_or_else(|| invalid("exact.packages", "selected package lacks captured bytes"))?;
        let observed_tree = tree_recipe_one(&evidence.files)?;
        if row.content.as_str() != observed_tree {
            return Err(invalid(
                "exact.packages",
                "selected content hash disagrees with captured recipe-1 tree or uses unsupported recipe",
            ));
        }
        hash.presence("selected.root.present", row.root.is_some());
        if let Some(root) = &row.root {
            reference(&mut hash, "selected.root", root)?;
        }
        package_ref(&mut hash, &row.package)?;
        hash.text("selected.version", &row.version.to_string())?;
        hash.text("selected.content", row.content.as_str())?;
        reference(&mut hash, "selected.capture", &row.capture)?;
        reference(&mut hash, "selected.configuration", &row.configuration)?;
        hash.text("selected.role", role(row.role))?;
        reference(&mut hash, "selected.capture.owner", evidence.owner)?;
        hash.text("selected.capture.tree", &observed_tree)?;
    }
    let mut members = lock.closure().external_backends.iter().collect::<Vec<_>>();
    members.sort_by_key(|row| (role(row.role), row.backend.as_str(), row.closure.as_str()));
    hash.count("lock.external.count", members.len());
    for row in members {
        let evidence = closures
            .get(&(row.backend.as_str(), row.closure.as_str()))
            .ok_or_else(|| invalid("exact.external", "external closure lacks captured bytes"))?;
        reference(&mut hash, "external.backend", &row.backend)?;
        reference(&mut hash, "external.closure", &row.closure)?;
        reference(&mut hash, "external.configuration", &row.configuration)?;
        hash.text("external.role", role(row.role))?;
        reference(&mut hash, "external.capture.owner", evidence.owner)?;
        hash.bytes(
            "external.capture.bytes_sha256",
            &Sha256::digest(evidence.bytes),
        );
    }
    Ok(ExactLockDigest(hash.finish()))
}

/// Target-owned metadata. Linux numeric IDs are legal only for a Linux
/// target; Windows ACL identity is deliberately unavailable in this atom.
pub enum PayloadOwnership {
    Linux { uid: u32, gid: u32 },
    Macos { uid: u32, gid: u32 },
    UnsupportedWindowsAcl,
}
pub enum PayloadKind<'a> {
    File { bytes: &'a [u8] },
    Directory,
    Link { target: &'a [u8], verified: bool },
    UnsupportedSpecial,
}
pub struct PayloadEntry<'a> {
    pub path: &'a TargetRelativePath,
    pub kind: PayloadKind<'a>,
    pub mode: u32,
    pub owner: PayloadOwnership,
    pub producer: ClosureMember,
}
pub struct GeneratedOutput<'a> {
    pub role: &'static str,
    pub name: &'a Reference,
    pub bytes: &'a [u8],
}
pub struct ExactGenerationTarget<'a> {
    pub mode: TargetMode,
    pub platform: &'a PlatformFacts,
    pub path_policy: TargetPathPolicy,
    pub comparison: CaseComparison,
}

fn target_mode(mode: TargetMode) -> &'static str {
    match mode {
        TargetMode::WorkspaceProjection => "workspace_projection",
        TargetMode::UserProfile => "user_profile",
        TargetMode::OfflineLinuxRootfs => "offline_linux_rootfs",
        TargetMode::LiveLinuxSystem => "live_linux_system",
    }
}
fn member_key(member: &ClosureMember) -> (String, String) {
    match member {
        ClosureMember::PackageCapture(capture) => ("package".into(), capture.as_str().into()),
        ClosureMember::External { backend, closure } => (
            format!("external:{}", backend.as_str()),
            closure.as_str().into(),
        ),
    }
}
fn payload_key(
    path: &TargetRelativePath,
    model: CaseComparison,
) -> Result<Vec<Vec<u8>>, EnvironmentError> {
    if path.components().is_empty() {
        return Err(invalid(
            "generation.path",
            "payload requires a relative component",
        ));
    }
    path.key(model).ok_or_else(|| {
        invalid(
            "generation.path",
            "target collision comparison is unavailable",
        )
    })
}
fn known_target(target: &ExactGenerationTarget<'_>) -> Result<TargetOs, EnvironmentError> {
    let Fact::Known(os) = target.platform.os() else {
        return Err(invalid(
            "generation.target",
            "target OS is unknown or inapplicable",
        ));
    };
    if !matches!(target.platform.architecture(), Fact::Known(_))
        || !matches!(target.platform.abi(), Fact::Known(_))
        || matches!(target.platform.libc(), Fact::Unknown)
    {
        return Err(invalid(
            "generation.target",
            "target architecture, ABI or libc is unknown",
        ));
    }
    match (os, target.path_policy) {
        (TargetOs::Linux, TargetPathPolicy::LinuxBytes)
        | (TargetOs::Macos, TargetPathPolicy::MacosPosixBytes)
        | (TargetOs::Windows, TargetPathPolicy::WindowsPortableWin32Utf8) => {}
        _ => {
            return Err(invalid(
                "generation.target",
                "target OS and path policy disagree or are unsupported",
            ));
        }
    }
    if target.comparison == CaseComparison::Unknown {
        return Err(invalid(
            "generation.target",
            "target collision policy is unknown",
        ));
    }
    Ok(*os)
}

/// Hash produced bytes independently of the recipe lock. It refuses incomplete
/// closure, unsupported metadata and lexical path collisions. This pure
/// receipt is evidence, never activation or physical-target authority.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-GENERATION-IDENTITY",
    r = 1
)]
pub fn exact_generation_digest(
    lock: &EnvironmentLock,
    exact_lock: &ExactLockDigest,
    target: &ExactGenerationTarget<'_>,
    dependency_closure: &[ClosureMember],
    payloads: &[PayloadEntry<'_>],
    outputs: &[GeneratedOutput<'_>],
) -> Result<ExactGenerationDigest, EnvironmentError> {
    let os = known_target(target)?;
    if !lock
        .closure()
        .target_constraints
        .compare(target.platform)
        .is_satisfied()
    {
        return Err(invalid(
            "generation.target",
            "target platform violates selected lock constraints",
        ));
    }
    let members = lock.members();
    let observed = dependency_closure.iter().cloned().collect::<BTreeSet<_>>();
    if observed != members || observed.len() != dependency_closure.len() {
        return Err(invalid(
            "generation.closure",
            "realized dependency closure is incomplete or duplicated",
        ));
    }
    let mut ordered = BTreeMap::<Vec<Vec<u8>>, &PayloadEntry<'_>>::new();
    for entry in payloads {
        if entry.path.policy() != target.path_policy || !members.contains(&entry.producer) {
            return Err(invalid(
                "generation.payload",
                "path policy or producer is outside selected target/closure",
            ));
        }
        let key = payload_key(entry.path, target.comparison)?;
        if ordered.insert(key, entry).is_some() {
            return Err(invalid(
                "generation.payload",
                "target lexical path collision",
            ));
        }
        if entry.mode & !0o7777 != 0 || entry.mode & 0o6000 != 0 {
            return Err(invalid(
                "generation.payload",
                "unsupported privileged or unknown mode bits",
            ));
        }
        match (&entry.owner, os) {
            (PayloadOwnership::Linux { .. }, TargetOs::Linux)
            | (PayloadOwnership::Macos { .. }, TargetOs::Macos) => {}
            (PayloadOwnership::UnsupportedWindowsAcl, TargetOs::Windows) => {
                return Err(invalid(
                    "generation.payload",
                    "Windows ACL metadata is unsupported",
                ));
            }
            _ => {
                return Err(invalid(
                    "generation.payload",
                    "ownership metadata belongs to another target OS",
                ));
            }
        }
        match &entry.kind {
            PayloadKind::File { .. } | PayloadKind::Directory => {}
            PayloadKind::Link { target, verified } if *verified && !target.is_empty() => {}
            PayloadKind::Link { .. } => {
                return Err(invalid(
                    "generation.payload",
                    "link target is unverified or empty",
                ));
            }
            PayloadKind::UnsupportedSpecial => {
                return Err(invalid("generation.payload", "special node is unsupported"));
            }
        }
    }
    let mut hash = Frames::new(GENERATION_DOMAIN);
    hash.text("generation.lock", exact_lock.as_str())?;
    hash.bytes("generation.recipe_epoch", &1u64.to_be_bytes());
    hash.bytes("generation.schema_epoch", &1u64.to_be_bytes());
    hash.text("generation.target_role", target_mode(target.mode))?;
    platform(&mut hash, target.platform)?;
    hash.text(
        "generation.path_policy",
        match target.path_policy {
            TargetPathPolicy::LinuxBytes => "linux_bytes",
            TargetPathPolicy::MacosPosixBytes => "macos_posix_bytes",
            TargetPathPolicy::WindowsPortableWin32Utf8 => "windows_portable_win32_utf8",
        },
    )?;
    hash.text(
        "generation.case",
        match target.comparison {
            CaseComparison::ByteSensitive => "byte_sensitive",
            CaseComparison::AsciiInsensitive => "ascii_insensitive",
            CaseComparison::Unknown => unreachable!("refused above"),
        },
    )?;
    hash.count("generation.closure.count", observed.len());
    for member in observed {
        let (role, id) = member_key(&member);
        hash.text("closure.role", &role)?;
        hash.text("closure.identity", &id)?;
    }
    hash.count("generation.payloads.count", ordered.len());
    for (_, entry) in ordered {
        hash.count("payload.path.count", entry.path.components().len());
        for component in entry.path.components() {
            hash.bytes("payload.path.component", component.bytes());
        }
        match &entry.kind {
            PayloadKind::File { bytes } => {
                hash.text("payload.kind", "file")?;
                hash.bytes("payload.content_sha256", &Sha256::digest(bytes));
            }
            PayloadKind::Directory => hash.text("payload.kind", "directory")?,
            PayloadKind::Link { target, .. } => {
                hash.text("payload.kind", "link")?;
                hash.bytes("payload.link_target", target);
            }
            PayloadKind::UnsupportedSpecial => unreachable!("refused above"),
        }
        hash.bytes("payload.mode", &u64::from(entry.mode).to_be_bytes());
        if let PayloadOwnership::Linux { uid, gid } = entry.owner {
            hash.bytes("payload.linux_uid", &u64::from(uid).to_be_bytes());
            hash.bytes("payload.linux_gid", &u64::from(gid).to_be_bytes());
        } else if let PayloadOwnership::Macos { uid, gid } = entry.owner {
            hash.bytes("payload.macos_uid", &u64::from(uid).to_be_bytes());
            hash.bytes("payload.macos_gid", &u64::from(gid).to_be_bytes());
        }
        let (role, id) = member_key(&entry.producer);
        hash.text("payload.producer.role", &role)?;
        hash.text("payload.producer.id", &id)?;
    }
    let mut outputs = outputs.iter().collect::<Vec<_>>();
    outputs.sort_by_key(|row| (row.role, row.name.as_str()));
    let mut output_keys = BTreeSet::new();
    hash.count("generation.outputs.count", outputs.len());
    for output in outputs {
        if !matches!(output.role, "boot" | "adapter" | "service")
            || !output_keys.insert((output.role, output.name.as_str()))
        {
            return Err(invalid(
                "generation.outputs",
                "unknown or duplicate generated output",
            ));
        }
        hash.text("output.role", output.role)?;
        reference(&mut hash, "output.name", output.name)?;
        hash.bytes("output.bytes_sha256", &Sha256::digest(output.bytes));
    }
    Ok(ExactGenerationDigest(hash.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentHash, PackageRef};

    fn r(value: &str) -> Reference {
        Reference::new(value).unwrap()
    }
    fn constraints(abi: &str) -> PlatformConstraints {
        PlatformConstraints::new(
            Requirement::Exact(TargetOs::Linux),
            Requirement::Exact(r("x86_64")),
            Requirement::Exact(r(abi)),
            Requirement::Exact(r("glibc")),
        )
    }
    fn definition(reverse: bool, abi: &str) -> EnvironmentDefinition {
        let mut roots = vec![
            DesiredRoot {
                name: r("a"),
                package: PackageRef::parse("flow:org.example/a@^1").unwrap(),
            },
            DesiredRoot {
                name: r("b"),
                package: PackageRef::parse("flow:org.example/b@^1").unwrap(),
            },
        ];
        if reverse {
            roots.reverse();
        }
        EnvironmentDefinition::new(
            DefinitionId::new("definition", "author").unwrap(),
            DefinitionComposition {
                roots,
                features: vec![r("z"), r("a")],
                choices: vec![],
                target_constraints: constraints(abi),
                runtime_bindings: vec![],
                tool_exports: vec![],
                service_exports: vec![],
            },
        )
        .unwrap()
    }
    fn tree(bytes: &[u8]) -> String {
        tree_recipe_one(&[CapturedFile {
            relative_path: "payload.bin".into(),
            bytes,
        }])
        .unwrap()
    }
    fn lock(
        definition: &EnvironmentDefinition,
        reverse: bool,
        a: &[u8],
        b: &[u8],
    ) -> EnvironmentLock {
        let mut packages = vec!["a", "b"]
            .into_iter()
            .map(|name| {
                SelectedPackage::new(SelectedPackageFacts {
                    root: Some(r(name)),
                    package: PackageRef::parse(&format!("flow:org.example/{name}@=1.1.0")).unwrap(),
                    version: semver::Version::new(1, 1, 0),
                    content: ContentHash::parse(&tree(if name == "a" { a } else { b })).unwrap(),
                    capture: r(&format!("capture-{name}")),
                    configuration: r("config"),
                    role: DependencyRole::BuildAndRuntime,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();
        if reverse {
            packages.reverse();
        }
        EnvironmentLock::new(
            LockId::new("opaque-lock", "resolver").unwrap(),
            definition,
            LockClosure {
                definition: definition.id().clone(),
                packages,
                external_backends: vec![],
                target_constraints: definition.composition().target_constraints.clone(),
            },
        )
        .unwrap()
    }
    fn evidence<'a>(
        lock: &'a EnvironmentLock,
        owner: &'a Reference,
        a: &'a [u8],
        b: &'a [u8],
    ) -> Vec<PackageCapture<'a>> {
        lock.closure()
            .packages
            .iter()
            .map(|package| {
                let capture = &package.facts().capture;
                PackageCapture {
                    capture,
                    owner,
                    files: vec![CapturedFile {
                        relative_path: "payload.bin".into(),
                        bytes: if capture.as_str() == "capture-a" {
                            a
                        } else {
                            b
                        },
                    }],
                }
            })
            .collect()
    }
    fn digest(
        definition: &EnvironmentDefinition,
        lock: &EnvironmentLock,
        a: &[u8],
        b: &[u8],
    ) -> ExactLockDigest {
        let owner = r("capture-owner");
        exact_lock_digest(definition, lock, &evidence(lock, &owner, a, b), &[]).unwrap()
    }
    fn linux() -> PlatformFacts {
        PlatformFacts::new(
            Fact::Known(TargetOs::Linux),
            Fact::Known(r("x86_64")),
            Fact::Known(r("gnu")),
            Fact::Known(r("glibc")),
        )
    }

    #[test]
    fn reordered_equivalent_definition_and_lock_have_one_exact_digest() {
        let a = definition(false, "gnu");
        let b = definition(true, "gnu");
        let la = lock(&a, false, b"one", b"two");
        let lb = lock(&b, true, b"one", b"two");
        assert_eq!(
            digest(&a, &la, b"one", b"two"),
            digest(&b, &lb, b"one", b"two")
        );
        let changed = definition(false, "musl");
        let lc = lock(&changed, false, b"one", b"two");
        assert_ne!(
            digest(&a, &la, b"one", b"two"),
            digest(&changed, &lc, b"one", b"two")
        );
        assert!(
            exact_lock_digest(
                &a,
                &la,
                &evidence(&la, &r("owner"), b"changed", b"two"),
                &[]
            )
            .is_err()
        );
        let changed_bytes = lock(&a, false, b"changed", b"two");
        assert_ne!(
            digest(&a, &la, b"one", b"two"),
            digest(&a, &changed_bytes, b"changed", b"two")
        );
    }

    #[test]
    fn missing_or_extra_capture_refuses_exact_lock() {
        let d = definition(false, "gnu");
        let l = lock(&d, false, b"a", b"b");
        let owner = r("capture-owner");
        let captures = evidence(&l, &owner, b"a", b"b");
        assert!(exact_lock_digest(&d, &l, &captures[..1], &[]).is_err());
        let extra = r("unknown");
        let mut captures = captures;
        captures.push(PackageCapture {
            capture: &extra,
            owner: &owner,
            files: vec![CapturedFile {
                relative_path: "payload.bin".into(),
                bytes: b"x",
            }],
        });
        assert!(exact_lock_digest(&d, &l, &captures, &[]).is_err());
        let mut legacy_closure = l.closure().clone();
        let mut legacy_facts = legacy_closure.packages[0].facts().clone();
        legacy_facts.content = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        legacy_closure.packages[0] = SelectedPackage::new(legacy_facts).unwrap();
        let legacy = EnvironmentLock::new(
            LockId::new("opaque-lock", "resolver").unwrap(),
            &d,
            legacy_closure,
        )
        .unwrap();
        assert!(
            exact_lock_digest(&d, &legacy, &evidence(&legacy, &owner, b"a", b"b"), &[]).is_err()
        );
    }

    #[test]
    fn payload_bytes_and_target_metadata_change_generation_digest() {
        let d = definition(false, "gnu");
        let l = lock(&d, false, b"one", b"two");
        let exact = digest(&d, &l, b"one", b"two");
        let platform = linux();
        let target = ExactGenerationTarget {
            mode: TargetMode::OfflineLinuxRootfs,
            platform: &platform,
            path_policy: TargetPathPolicy::LinuxBytes,
            comparison: CaseComparison::ByteSensitive,
        };
        let path = TargetRelativePath::new(
            TargetPathPolicy::LinuxBytes,
            vec![b"usr".to_vec(), b"bin".to_vec()],
        )
        .unwrap();
        let closure = l.members().into_iter().collect::<Vec<_>>();
        let entry = |bytes: &'static [u8], mode| PayloadEntry {
            path: &path,
            kind: PayloadKind::File { bytes },
            mode,
            owner: PayloadOwnership::Linux { uid: 0, gid: 0 },
            producer: ClosureMember::PackageCapture(r("capture-a")),
        };
        let first =
            exact_generation_digest(&l, &exact, &target, &closure, &[entry(b"a", 0o755)], &[])
                .unwrap();
        assert_ne!(
            first,
            exact_generation_digest(&l, &exact, &target, &closure, &[entry(b"b", 0o755)], &[])
                .unwrap()
        );
        assert_ne!(
            first,
            exact_generation_digest(&l, &exact, &target, &closure, &[entry(b"a", 0o644)], &[])
                .unwrap()
        );
        let musl = PlatformFacts::new(
            Fact::Known(TargetOs::Linux),
            Fact::Known(r("x86_64")),
            Fact::Known(r("musl")),
            Fact::Known(r("glibc")),
        );
        let changed_target = ExactGenerationTarget {
            platform: &musl,
            ..target
        };
        assert!(
            exact_generation_digest(
                &l,
                &exact,
                &changed_target,
                &closure,
                &[entry(b"a", 0o755)],
                &[]
            )
            .is_err()
        );
        let second_path =
            TargetRelativePath::new(TargetPathPolicy::LinuxBytes, vec![b"etc".to_vec()]).unwrap();
        let directory = || PayloadEntry {
            path: &second_path,
            kind: PayloadKind::Directory,
            mode: 0o755,
            owner: PayloadOwnership::Linux { uid: 0, gid: 0 },
            producer: ClosureMember::PackageCapture(r("capture-b")),
        };
        assert_eq!(
            exact_generation_digest(
                &l,
                &exact,
                &target,
                &closure,
                &[entry(b"a", 0o755), directory()],
                &[]
            )
            .unwrap(),
            exact_generation_digest(
                &l,
                &exact,
                &target,
                &closure,
                &[directory(), entry(b"a", 0o755)],
                &[]
            )
            .unwrap(),
        );
    }

    #[test]
    fn wrong_target_privileged_and_incomplete_payload_evidence_refuse() {
        let d = definition(false, "gnu");
        let l = lock(&d, false, b"one", b"two");
        let exact = digest(&d, &l, b"one", b"two");
        let windows = PlatformFacts::new(
            Fact::Known(TargetOs::Windows),
            Fact::Known(r("x86_64")),
            Fact::Known(r("msvc")),
            Fact::NotApplicable,
        );
        let target = ExactGenerationTarget {
            mode: TargetMode::WorkspaceProjection,
            platform: &windows,
            path_policy: TargetPathPolicy::WindowsPortableWin32Utf8,
            comparison: CaseComparison::AsciiInsensitive,
        };
        let path = TargetRelativePath::new(
            TargetPathPolicy::WindowsPortableWin32Utf8,
            vec![b"tool".to_vec()],
        )
        .unwrap();
        let closure = l.members().into_iter().collect::<Vec<_>>();
        let row = PayloadEntry {
            path: &path,
            kind: PayloadKind::File { bytes: b"a" },
            mode: 0o755,
            owner: PayloadOwnership::Linux { uid: 1, gid: 1 },
            producer: ClosureMember::PackageCapture(r("capture-a")),
        };
        assert!(exact_generation_digest(&l, &exact, &target, &closure, &[row], &[]).is_err());
        assert!(exact_generation_digest(&l, &exact, &target, &closure[..1], &[], &[]).is_err());
        let linux = linux();
        let target = ExactGenerationTarget {
            mode: TargetMode::OfflineLinuxRootfs,
            platform: &linux,
            path_policy: TargetPathPolicy::LinuxBytes,
            comparison: CaseComparison::ByteSensitive,
        };
        let linux_path =
            TargetRelativePath::new(TargetPathPolicy::LinuxBytes, vec![b"tool".to_vec()]).unwrap();
        for kind in [
            PayloadKind::UnsupportedSpecial,
            PayloadKind::Link {
                target: b"missing",
                verified: false,
            },
        ] {
            let row = PayloadEntry {
                path: &linux_path,
                kind,
                mode: 0o755,
                owner: PayloadOwnership::Linux { uid: 0, gid: 0 },
                producer: ClosureMember::PackageCapture(r("capture-a")),
            };
            assert!(exact_generation_digest(&l, &exact, &target, &closure, &[row], &[]).is_err());
        }
    }

    #[test]
    fn macos_generation_uses_distinct_posix_policy_and_principal_tags() {
        let baseline = definition(false, "darwin");
        let mut composition = baseline.composition().clone();
        composition.target_constraints = PlatformConstraints::new(
            Requirement::Exact(TargetOs::Macos),
            Requirement::Exact(r("aarch64")),
            Requirement::Exact(r("darwin")),
            Requirement::NotApplicable,
        );
        let definition = EnvironmentDefinition::new(baseline.id().clone(), composition).unwrap();
        let lock = lock(&definition, false, b"one", b"two");
        let exact = digest(&definition, &lock, b"one", b"two");
        let platform = PlatformFacts::new(
            Fact::Known(TargetOs::Macos),
            Fact::Known(r("aarch64")),
            Fact::Known(r("darwin")),
            Fact::NotApplicable,
        );
        let target = ExactGenerationTarget {
            mode: TargetMode::UserProfile,
            platform: &platform,
            path_policy: TargetPathPolicy::MacosPosixBytes,
            comparison: CaseComparison::ByteSensitive,
        };
        let path = TargetRelativePath::new(
            TargetPathPolicy::MacosPosixBytes,
            vec![b"Library".to_vec(), b"tool".to_vec()],
        )
        .unwrap();
        let row = PayloadEntry {
            path: &path,
            kind: PayloadKind::File { bytes: b"mac" },
            mode: 0o755,
            owner: PayloadOwnership::Macos { uid: 501, gid: 20 },
            producer: ClosureMember::PackageCapture(r("capture-a")),
        };
        let closure = lock.members().into_iter().collect::<Vec<_>>();
        assert!(exact_generation_digest(&lock, &exact, &target, &closure, &[row], &[]).is_ok());
        let wrong = ExactGenerationTarget {
            path_policy: TargetPathPolicy::LinuxBytes,
            ..target
        };
        assert!(exact_generation_digest(&lock, &exact, &wrong, &closure, &[], &[]).is_err());
    }

    #[test]
    fn windows_target_never_serializes_linux_ids_or_unproven_acl() {
        let baseline = definition(false, "msvc");
        let mut composition = baseline.composition().clone();
        composition.target_constraints = PlatformConstraints::new(
            Requirement::Exact(TargetOs::Windows),
            Requirement::Exact(r("x86_64")),
            Requirement::Exact(r("msvc")),
            Requirement::NotApplicable,
        );
        let definition = EnvironmentDefinition::new(baseline.id().clone(), composition).unwrap();
        let lock = lock(&definition, false, b"one", b"two");
        let exact = digest(&definition, &lock, b"one", b"two");
        let platform = PlatformFacts::new(
            Fact::Known(TargetOs::Windows),
            Fact::Known(r("x86_64")),
            Fact::Known(r("msvc")),
            Fact::NotApplicable,
        );
        let target = ExactGenerationTarget {
            mode: TargetMode::WorkspaceProjection,
            platform: &platform,
            path_policy: TargetPathPolicy::WindowsPortableWin32Utf8,
            comparison: CaseComparison::AsciiInsensitive,
        };
        let path = TargetRelativePath::new(
            TargetPathPolicy::WindowsPortableWin32Utf8,
            vec![b"bin".to_vec()],
        )
        .unwrap();
        let closure = lock.members().into_iter().collect::<Vec<_>>();
        for owner in [
            PayloadOwnership::Linux { uid: 1, gid: 2 },
            PayloadOwnership::UnsupportedWindowsAcl,
        ] {
            let row = PayloadEntry {
                path: &path,
                kind: PayloadKind::File { bytes: b"tool" },
                mode: 0o755,
                owner,
                producer: ClosureMember::PackageCapture(r("capture-a")),
            };
            let error =
                exact_generation_digest(&lock, &exact, &target, &closure, &[row], &[]).unwrap_err();
            assert_eq!(error.field, "generation.payload");
        }
    }
}
