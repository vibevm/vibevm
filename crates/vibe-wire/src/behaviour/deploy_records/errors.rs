//! The typed refusals of the deploy journal pair's scalar laws. The
//! same refusal discipline the trace cells carry: a journal and a
//! receipt are read from disk, so no variant here clones a wire
//! string — every untrusted scalar rides a bounded [`ScalarPreview`]
//! (shared with the trace index cell, one type, not a second preview),
//! every member name is bounded by construction, and every row refusal
//! names the list position it sat in.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::behaviour::scalars::ProviderKeyDefect;
use crate::generated::deploy_receipt::ReceiptStatus;

/// One broken deploy-intent law, with the context needed to name the
/// offender. Typed end to end — no stringly `detail` — so a test can
/// assert the exact family a mutation lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployIntentError {
    /// `schema` is not this validator's epoch — a newer journal must
    /// fail loudly, not parse into a wrong meaning.
    SchemaEpoch { found: u32 },
    /// `plan_hash` is not exactly 64 lowercase hex.
    PlanHashNotHex { plan_hash: ScalarPreview },
    /// `target.profile` is not a portable token.
    ProfileNotPortableToken { profile: ScalarPreview },
    /// `target.target` is not a portable token.
    TargetNotPortableToken { target: ScalarPreview },
    /// A planned resource's `desired_digest` is not exactly 64
    /// lowercase hex. `row` is the resource's list position.
    DesiredDigestNotHex {
        row: usize,
        desired_digest: ScalarPreview,
    },
    /// A planned resource's optional `prior_digest` is not exactly 64
    /// lowercase hex. `row` is the resource's list position.
    PriorDigestNotHex {
        row: usize,
        prior_digest: ScalarPreview,
    },
    /// A planned resource identity is blank or carries CR, LF or NUL.
    /// `row` is the resource's list position.
    UnsafeResource { row: usize, value: ScalarPreview },
    /// A free-text member is blank or carries CR, LF or NUL. `field`
    /// names the member in wire spelling.
    UnsafeScalar {
        field: &'static str,
        value: ScalarPreview,
    },
}

/// One broken deploy-receipt law, with the context needed to name the
/// offender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployReceiptError {
    /// `schema` is not this validator's epoch.
    SchemaEpoch { found: u32 },
    /// `profile` is not a portable token.
    ProfileNotPortableToken { profile: ScalarPreview },
    /// `target` is not a portable token.
    TargetNotPortableToken { target: ScalarPreview },
    /// `artifact_digest` or `desired_config_digest` is not exactly 64
    /// lowercase hex. `member` names it in wire spelling.
    DigestNotHex {
        member: &'static str,
        value: ScalarPreview,
    },
    /// `provider.key` breaks the ExtensionKey shape.
    BadProviderKey {
        key: ScalarPreview,
        defect: ProviderKeyDefect,
    },
    /// `provider.content_hash` is not `sha256:` + 64 lowercase hex —
    /// the one identity spelling every lockfile row carries.
    BadContentHash { content_hash: ScalarPreview },
    /// An owned resource's `post_digest` is not exactly 64 lowercase
    /// hex. `row` is the resource's list position.
    PostDigestNotHex {
        row: usize,
        post_digest: ScalarPreview,
    },
    /// An owned resource identity is blank or carries CR, LF or NUL.
    /// `row` is the resource's list position.
    UnsafeResource { row: usize, value: ScalarPreview },
    /// A free-text member is blank or carries CR, LF or NUL. `field`
    /// names the member in wire spelling.
    UnsafeScalar {
        field: &'static str,
        value: ScalarPreview,
    },
    /// A terminal status (`verified`, `failed`, `rolled-back`) carries
    /// no `finalized_at`; receipt finalisation is last.
    TerminalNotFinalised { status: ReceiptStatus },
    /// A mid-flight `applied` receipt carries a `finalized_at`; apply
    /// is not finalisation.
    AppliedFinalised,
}

impl std::error::Error for DeployIntentError {}
impl std::error::Error for DeployReceiptError {}

