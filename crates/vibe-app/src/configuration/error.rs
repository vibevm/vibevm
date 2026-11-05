//! Structured common-configuration refusal, retaining a safe inspectable evaluation.
#[derive(Debug, thiserror::Error)]
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS"
)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#INPUT-NORMALIZATION")]
pub enum ConfigurationError {
    #[error("import `{operation}` needs fresh scoped secret input for `{key}` (spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT); fix: continue the same import with --secret-input", operation = .target.operation)]
    ImportInputRequired {
        target: vibe_wire::generated::shared::ApplicationImportTarget,
        key: String,
        parameters: Vec<String>,
    },
    #[error(
        "portable format `{format}` cannot represent retained common configuration for {operation} of `{package}` (spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY); use direct configuration input until a typed rich portable format is available"
    )]
    UnsupportedFormat {
        format: &'static str,
        operation: &'static str,
        package: String,
    },
    #[error(
        "common configuration of installer `{option}` is unresolved (spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS); fix: inspect the configuration plan and supply the missing answers or reviewed input"
    )]
    Unresolved {
        option: String,
        evaluation: Box<super::ConfigurationEvaluation>,
    },
    #[error(
        "invalid common configuration input for installer `{option}`: {reason} (spec://org.vibevm.core/vibevm/common/PROP-069#INPUT-NORMALIZATION)"
    )]
    Invalid { option: String, reason: String },
}
