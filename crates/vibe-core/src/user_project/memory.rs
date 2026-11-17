//! Shared memory guidance for classic and request-local personal boot.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-MEMORY-DEFAULT");

/// Instruction delivery is not authority to create records during boot.
/// An authored user snippet may durably choose another memory mechanism.
pub const INSTRUCTIONS: &str = "Personal memory: when authorized to remember a user preference, working style, machine fact or cross-project convention, write an ordinary authored record under the personal project's vibevm/vibespecs, not harness auto-memory. A fact collaborators need belongs in the project's repository. Never store credentials. An explicit durable opt-out in the loaded user snippet selects the user's other memory mechanism; respect it. A record must be boot-indexed or explicitly linked from the authored user boot snippet to reach a later session; arbitrary records are not auto-loaded. Boot never writes records or edits INDEX. An absent/unavailable personal root requires explicit user init/action before a memory write, not silent harness fallback.\n";
