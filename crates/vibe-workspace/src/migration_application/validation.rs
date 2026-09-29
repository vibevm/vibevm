//! Generated decoding plus finite operation/journal semantic admission.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLICATION-WIRE"
);

use super::identity::{intent_identity, raw_sha256};
use super::{
    ApplicationValidationError as Fault, OwnedManifestMigrationFile, ValidatedApply,
    ValidatedOperation, absolute_spelling,
};
use crate::migration_planning::{MigrationPlanningResponse, plan_migration_json};
use serde_json::Value;
use vibe_core::manifest::Manifest;
use vibe_wire::generated::migration::e1::{
    application_journal::ApplicationJournal,
    operation_report::{ApplicationErrorCode as Code, OperationReportOperation as Operation},
    operation_request::OperationRequest,
    plan_report::{Outcome, Plan},
};

type Checked<T> = Result<T, Fault>;
fn invalid(code: Code, field: &'static str, reason: &'static str) -> Fault {
    Fault::new(code, field, reason)
}
pub(crate) fn hex64(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn physical_token(text: &str) -> bool {
    text.strip_prefix("sha256:").is_some_and(hex64)
}
fn decimal(text: &str) -> Checked<u64> {
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(invalid(
            Code::JournalInvalid,
            "length",
            "length is not canonical unsigned decimal",
        ));
    }
    text.parse().map_err(|_| {
        invalid(
            Code::JournalInvalid,
            "length",
            "length exceeds unsigned 64-bit range",
        )
    })
}
fn nonblank(text: &str) -> bool {
    !text.trim().is_empty()
}
fn identity(text: &str, field: &'static str, code: Code) -> Checked<()> {
    if hex64(text) {
        Ok(())
    } else {
        Err(invalid(
            code,
            field,
            "identity must be 64 lowercase hexadecimal digits",
        ))
    }
}

fn authority(
    project: &str,
    store: &str,
    target: &str,
    grant: &OwnedManifestMigrationFile,
) -> Checked<()> {
    absolute_spelling(std::path::Path::new(project))?;
    absolute_spelling(std::path::Path::new(store))?;
    vibe_safefs::split_relative(target).map_err(|_| {
        invalid(
            Code::AuthorityRefused,
            "authority.target_relative",
            "target is not portable",
        )
    })?;
    if project != grant.project_spelling
        || store != grant.store_spelling
        || target != grant.target_relative
    {
        return Err(invalid(
            Code::AuthorityRefused,
            "authority",
            "request does not match explicit one-file grant",
        ));
    }
    Ok(())
}

fn replay(text: &str) -> Checked<(Plan, bool)> {
    match plan_migration_json(text) {
        MigrationPlanningResponse::Report(report) => match report.outcome {
            Outcome::Planned(p) => Ok((p.plan, false)),
            Outcome::NoOp(p) => Ok((p.plan, true)),
            _ => Err(invalid(
                Code::PlanRefused,
                "approved_request_json",
                "approved request must replay to planned or no_op",
            )),
        },
        MigrationPlanningResponse::Preflight(_) => Err(invalid(
            Code::InvalidRequest,
            "approved_request_json",
            "approved planning request failed strict preflight",
        )),
    }
}
fn frozen(plan: &Plan, is_no_op: bool) -> Checked<()> {
    let manifest = Manifest::parse_str(&plan.after).map_err(|_| {
        invalid(
            Code::PlanRefused,
            "plan.after",
            "checked output could not be validated",
        )
    })?;
    if !is_no_op
        && manifest
            .require_package()
            .map_err(|_| {
                invalid(
                    Code::PlanRefused,
                    "plan.after",
                    "checked output has no package",
                )
            })?
            .frozen
    {
        return Err(invalid(
            Code::AuthorityRefused,
            "plan.after.package.frozen",
            "effectful application to a frozen coordinate is excluded",
        ));
    }
    Ok(())
}
fn plan_identities(plan: &Plan, before: &str, after: &str, diff: &str) -> Checked<()> {
    for (field, value) in [
        ("before_content_id", before),
        ("after_content_id", after),
        ("diff_id", diff),
    ] {
        identity(value, field, Code::IntentMismatch)?;
    }
    if before != plan.before_identity || after != plan.after_identity || diff != plan.diff_identity
    {
        return Err(invalid(
            Code::IntentMismatch,
            "plan.identities",
            "supplied identities differ from exact planner replay",
        ));
    }
    Ok(())
}

pub fn decode_operation_json(
    text: &str,
    grant: &OwnedManifestMigrationFile,
) -> Checked<ValidatedOperation> {
    let value: Value = serde_json::from_str(text)
        .map_err(|_| invalid(Code::InvalidRequest, "request", "request is not valid JSON"))?;
    // Preserve only recognized identity for truthful malformed preflight reports.
    let operation = value
        .get("operation")
        .and_then(Value::as_str)
        .and_then(|v| match v {
            "apply" => Some(Operation::Apply),
            "inspect" => Some(Operation::Inspect),
            "recover" => Some(Operation::Recover),
            _ => None,
        });
    let id = value
        .get("operation_id")
        .and_then(Value::as_str)
        .filter(|s| nonblank(s))
        .map(str::to_owned);
    let validated = (|| {
        let request: OperationRequest = serde_json::from_value(value).map_err(|_| {
            invalid(
                Code::InvalidRequest,
                "request",
                "request does not match registered shape",
            )
        })?;
        validate_operation(request, grant)
    })();
    validated.map_err(|mut e| {
        e.operation = operation;
        e.operation_id = id;
        e
    })
}

