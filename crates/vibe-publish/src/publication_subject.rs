//! Pure validation and identity for one prepared distribution subject.
//!
//! This cell consumes already captured values. It cannot read a source file,
//! call a release host or grant a publication effect.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PUBLICATION-EXACT-SUBJECT");

use sha2::{Digest, Sha256};
use vibe_wire::generated::publication_subject::{
    PublicationAsset, PublicationComponentName, PublicationInstallerKind,
    PublicationSmokeProbeStatus, PublicationSubject,
};

use crate::release_manifest::{
    DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME, DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    DISTRIBUTION_BASH_INSTALLER_FILENAME, DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
    DISTRIBUTION_BUNDLE_MAX_BYTES, DISTRIBUTION_CMD_INSTALLER_FILENAME,
    DISTRIBUTION_COMPONENT_MAX_BYTES, DISTRIBUTION_MANIFEST_MAX_BYTES,
    DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME, DISTRIBUTION_PRODUCT, DISTRIBUTION_REPOSITORY,
    DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
    SUPPORTED_DISTRIBUTION_TARGETS,
};

/// The owner-selected identity, not values inferred from a request body.
pub struct PublicationSubjectExpectations<'a> {
    pub version: &'a str,
    pub repository: &'a str,
    pub smoke_policy_id: &'a str,
    pub smoke_policy_epoch: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PublicationSubjectError {
    #[error("publication subject `{field}` is invalid: {reason}")]
    Invalid { field: String, reason: String },
}

fn invalid(field: impl Into<String>, reason: impl Into<String>) -> PublicationSubjectError {
    PublicationSubjectError::Invalid {
        field: field.into(),
        reason: reason.into(),
    }
}

fn nonblank(field: &str, value: &str) -> Result<(), PublicationSubjectError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(invalid(field, "blank, padded or control-bearing text"));
    }
    Ok(())
}

const SMOKE_EVIDENCE_REF_MAX_BYTES: usize = 512;

fn nonexecution_evidence(field: &str, value: &str) -> Result<(), PublicationSubjectError> {
    nonblank(field, value)?;
    if value.len() > SMOKE_EVIDENCE_REF_MAX_BYTES {
        return Err(invalid(
            field,
            "non-execution evidence reference exceeds 512 bytes",
        ));
    }
    let Some((reason, host)) = value
        .strip_prefix("reason=")
        .and_then(|rest| rest.split_once(";host="))
    else {
        return Err(invalid(
            field,
            "non-execution evidence must identify reason and host",
        ));
    };
    if reason.is_empty() || host.is_empty() || reason.contains(';') || host.contains(';') {
        return Err(invalid(
            field,
            "non-execution evidence must identify reason and host",
        ));
    }
    Ok(())
}

fn git_oid(field: &str, value: &str) -> Result<(), PublicationSubjectError> {
    if !matches!(value.len(), 40 | 64) || !lower_hex(value) {
        return Err(invalid(field, "expected a full lowercase Git object ID"));
    }
    Ok(())
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn digest(field: &str, value: &str) -> Result<(), PublicationSubjectError> {
    if !value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && lower_hex(hex))
    {
        return Err(invalid(field, "expected sha256:<64 lowercase hex>"));
    }
    Ok(())
}

fn byte_count(field: &str, value: &str, max: u64) -> Result<u64, PublicationSubjectError> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| invalid(field, "expected canonical decimal u64 byte count"))?;
    if parsed == 0 || parsed > max || parsed.to_string() != value {
        return Err(invalid(
            field,
            "byte count is zero, over limit or noncanonical",
        ));
    }
    Ok(parsed)
}

fn asset(
    field: &str,
    value: &PublicationAsset,
    name: &str,
    media: &str,
    max: u64,
) -> Result<(), PublicationSubjectError> {
    if value.name != name || value.media_type != media {
        return Err(invalid(
            field,
            "asset name or media type differs from its fixed role",
        ));
    }
    byte_count(&format!("{field}.size_bytes"), &value.size_bytes, max)?;
    digest(&format!("{field}.sha256"), &value.sha256)
}