/// The wire spelling of a receipt status — the closed enum carries no
/// `Display` of its own, and a refusal quoting the exact wire word
/// beats one that names the Rust variant.
fn status_spelling(status: &ReceiptStatus) -> &'static str {
    match status {
        ReceiptStatus::Applied => "applied",
        ReceiptStatus::Failed => "failed",
        ReceiptStatus::RolledBack => "rolled-back",
        ReceiptStatus::Verified => "verified",
    }
}

impl std::fmt::Display for DeployIntentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DeployIntentError as E;
        match self {
            E::SchemaEpoch { found } => write!(
                f,
                "schema = {found} is not the deploy-intent epoch {epoch}",
                epoch = super::INTENT_EPOCH
            ),
            E::PlanHashNotHex { plan_hash } => write!(
                f,
                "plan_hash {plan_hash} is not exactly 64 lowercase hex characters"
            ),
            E::ProfileNotPortableToken { profile } => write!(
                f,
                "target.profile {profile} does not obey the portable-token grammar \
                 `[a-z0-9][a-z0-9._-]{{0,63}}`"
            ),
            E::TargetNotPortableToken { target } => write!(
                f,
                "target.target {target} does not obey the portable-token grammar \
                 `[a-z0-9][a-z0-9._-]{{0,63}}`"
            ),
            E::DesiredDigestNotHex {
                row,
                desired_digest,
            } => write!(
                f,
                "resources[{row}].desired_digest {desired_digest} is not exactly 64 lowercase \
                 hex characters"
            ),
            E::PriorDigestNotHex { row, prior_digest } => write!(
                f,
                "resources[{row}].prior_digest {prior_digest} is not exactly 64 lowercase hex \
                 characters"
            ),
            E::UnsafeResource { row, value } => write!(
                f,
                "resources[{row}].resource {value} is empty, whitespace-only or carries CR, LF \
                 or NUL"
            ),
            E::UnsafeScalar { field, value } => write!(
                f,
                "{field} {value} is empty, whitespace-only or carries CR, LF or NUL"
            ),
        }
    }
}

impl std::fmt::Display for DeployReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DeployReceiptError as E;
        match self {
            E::SchemaEpoch { found } => write!(
                f,
                "schema = {found} is not the deploy-receipt epoch {epoch}",
                epoch = super::RECEIPT_EPOCH
            ),
            E::ProfileNotPortableToken { profile } => write!(
                f,
                "profile {profile} does not obey the portable-token grammar \
                 `[a-z0-9][a-z0-9._-]{{0,63}}`"
            ),
            E::TargetNotPortableToken { target } => write!(
                f,
                "target {target} does not obey the portable-token grammar \
                 `[a-z0-9][a-z0-9._-]{{0,63}}`"
            ),
            E::DigestNotHex { member, value } => write!(
                f,
                "{member} {value} is not exactly 64 lowercase hex characters"
            ),
            E::BadProviderKey { key, defect } => write!(
                f,
                "provider.key {key} {} (the spelling is `group/name#id`)",
                defect.phrase()
            ),
            E::BadContentHash { content_hash } => write!(
                f,
                "provider.content_hash {content_hash} is not `sha256:` followed by 64 lowercase \
                 hex"
            ),
            E::PostDigestNotHex { row, post_digest } => write!(
                f,
                "resources[{row}].post_digest {post_digest} is not exactly 64 lowercase hex \
                 characters"
            ),
            E::UnsafeResource { row, value } => write!(
                f,
                "resources[{row}].resource {value} is empty, whitespace-only or carries CR, LF \
                 or NUL"
            ),
            E::UnsafeScalar { field, value } => write!(
                f,
                "{field} {value} is empty, whitespace-only or carries CR, LF or NUL"
            ),
            E::TerminalNotFinalised { status } => write!(
                f,
                "a `{}` receipt carries no finalized_at; receipt finalisation is last",
                status_spelling(status)
            ),
            E::AppliedFinalised => write!(
                f,
                "an `applied` receipt carries a finalized_at; apply is mid-flight, not \
                 finalisation"
            ),
        }
    }
}