fn validate_operation(
    request: OperationRequest,
    grant: &OwnedManifestMigrationFile,
) -> Checked<ValidatedOperation> {
    let (schema, id, a) = match &request {
        OperationRequest::Apply(r) => (r.schema, &r.operation_id, &r.authority),
        OperationRequest::Inspect(r) => (r.schema, &r.operation_id, &r.authority),
        OperationRequest::Recover(r) => (r.schema, &r.operation_id, &r.authority),
    };
    if schema != 1 {
        return Err(invalid(
            Code::UnsupportedSchema,
            "schema",
            "unsupported operation schema",
        ));
    }
    if !nonblank(id) {
        return Err(invalid(
            Code::InvalidRequest,
            "operation_id",
            "operation id must be nonblank",
        ));
    }
    authority(
        &a.project_root,
        &a.recovery_store_root,
        &a.target_relative,
        grant,
    )?;
    match request {
        OperationRequest::Apply(r) => {
            let (plan, is_no_op) = replay(&r.approved_request_json)?;
            plan_identities(&plan, &r.before_content_id, &r.after_content_id, &r.diff_id)?;
            frozen(&plan, is_no_op)?;
            Ok(ValidatedOperation::Apply(ValidatedApply {
                request: *r,
                plan,
                is_no_op,
            }))
        }
        OperationRequest::Inspect(r) => Ok(ValidatedOperation::Inspect(*r)),
        OperationRequest::Recover(r) => {
            identity(
                &r.expected_intent_id,
                "expected_intent_id",
                Code::IntentMismatch,
            )?;
            Ok(ValidatedOperation::Recover(*r))
        }
    }
}

pub fn decode_journal_json(text: &str, grant: &OwnedManifestMigrationFile) -> Checked<Plan> {
    let journal: ApplicationJournal = serde_json::from_str(text).map_err(|_| {
        invalid(
            Code::JournalInvalid,
            "journal",
            "journal does not match registered shape",
        )
    })?;
    validate_journal(&journal, grant)
}

/// Validates retained logical intent; actual filesystem bindings and before.bin
/// bytes must still be measured by the runtime under the participating barrier.
pub fn validate_journal(
    j: &ApplicationJournal,
    grant: &OwnedManifestMigrationFile,
) -> Checked<Plan> {
    if j.schema != 1 || j.protocol_revision != 1 {
        return Err(invalid(
            Code::JournalInvalid,
            "schema/protocol_revision",
            "unsupported journal schema or protocol",
        ));
    }
    if !nonblank(&j.operation_id) {
        return Err(invalid(
            Code::JournalInvalid,
            "operation_id",
            "operation id must be nonblank",
        ));
    }
    authority(
        &j.authority.project_root,
        &j.authority.recovery_store_root,
        &j.authority.target_relative,
        grant,
    )?;
    if !physical_token(&j.project_binding) || !physical_token(&j.store_binding) {
        return Err(invalid(
            Code::JournalInvalid,
            "bindings",
            "binding is not an opaque safefs identity token",
        ));
    }
    if j.unix_mode.is_some_and(|m| m > 0o7777) {
        return Err(invalid(
            Code::JournalInvalid,
            "unix_mode",
            "mode exceeds safefs permission preservation range",
        ));
    }
    let (plan, no_op) = replay(&j.approved_request_json)?;
    frozen(&plan, no_op)?;
    plan_identities(&plan, &j.before_content_id, &j.after_content_id, &j.diff_id)?;
    identity(&j.intent_id, "intent_id", Code::JournalInvalid)?;
    for (field, value) in [
        ("before_raw_sha256", &j.before_raw_sha256),
        ("after_raw_sha256", &j.after_raw_sha256),
    ] {
        identity(value, field, Code::JournalInvalid)?;
    }
    if decimal(&j.before_length)? != plan.before.len() as u64
        || decimal(&j.after_length)? != plan.after.len() as u64
    {
        return Err(invalid(
            Code::JournalInvalid,
            "length",
            "retained lengths differ from replayed bytes",
        ));
    }
    if raw_sha256(plan.before.as_bytes()) != j.before_raw_sha256
        || raw_sha256(plan.after.as_bytes()) != j.after_raw_sha256
    {
        return Err(invalid(
            Code::JournalInvalid,
            "raw_sha256",
            "raw hashes differ from replayed bytes",
        ));
    }
    if intent_identity(j) != j.intent_id {
        return Err(invalid(
            Code::IntentMismatch,
            "intent_id",
            "immutable intent hash differs",
        ));
    }
    Ok(plan)
}
