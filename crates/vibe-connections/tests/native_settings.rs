//! Native adapter interface smoke test; no ordinary user configuration.
use vibe_connections::{NativeSettingsAdapter, PiSettingsAdapter};

#[test]
fn pi_native_adapter_admits_only_its_verified_protocol() {
    let adapter = PiSettingsAdapter;
    assert_eq!(adapter.id(), "pi");
    assert!(adapter.supports("openai-chat-completions"));
    assert!(!adapter.supports("anthropic-messages"));
    assert!(!adapter.supports("openai-responses"));
}
