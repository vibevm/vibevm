//! Generated local operator reports; no machine query/effect authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD-PREPARATION");
use std::collections::BTreeSet;
pub(super) use vibe_wire::generated::change_scaffold_cli_report::e1::report::*;
pub(super) fn describe(
    output: &vibe_requirements::change::ScaffoldOutput,
) -> anyhow::Result<Prepared> {
    let files = output
        .files
        .iter()
        .map(|(name, bytes)| Ok((name.clone(), std::str::from_utf8(bytes)?.to_owned())))
        .collect::<anyhow::Result<_>>()?;
    let origins = output
        .analysis
        .manifest
        .origins()
        .iter()
        .map(|origin| {
            Ok(Origin {
                package: origin.package.clone(),
                selected_content_hash: origin
                    .selected_content_hash
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("unbound scaffold output"))?,
                manifest_sha256: origin.manifest_sha256.clone(),
                documents: origin
                    .documents
                    .iter()
                    .map(|row| OriginDocument {
                        address: row.address.clone(),
                        sha256: row.sha256.clone(),
                    })
                    .collect(),
            })
        })
        .collect::<anyhow::Result<_>>()?;
    Ok(Prepared {
        id: output.analysis.manifest.id().as_str().into(),
        directory: output
            .directory
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("scaffold output path encoding"))?
            .into(),
        capture: output.capture_id.clone(),
        filter: output.filter_id.clone(),
        selected_addresses: output.selected_addresses.clone(),
        files,
        origins,
        source_strengths: output
            .source_strengths
            .iter()
            .map(|(path, grade)| {
                Ok((
                    path.clone(),
                    match grade.as_str() {
                        "observed" => SourceStrength::Observed,
                        "record-consistent" => SourceStrength::RecordConsistent,
                        "source-proven" => SourceStrength::SourceProven,
                        _ => anyhow::bail!("unsupported retained source grade"),
                    },
                ))
            })
            .collect::<anyhow::Result<_>>()?,
        strength: "retained incumbent grades; memory-held; not authentication".into(),
    })
}
pub(super) fn applied(prepared: Prepared, outcome: super::ScaffoldOutcome) -> Report {
    let value = Outcome {
        state: match outcome.state {
            super::ScaffoldState::Completed => OutcomeState::Completed,
            super::ScaffoldState::Partial => OutcomeState::Partial,
        },
        id: outcome.id,
        journal: outcome.journal,
        created_directories: outcome.created_directories,
        directory_uncertain: outcome.directory_uncertain,
        uncertain_directories: outcome.uncertain_directories,
        verified: outcome.verified,
        uncertain: outcome.uncertain,
        pending: outcome.pending,
        reason: outcome.reason,
        durability_reduced: outcome.durability_reduced,
        infrastructure_may_exist: outcome.infrastructure_may_exist,
    };
    match value.state {
        OutcomeState::Completed => Report::Completed(Box::new(ReportCompleted {
            schema: 1,
            prepared,
            outcome: value,
        })),
        OutcomeState::Partial => Report::Partial(Box::new(ReportPartial {
            schema: 1,
            prepared,
            outcome: value,
        })),
    }
}
pub(super) fn refused(error: &anyhow::Error, apply: bool) -> Report {
    use super::ScaffoldRefusal as E;
    use vibe_orchestrator::document_analysis::edit::EditError as D;
    let code = match error.downcast_ref::<E>() {
        Some(E::Occupied) => ReportRefusedCode::Occupied,
        Some(E::Origin) => ReportRefusedCode::Origin,
        Some(E::Invalid) => ReportRefusedCode::Invalid,
        Some(E::Infrastructure(_)) => ReportRefusedCode::Infrastructure,
        Some(E::ThirdState) => ReportRefusedCode::ThirdState,
        Some(E::Journal) => ReportRefusedCode::Journal,
        Some(E::Edit(D::SaveRequired)) => ReportRefusedCode::SaveRequired,
        Some(E::Edit(D::Stale)) => ReportRefusedCode::Stale,
        Some(E::Edit(D::AuthorityRefused)) => ReportRefusedCode::AuthorityRefused,
        Some(E::Edit(_)) => ReportRefusedCode::Coordination,
        None if error
            .downcast_ref::<vibe_core::manifest::change::ChangeError>()
            .is_some() =>
        {
            ReportRefusedCode::Invalid
        }
        None => ReportRefusedCode::Infrastructure,
    };
    Report::Refused(Box::new(ReportRefused {
        schema: 1,
        code,
        message: error.to_string(),
        infrastructure_may_exist: apply,
    }))
}
pub(super) fn bytes(report: &Report) -> anyhow::Result<Vec<u8>> {
    validate(report)?;
    let bytes = serde_json::to_vec(report)?;
    decode(&bytes)?;
    Ok(bytes)
}
pub(super) fn decode(raw: &[u8]) -> anyhow::Result<Report> {
    anyhow::ensure!(raw.len() <= 32 * 1024 * 1024, "scaffold report byte bound");
    let report: Report = serde_json::from_slice(raw)?;
    validate(&report)?;
    Ok(report)
}
fn validate(report: &Report) -> anyhow::Result<()> {
    let (schema, prepared, outcome, completed) = match report {
        Report::Prepared(r) => (r.schema, Some(&r.prepared), None, false),
        Report::Completed(r) => (r.schema, Some(&r.prepared), Some(&r.outcome), true),
        Report::Partial(r) => (r.schema, Some(&r.prepared), Some(&r.outcome), false),
        Report::Refused(r) => {
            anyhow::ensure!(
                r.schema == 1 && !r.message.trim().is_empty(),
                "scaffold refused epoch/diagnostic"
            );
            return Ok(());
        }
    };
    anyhow::ensure!(schema == 1, "scaffold report epoch");
    let p = prepared.unwrap();
    vibe_core::manifest::change::ChangeId::parse(&p.id)?;
    vibe_safefs::split_relative(&p.directory)?;
    anyhow::ensure!(
        p.directory.ends_with(&format!("/changes/{}", p.id)),
        "scaffold directory identity"
    );
    anyhow::ensure!(
        !p.capture.is_empty()
            && !p.filter.is_empty()
            && p.origins.len() == 1
            && !p.selected_addresses.is_empty()
            && !p.source_strengths.is_empty(),
        "scaffold retained identity"
    );
    anyhow::ensure!(
        p.files.keys().map(String::as_str).collect::<Vec<_>>()
            == ["acceptance.md", "change.toml", "intent.md", "refinement.md"],
        "scaffold role inventory"
    );
    let origin = &p.origins[0];
    let pin = vibe_core::PackageRef::parse(&origin.package)?;
    anyhow::ensure!(
        pin.version.is_exact_pin() && pin.kind.is_some() && pin.group.is_some(),
        "scaffold exact origin pin"
    );
    vibe_core::ContentHash::parse(&origin.selected_content_hash)?;
    vibe_core::ContentHash::parse(&origin.manifest_sha256)?;
    let selected: BTreeSet<_> = p.selected_addresses.iter().collect();
    let cited: BTreeSet<_> = origin.documents.iter().map(|d| &d.address).collect();
    anyhow::ensure!(
        selected.len() == p.selected_addresses.len()
            && cited == selected
            && cited.len() == origin.documents.len(),
        "scaffold origin/selection inventory"
    );
    for d in &origin.documents {
        vibe_core::ContentHash::parse(&d.sha256)?;
    }
    if let Some(o) = outcome {
        anyhow::ensure!(
            o.id.len() == 64
                && o.id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && o.journal == format!(".vibe/change-scaffolds/{}.toml", o.id),
            "scaffold journal identity"
        );
        let expected: BTreeSet<_> = p
            .files
            .keys()
            .map(|n| format!("{}/{n}", p.directory))
            .collect();
        let actual: Vec<_> = o
            .verified
            .iter()
            .chain(&o.uncertain)
            .chain(&o.pending)
            .cloned()
            .collect();
        let set: BTreeSet<_> = actual.iter().cloned().collect();
        anyhow::ensure!(
            set == expected && actual.len() == expected.len(),
            "scaffold outcome partition"
        );
        if completed {
            anyhow::ensure!(
                o.state == OutcomeState::Completed
                    && o.reason.is_none()
                    && o.pending.is_empty()
                    && o.uncertain.is_empty()
                    && !o.directory_uncertain
                    && o.uncertain_directories.is_empty()
                    && o.created_directories.contains(&p.directory),
                "scaffold completed truth"
            );
        } else {
            anyhow::ensure!(
                o.state == OutcomeState::Partial
                    && o.reason.as_ref().is_some_and(|s| !s.is_empty()),
                "scaffold partial truth"
            );
        }
        for path in o.created_directories.iter().chain(&o.uncertain_directories) {
            vibe_safefs::split_relative(path)?;
            anyhow::ensure!(
                std::path::Path::new(&p.directory).starts_with(path),
                "scaffold directory scope"
            );
        }
    }
    Ok(())
}
