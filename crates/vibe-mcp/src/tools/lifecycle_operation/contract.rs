//! MCP request encoding and generated publication never grant authority.
use serde::Serialize;
use serde_json::Value;
use std::io::Write;
pub use vibe_orchestrator::machine_operation::fixed_lifecycle::{PUBLICATION_CAP, REQUEST_CAP};
pub use vibe_wire::generated::machine::e1::lifecycle_action_error::LifecycleActionError;
#[cfg(test)]
pub use vibe_wire::generated::machine::e1::lifecycle_action_error::LifecycleActionErrorEffectObservation;
pub fn size<T: Serialize>(value: &T, cap: usize) -> Result<usize, String> {
    struct Counter {
        count: usize,
        cap: usize,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.count = self
                .count
                .checked_add(bytes.len())
                .filter(|n| *n <= self.cap)
                .ok_or_else(|| std::io::Error::other("frame cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { count: 0, cap };
    serde_json::to_writer(&mut counter, value)
        .map_err(|_| "serialized frame exceeds cap".to_owned())?;
    Ok(counter.count)
}
pub fn encode<T: Serialize>(v: &T, cap: usize) -> Result<String, String> {
    let n = size(v, cap)?;
    let mut out = Vec::with_capacity(n);
    serde_json::to_writer(&mut out, v).map_err(|e| e.to_string())?;
    String::from_utf8(out).map_err(|e| e.to_string())
}
pub fn value<T: Serialize>(v: &T) -> Result<Value, String> {
    let bytes = encode(v, PUBLICATION_CAP)?;
    serde_json::from_str(&bytes).map_err(|e| e.to_string())
}