/// Structural JTD decoding is only the first gate. This pure semantic gate
/// checks identity and evidence coherence while retaining all four typed
/// smoke outcomes, including a skipped or unavailable probe. Apply readiness
/// is a separate decision below.
pub fn validate_publication_subject(
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
) -> Result<(), PublicationSubjectError> {
    if subject.schema != 1 {
        return Err(invalid("schema", "only epoch one is supported"));
    }
    if expected.version != "1.1.0"
        || subject.version != expected.version
        || !subject.mutable
        || subject.product != DISTRIBUTION_PRODUCT
        || expected.repository != DISTRIBUTION_REPOSITORY
        || subject.repository != expected.repository
    {
        return Err(invalid(
            "identity",
            "current mutable 1.1.0 product/repository identity differs from owner selection",
        ));
    }
    git_oid("source.commit", &subject.source.commit)?;
    git_oid("source.tree", &subject.source.tree)?;
    asset(
        "source.cargo_lock",
        &subject.source.cargo_lock,
        "Cargo.lock",
        "text/plain",
        u64::MAX,
    )?;
    if subject.targets.len() != SUPPORTED_DISTRIBUTION_TARGETS.len() {
        return Err(invalid(
            "targets",
            "the complete five-target set is required",
        ));
    }
    let mut first_archive: Option<(&str, &str)> = None;
    for (index, (row, selected)) in subject
        .targets
        .iter()
        .zip(SUPPORTED_DISTRIBUTION_TARGETS)
        .enumerate()
    {
        let field = format!("targets[{index}]");
        if row.target != selected {
            return Err(invalid(
                &field,
                "target is absent, duplicate or out of canonical order",
            ));
        }
        let windows = selected == "x86_64-pc-windows-msvc";
        let suffix = if windows { ".exe" } else { "" };
        asset(
            &format!("{field}.bundle"),
            &row.bundle,
            &format!("vibevm-{}-{selected}.zip", subject.version),
            "application/zip",
            DISTRIBUTION_BUNDLE_MAX_BYTES,
        )?;
        asset(
            &format!("{field}.bootstrap"),
            &row.bootstrap,
            &format!("vibe-bootstrap-{selected}{suffix}"),
            "application/octet-stream",
            DISTRIBUTION_BOOTSTRAP_MAX_BYTES,
        )?;
        asset(
            &format!("{field}.fragment"),
            &row.fragment,
            &format!("vibevm-{}-{selected}.fragment.json", subject.version),
            "application/json",
            DISTRIBUTION_MANIFEST_MAX_BYTES,
        )?;
        if row.components.len() != 2
            || row.components[0].name != PublicationComponentName::Vibe
            || row.components[1].name != PublicationComponentName::VibeIndex
        {
            return Err(invalid(
                format!("{field}.components"),
                "expected ordered vibe and vibe-index",
            ));
        }
        for (component_index, component) in row.components.iter().enumerate() {
            let name = if component_index == 0 {
                "vibe"
            } else {
                "vibe-index"
            };
            if component.path != format!("{name}{suffix}") {
                return Err(invalid(
                    format!("{field}.components[{component_index}].path"),
                    "unexpected component path",
                ));
            }
            byte_count(
                &format!("{field}.components[{component_index}].size_bytes"),
                &component.size_bytes,
                DISTRIBUTION_COMPONENT_MAX_BYTES,
            )?;
            digest(
                &format!("{field}.components[{component_index}].sha256"),
                &component.sha256,
            )?;
        }
        if row.bootstrap.size_bytes != row.components[0].size_bytes
            || row.bootstrap.sha256 != row.components[0].sha256
        {
            return Err(invalid(
                format!("{field}.bootstrap"),
                "raw bootstrap differs from the vibe component",
            ));
        }
        let archive = &row.source_archive;
        if archive.path != DISTRIBUTION_SOURCE_ARCHIVE_FILENAME {
            return Err(invalid(
                format!("{field}.source_archive.path"),
                "unexpected source archive path",
            ));
        }
        byte_count(
            &format!("{field}.source_archive.size_bytes"),
            &archive.size_bytes,
            DISTRIBUTION_SOURCE_ARCHIVE_MAX_BYTES,
        )?;
        digest(&format!("{field}.source_archive.sha256"), &archive.sha256)?;
        if archive.tree_oid != subject.source.tree {
            return Err(invalid(
                format!("{field}.source_archive.tree_oid"),
                "archive tree differs from committed source tree",
            ));
        }
        if let Some((size, sha)) = first_archive {
            if archive.size_bytes != size || archive.sha256 != sha {
                return Err(invalid(
                    format!("{field}.source_archive"),
                    "platforms disagree about the pinned source archive",
                ));
            }
        } else {
            first_archive = Some((&archive.size_bytes, &archive.sha256));
        }
    }
    asset(
        "aggregate",
        &subject.aggregate,
        DISTRIBUTION_AGGREGATE_MANIFEST_FILENAME,
        "application/json",
        DISTRIBUTION_AGGREGATE_MANIFEST_MAX_BYTES,
    )?;
    let installers = [
        (
            PublicationInstallerKind::Bash,
            DISTRIBUTION_BASH_INSTALLER_FILENAME,
            "text/x-shellscript",
        ),
        (
            PublicationInstallerKind::Powershell,
            DISTRIBUTION_POWERSHELL_INSTALLER_FILENAME,
            "text/plain",
        ),
        (
            PublicationInstallerKind::Cmd,
            DISTRIBUTION_CMD_INSTALLER_FILENAME,
            "text/plain",
        ),
    ];
    if subject.installers.len() != installers.len() {
        return Err(invalid(
            "installers",
            "exactly three installer assets are required",
        ));
    }
    for (index, (row, (kind, name, media))) in subject.installers.iter().zip(installers).enumerate()
    {
        if row.kind != kind {
            return Err(invalid(
                format!("installers[{index}].kind"),
                "installer order or kind differs",
            ));
        }
        asset(
            &format!("installers[{index}].asset"),
            &row.asset,
            name,
            media,
            u64::MAX,
        )?;
    }
    if expected.smoke_policy_id != "distribution-smoke/e1"
        || expected.smoke_policy_epoch != 1
        || subject.smoke.policy_id != expected.smoke_policy_id
        || subject.smoke.policy_epoch != expected.smoke_policy_epoch
        || subject.smoke.probes.len() != subject.targets.len()
    {
        return Err(invalid(
            "smoke",
            "the owner-selected finite distribution-smoke/e1 policy differs",
        ));
    }
    for (index, (probe, target)) in subject
        .smoke
        .probes
        .iter()
        .zip(&subject.targets)
        .enumerate()
    {
        if probe.probe_id != "launch" || probe.target != target.target {
            return Err(invalid(
                format!("smoke.probes[{index}]"),
                "expected one launch probe per canonical target",
            ));
        }
        if let Some(tested) = probe.tested_sha256.as_deref() {
            digest(&format!("smoke.probes[{index}].tested_sha256"), tested)?;
            if tested != target.bundle.sha256 {
                return Err(invalid(
                    format!("smoke.probes[{index}].tested_sha256"),
                    "probe evidence names bytes other than this target's bundle",
                ));
            }
        }
        if let Some(runner) = probe.runner_ref.as_deref() {
            nonblank(&format!("smoke.probes[{index}].runner_ref"), runner)?;
        }
        match &probe.status {
            PublicationSmokeProbeStatus::Passed | PublicationSmokeProbeStatus::Failed => {
                let Some(evidence) = probe.evidence_ref.as_deref() else {
                    return Err(invalid(
                        format!("smoke.probes[{index}]"),
                        "an executed probe lacks exact bundle, runner or evidence witness",
                    ));
                };
                nonblank(&format!("smoke.probes[{index}].evidence_ref"), evidence)?;
                if probe.tested_sha256.is_none() || probe.runner_ref.is_none() {
                    return Err(invalid(
                        format!("smoke.probes[{index}]"),
                        "an executed probe lacks exact bundle, runner or evidence witness",
                    ));
                }
            }
            PublicationSmokeProbeStatus::Skipped | PublicationSmokeProbeStatus::Unavailable => {
                if probe.tested_sha256.is_some() || probe.runner_ref.is_some() {
                    return Err(invalid(
                        format!("smoke.probes[{index}]"),
                        "a probe that did not execute cannot claim tested bytes or a runner",
                    ));
                }
                let evidence = probe.evidence_ref.as_deref().ok_or_else(|| {
                    invalid(
                        format!("smoke.probes[{index}].evidence_ref"),
                        "non-execution evidence must identify reason and host",
                    )
                })?;
                nonexecution_evidence(&format!("smoke.probes[{index}].evidence_ref"), evidence)?;
            }
        }
    }
    Ok(())
}

