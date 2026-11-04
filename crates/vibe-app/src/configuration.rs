//! Common selected-installer configuration and its generated public vocabulary.
//! Original answers are distinct from effective values and automatic default use.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-BASIS");

mod batch;
mod choices;
mod definition;
mod error;
pub(crate) mod record;
mod review;
pub use batch::classify_shared;
mod describe;
pub use describe::describe;
mod evaluate;
mod input;
mod parameters;
pub(crate) mod plan;
mod vocabulary;
pub use error::ConfigurationError;
pub use evaluate::{EvaluatedConfiguration, EvaluationRequest, evaluate};
pub use input::parse_submission;

pub use vibe_wire::generated::configuration::e1::input::ConfigurationSubmission;
pub use vibe_wire::generated::shared::{ConfigurationEvaluation, ConfigurationRecord};
