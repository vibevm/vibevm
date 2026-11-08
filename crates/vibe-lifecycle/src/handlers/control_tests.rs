use super::*;

#[test]
fn controlled_native_and_agent_refuse_before_provider_execution() {
    let root = tempfile::tempdir().unwrap();
    let process = crate::process::ControlledProcessRunner::new(
        vibe_core::cancellation::CancellationToken::new(),
    );
    let runtime = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &crate::NoAgentBackend,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    for handler in [
        ExtensionHandler::Native {
            crate_dir: Some("native".into()),
            prebuilt: None,
        },
        ExtensionHandler::Agent {
            prompt: "spec://org.example/pkg/prompt#root".into(),
        },
    ] {
        let declaration = row(root.path(), handler);
        let refusal = runtime.admit_control(&declaration).unwrap_err();
        assert!(refusal.contains("no cooperative stop capability"));
        assert!(refusal.contains("no provider call started"));
    }
}