/// The default versioned smoke rule admits only exact passed probes. A
/// skipped probe remains a valid inspectable subject, but an apply decision
/// needs a different explicit future policy/rule ID to permit that skip.
pub fn require_apply_ready_default_smoke(
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
) -> Result<(), PublicationSubjectError> {
    validate_publication_subject(subject, expected)?;
    for (index, probe) in subject.smoke.probes.iter().enumerate() {
        if probe.status != PublicationSmokeProbeStatus::Passed {
            return Err(invalid(
                format!("smoke.probes[{index}].status"),
                "not passed under distribution-smoke/e1; a skipped probe requires a separate explicit versioned policy rule",
            ));
        }
    }
    Ok(())
}

struct Frames(Sha256);

impl Frames {
    fn new() -> Self {
        let mut hash = Sha256::new();
        hash.update(b"vibevm/publication-subject/e1\0");
        Self(hash)
    }

    fn add(&mut self, tag: &str, value: &[u8]) {
        self.0.update(
            u16::try_from(tag.len())
                .expect("fixed frame tag")
                .to_be_bytes(),
        );
        self.0.update(tag.as_bytes());
        self.0.update(
            u64::try_from(value.len())
                .expect("bounded frame value")
                .to_be_bytes(),
        );
        self.0.update(value);
    }

