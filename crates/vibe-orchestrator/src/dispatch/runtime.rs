//! The same dispatch runtime; invocation bounds missing-tool preparation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#INVOCATION-ROLE-LAW");
use super::ProtectedScriptExecution;
use super::backends::WorkspaceBinaryBackend;
use crate::ports::RunObserver;
use vibe_lifecycle::AgentBackend;
use vibe_lifecycle::handlers::{HandlerRuntime, NativeBackend, PackageBindingBackend};
use vibe_lifecycle::process::{ProcessRunner, SystemProcessRunner};
use vibe_workspace::hooks::InterpreterProbe;

pub(super) fn runtime<'a>(
    observer: &dyn RunObserver,
    package_binding: &'a dyn PackageBindingBackend,
    native: &'a dyn NativeBackend,
    agent: &'a dyn AgentBackend,
    protected: Option<ProtectedScriptExecution<'a>>,
    allow_build: bool,
) -> HandlerRuntime<'a> {
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    static BINARY_INHERIT: WorkspaceBinaryBackend = WorkspaceBinaryBackend {
        quiet: false,
        allow_build: true,
    };
    static BINARY_QUIET: WorkspaceBinaryBackend = WorkspaceBinaryBackend {
        quiet: true,
        allow_build: true,
    };
    static BINARY_READ_INHERIT: WorkspaceBinaryBackend = WorkspaceBinaryBackend {
        quiet: false,
        allow_build: false,
    };
    static BINARY_READ_QUIET: WorkspaceBinaryBackend = WorkspaceBinaryBackend {
        quiet: true,
        allow_build: false,
    };
    static PROBE: vibe_workspace::hooks::SystemProbe = vibe_workspace::hooks::SystemProbe;
    let process: &dyn ProcessRunner = protected.map_or(&PROCESS, |value| value.process);
    let probe: &dyn InterpreterProbe = protected.map_or(&PROBE, |value| value.probe);
    HandlerRuntime {
        process,
        binary: match (allow_build, observer.binary_quiet()) {
            (true, true) => &BINARY_QUIET,
            (true, false) => &BINARY_INHERIT,
            (false, true) => &BINARY_READ_QUIET,
            (false, false) => &BINARY_READ_INHERIT,
        },
        native,
        package_binding,
        agent,
        probe,
        streams: observer.stream_mode(),
        protected_script: protected.map(|value| value.stage),
    }
}
