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

/// A backend that can close its in-flight request is admitted under a stop;
/// the native handler still is not.
#[test]
fn controlled_agent_is_admitted_only_through_a_stop_capable_backend() {
    struct Supervised;
    impl crate::agent::AgentBackend for Supervised {
        fn resolve_prompt(
            &self,
            _request: &crate::agent::PromptRequest,
        ) -> Result<crate::agent::ResolvedPrompt, String> {
            Err("unused".into())
        }
        fn complete(
            &self,
            _request: &crate::agent::AgentRequest,
        ) -> Result<crate::agent::AgentCompletion, String> {
            Err("unused".into())
        }
        fn supports_control(&self) -> bool {
            true
        }
    }
    let root = tempfile::tempdir().unwrap();
    let token = vibe_core::cancellation::CancellationToken::new();
    let process = crate::process::ControlledProcessRunner::new(token.clone());
    let runtime = HandlerRuntime {
        process: &process,
        binary: &NoBinaryBackend,
        native: &NoNativeBackend,
        package_binding: &NoPackageBindingBackend,
        agent: &Supervised,
        probe: &BashProbe,
        streams: StreamMode::Capture,
    };
    let agent = row(
        root.path(),
        ExtensionHandler::Agent {
            prompt: "spec://org.example/pkg/prompt#root".into(),
        },
    );
    runtime.admit_control(&agent).unwrap();
    let native = row(
        root.path(),
        ExtensionHandler::Native {
            crate_dir: Some("native".into()),
            prebuilt: None,
        },
    );
    assert!(runtime.admit_control(&native).is_err());
    token.cancel();
    assert!(
        runtime.admit_control(&agent).is_err(),
        "a stop still refuses"
    );
}