    fn text(&mut self, tag: &str, value: &str) {
        self.add(tag, value.as_bytes());
    }
    fn count(&mut self, tag: &str, value: usize) {
        self.add(
            tag,
            &u64::try_from(value)
                .expect("bounded collection")
                .to_be_bytes(),
        );
    }
    fn integer(&mut self, tag: &str, value: u64) {
        self.add(tag, &value.to_be_bytes());
    }
    fn digest(&mut self, tag: &str, value: &str) {
        self.add(
            tag,
            &hex_bytes(value.strip_prefix("sha256:").expect("validated digest")),
        );
    }
    fn oid(&mut self, tag: &str, value: &str) {
        self.add(tag, &hex_bytes(value));
    }
    fn optional(&mut self, tag: &str, value: Option<&str>) {
        self.add(&format!("{tag}.present"), &[u8::from(value.is_some())]);
        if let Some(value) = value {
            self.text(tag, value);
        }
    }

    fn optional_digest(&mut self, tag: &str, value: Option<&str>) {
        self.add(&format!("{tag}.present"), &[u8::from(value.is_some())]);
        if let Some(value) = value {
            self.digest(tag, value);
        }
    }
    fn asset(&mut self, tag: &str, value: &PublicationAsset) {
        self.text(&format!("{tag}.name"), &value.name);
        self.text(&format!("{tag}.media_type"), &value.media_type);
        self.integer(
            &format!("{tag}.size_bytes"),
            value.size_bytes.parse().expect("validated size"),
        );
        self.digest(&format!("{tag}.sha256"), &value.sha256);
    }
    fn finish(self) -> String {
        format!("sha256:{:x}", self.0.finalize())
    }
}

fn hex_bytes(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            fn nibble(value: u8) -> u8 {
                if value.is_ascii_digit() {
                    value - b'0'
                } else {
                    value - b'a' + 10
                }
            }
            nibble(pair[0]) << 4 | nibble(pair[1])
        })
        .collect()
}

/// SHA-256 over the validated subject's complete canonical frame sequence.
/// No path, network client, credential or filesystem capability enters here.
pub fn publication_subject_digest(
    subject: &PublicationSubject,
    expected: &PublicationSubjectExpectations<'_>,
) -> Result<String, PublicationSubjectError> {
    validate_publication_subject(subject, expected)?;
    let mut hash = Frames::new();
    hash.add("schema", &subject.schema.to_be_bytes());
    hash.text("product", &subject.product);
    hash.text("repository", &subject.repository);
    hash.text("version", &subject.version);
    hash.add("mutable", &[u8::from(subject.mutable)]);
    hash.oid("source.commit", &subject.source.commit);
    hash.oid("source.tree", &subject.source.tree);
    hash.asset("source.cargo_lock", &subject.source.cargo_lock);
    hash.count("targets.count", subject.targets.len());
    for target in &subject.targets {
        hash.text("target", &target.target);
        hash.asset("target.bundle", &target.bundle);
        hash.asset("target.bootstrap", &target.bootstrap);
        hash.asset("target.fragment", &target.fragment);
        hash.count("target.components.count", target.components.len());
        for component in &target.components {
            hash.text(
                "component.name",
                match &component.name {
                    PublicationComponentName::Vibe => "vibe",
                    PublicationComponentName::VibeIndex => "vibe-index",
                },
            );
            hash.text("component.path", &component.path);
            hash.integer(
                "component.size_bytes",
                component.size_bytes.parse().expect("validated size"),
            );
            hash.digest("component.sha256", &component.sha256);
        }
        hash.text("source_archive.path", &target.source_archive.path);
        hash.integer(
            "source_archive.size_bytes",
            target
                .source_archive
                .size_bytes
                .parse()
                .expect("validated size"),
        );
        hash.digest("source_archive.sha256", &target.source_archive.sha256);
        hash.oid("source_archive.tree_oid", &target.source_archive.tree_oid);
    }
    hash.asset("aggregate", &subject.aggregate);
    hash.count("installers.count", subject.installers.len());
    for installer in &subject.installers {
        hash.text(
            "installer.kind",
            match &installer.kind {
                PublicationInstallerKind::Bash => "bash",
                PublicationInstallerKind::Powershell => "powershell",
                PublicationInstallerKind::Cmd => "cmd",
            },
        );
        hash.asset("installer.asset", &installer.asset);
    }
    hash.text("smoke.policy_id", &subject.smoke.policy_id);
    hash.add(
        "smoke.policy_epoch",
        &subject.smoke.policy_epoch.to_be_bytes(),
    );
    hash.count("smoke.probes.count", subject.smoke.probes.len());
    for probe in &subject.smoke.probes {
        hash.text("probe.id", &probe.probe_id);
        hash.text("probe.target", &probe.target);
        hash.text(
            "probe.status",
            match &probe.status {
                PublicationSmokeProbeStatus::Passed => "passed",
                PublicationSmokeProbeStatus::Failed => "failed",
                PublicationSmokeProbeStatus::Skipped => "skipped",
                PublicationSmokeProbeStatus::Unavailable => "unavailable",
            },
        );
        hash.optional_digest("probe.tested_sha256", probe.tested_sha256.as_deref());
        hash.optional("probe.runner_ref", probe.runner_ref.as_deref());
        hash.optional("probe.evidence_ref", probe.evidence_ref.as_deref());
    }
    Ok(hash.finish())
}

#[cfg(test)]
#[path = "publication_subject/tests.rs"]
mod tests;
