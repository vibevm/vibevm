//! Controlled native invocation through one operation-owned helper process.
//!
//! An uncontrolled operation keeps the process-lifetime in-process loader.
//! A controlled one loads and invokes its images in a helper session that
//! the operation's token owns: one helper per operation, reused across the
//! lifecycle, compiler and mechanism seams, killed and reaped on an
//! acknowledged stop. The parent keeps artifact, source, fact and receipt
//! authority; the helper re-validates each admitted image before loading it.
//! This boundary provides cancellation, not a sandbox.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW");

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use vibe_core::cancellation::{CancellationToken, WeakCancellationToken};
use vibe_native_loader::helper::{
    self as wire, Direction, HELPER_MODE_ARG, HelperChannel, HelperFault, SESSION_BYTES,
};
use vibe_native_loader::{NativeLoadError, NativeLoader};

use crate::process::session::SessionProcess;

const POLL_MIN: Duration = Duration::from_micros(100);
const POLL_MAX: Duration = Duration::from_millis(10);
const CLOSE_GRACE: Duration = Duration::from_secs(2);

/// Refusal to configure the trusted native helper executable.
///
/// ```
/// use vibe_lifecycle::native::{NativeHelperCommand, NativeHelperError};
/// let refused = NativeHelperCommand::trusted("vibe".into()).unwrap_err();
/// assert!(matches!(refused, NativeHelperError::Untrusted { .. }));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW")]
pub enum NativeHelperError {
    #[error(
        "native helper executable `{program}` is not an absolute regular file (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW; fix: supply the running vibe executable resolved by the composition owner)"
    )]
    Untrusted { program: String },
    #[error(
        "a native helper executable is already installed for this process (violates spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW; fix: install the composition owner's helper exactly once at startup)"
    )]
    AlreadyInstalled,
}

/// The trusted helper command supplied by the composition owner.
///
/// It names the running `vibe` executable (or a test harness); it never
/// comes from plugin metadata or an ambient executable lookup. The spawned
/// argv is `program prefix... HELPER_MODE_ARG <scratch> <session>`.
///
/// ```
/// use vibe_lifecycle::native::NativeHelperCommand;
/// assert!(NativeHelperCommand::trusted("relative-vibe".into()).is_err());
/// ```
#[derive(Debug, Clone)]
pub struct NativeHelperCommand {
    program: PathBuf,
    prefix: Vec<OsString>,
}

impl NativeHelperCommand {
    pub fn trusted(program: PathBuf) -> Result<Self, NativeHelperError> {
        if !program.is_absolute() || !program.is_file() {
            return Err(NativeHelperError::Untrusted {
                program: program.display().to_string(),
            });
        }
        Ok(Self {
            program,
            prefix: Vec::new(),
        })
    }

    /// Arguments placed before the helper mode word, for a harness whose own
    /// argv grammar must first route to its helper entry.
    #[must_use]
    pub fn with_prefix_args(mut self, prefix: Vec<OsString>) -> Self {
        self.prefix = prefix;
        self
    }
}

static INSTALLED: OnceLock<NativeHelperCommand> = OnceLock::new();

/// Install the process-wide helper used by every controlled operation that
/// has no explicitly attached one. The composition owner calls this once.
pub fn install_native_helper(command: NativeHelperCommand) -> Result<(), NativeHelperError> {
    INSTALLED
        .set(command)
        .map_err(|_| NativeHelperError::AlreadyInstalled)
}

/// Bind `command` as the helper of the operation `token` controls.
///
/// No process starts until that operation's first native load.
pub fn attach_native_helper(token: &CancellationToken, command: NativeHelperCommand) {
    let _ = token.operation_resource(|| OperationNative::new(command, token.downgrade()));
}

/// `unknown` once this operation's helper may have loaded or invoked
/// plugin code, else `not-started`; process termination never upgrades it.
pub fn native_plugin_effects(token: &CancellationToken) -> &'static str {
    match token.existing_operation_resource::<OperationNative>() {
        Some(native) if native.session.effects.load(Ordering::SeqCst) => "unknown",
        _ => "not-started",
    }
}

/// Whether a controlled operation on `token` can stop its native calls.
pub(crate) fn stop_capable(token: &CancellationToken) -> bool {
    token.has_control()
        && (INSTALLED.get().is_some()
            || token
                .existing_operation_resource::<OperationNative>()
                .is_some())
}

/// The loader one native call uses.
pub(crate) enum NativeRuntime {
    /// Today's in-process process-lifetime loader.
    InProcess(&'static NativeLoader),
    /// The operation's helper-backed loader.
    Helper(Arc<OperationNative>),
}

impl NativeRuntime {
    /// The runtime for a call under `token`; `None` when the operation is
    /// controlled but no helper exists, so the caller decides how to refuse.
    pub(crate) fn select(
        token: &CancellationToken,
        process: &'static NativeLoader,
    ) -> Option<Self> {
        if !token.has_control() {
            return Some(Self::InProcess(process));
        }
        if let Some(native) = token.existing_operation_resource::<OperationNative>() {
            return Some(Self::Helper(native));
        }
        let command = INSTALLED.get()?.clone();
        Some(Self::Helper(token.operation_resource(|| {
            OperationNative::new(command, token.downgrade())
        })))
    }

    /// Record the parent-verified bytes of `image` and return the loader
    /// that may open it.
    pub(crate) fn admit(
        &self,
        image: &Path,
        digest: &str,
        bytes: u64,
    ) -> Result<&NativeLoader, NativeLoadError> {
        match self {
            Self::InProcess(loader) => Ok(loader),
            Self::Helper(native) => {
                native.loader.expect_image(image, digest, bytes)?;
                Ok(&native.loader)
            }
        }
    }
}

/// The running helper's pid and private scratch of `token`'s operation.
#[cfg(test)]
pub(crate) fn running_helper(token: &CancellationToken) -> Option<(u32, PathBuf)> {
    let native = token.existing_operation_resource::<OperationNative>()?;
    match &*native.session.state.lock().ok()? {
        State::Running(running) => Some((running.process.id(), running.scratch.clone())),
        _ => None,
    }
}

/// One operation's helper session and the loader that routes through it.
pub(crate) struct OperationNative {
    session: Arc<HelperSession>,
    loader: NativeLoader,
}

impl OperationNative {
    fn new(command: NativeHelperCommand, control: WeakCancellationToken) -> Self {
        let session = Arc::new(HelperSession {
            command,
            control,
            effects: AtomicBool::new(false),
            state: Mutex::new(State::Idle),
        });
        let loader = NativeLoader::with_helper(Arc::clone(&session) as Arc<dyn HelperChannel>);
        Self { session, loader }
    }
}

struct HelperSession {
    command: NativeHelperCommand,
    control: WeakCancellationToken,
    effects: AtomicBool,
    state: Mutex<State>,
}

enum State {
    Idle,
    Running(Running),
    Closed(String),
}

struct Running {
    process: SessionProcess,
    scratch: PathBuf,
    session: [u8; SESSION_BYTES],
    next: u64,
}

impl HelperSession {
    fn start(&self) -> Result<Running, String> {
        let mut session = [0_u8; SESSION_BYTES];
        getrandom::fill(&mut session)
            .map_err(|error| format!("helper session identity: {error}"))?;
        let scratch = private_scratch(&session)?;
        let mut args = self.command.prefix.clone();
        args.extend([
            OsString::from(HELPER_MODE_ARG),
            scratch.clone().into_os_string(),
            OsString::from(wire::session_hex(&session)),
        ]);
        match SessionProcess::spawn(&self.command.program, &args) {
            Ok(process) => Ok(Running {
                process,
                scratch,
                session,
                next: 1,
            }),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&scratch);
                Err(error.to_string())
            }
        }
    }
}

impl HelperChannel for HelperSession {
    fn exchange(&self, request: Vec<u8>) -> Result<Vec<u8>, HelperFault> {
        let token = self
            .control
            .upgrade()
            .ok_or_else(|| HelperFault::Transport("the owning operation has ended".to_owned()))?;
        token
            .checkpoint()
            .map_err(|_| HelperFault::StoppedBeforeCall)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| HelperFault::Transport("helper session lock is poisoned".to_owned()))?;
        if matches!(*state, State::Idle) {
            *state = State::Running(self.start().map_err(HelperFault::Transport)?);
        }
        let running = match &mut *state {
            State::Running(running) => running,
            State::Closed(reason) => {
                return Err(HelperFault::Transport(format!(
                    "helper session is closed ({reason}); a possibly effectful helper is never restarted"
                )));
            }
            State::Idle => {
                return Err(HelperFault::Transport(
                    "helper session did not start".to_owned(),
                ));
            }
        };
        token
            .checkpoint()
            .map_err(|_| HelperFault::StoppedBeforeCall)?;
        // Loading may run plugin constructors; any later stop reports unknown effects.
        token.mark_unowned_effects();
        self.effects.store(true, Ordering::SeqCst);
        let sequence = running.next;
        running.next += 1;
        let outcome = running.wait(&token, sequence, &request);
        if let Err(fault) = &outcome {
            let reason = match fault {
                HelperFault::Stopped(_) => "stopped".to_owned(),
                HelperFault::StoppedBeforeCall => "stopped before call".to_owned(),
                HelperFault::Transport(reason) => reason.clone(),
            };
            *state = State::Closed(reason);
        }
        outcome
    }
}

impl Running {
    fn wait(
        &mut self,
        token: &CancellationToken,
        sequence: u64,
        request: &[u8],
    ) -> Result<Vec<u8>, HelperFault> {
        if let Err(reason) = wire::publish(
            &self.scratch,
            Direction::Request,
            &self.session,
            sequence,
            request,
        ) {
            return Err(self.fail(reason));
        }
        let mut pause = POLL_MIN;
        loop {
            if token.checkpoint().is_err() {
                let reaped = self.process.kill_and_reap();
                let removed = std::fs::remove_dir_all(&self.scratch);
                return Err(HelperFault::Stopped(match (reaped, removed) {
                    (Ok(()), Ok(())) => "operation stop acknowledged during the call".to_owned(),
                    (Ok(()), Err(error)) => format!(
                        "operation stop acknowledged during the call; helper scratch removal failed: {error}"
                    ),
                    (Err(error), _) => format!(
                        "operation stop acknowledged during the call; helper reap failed ({error}); its scratch is retained"
                    ),
                }));
            }
            match wire::take(&self.scratch, Direction::Reply, &self.session, sequence) {
                Ok(Some(reply)) => return Ok(reply),
                Ok(None) => {}
                Err(reason) => return Err(self.fail(reason)),
            }
            match self.process.exited() {
                Ok(Some(status)) => {
                    return Err(self.fail(format!(
                        "helper exited during the call ({status}); native plugin effects are unknown"
                    )));
                }
                Ok(None) => {}
                Err(reason) => return Err(self.fail(reason)),
            }
            std::thread::sleep(pause);
            pause = (pause * 2).min(POLL_MAX);
        }
    }

    fn fail(&mut self, reason: String) -> HelperFault {
        let reaped = self.process.kill_and_reap();
        let removed = reaped
            .is_ok()
            .then(|| std::fs::remove_dir_all(&self.scratch));
        HelperFault::Transport(match (reaped, removed) {
            (Ok(()), Some(Ok(()))) => reason,
            (Ok(()), _) => format!("{reason}; helper scratch removal failed"),
            (Err(error), _) => format!("{reason}; helper reap failed ({error})"),
        })
    }
}

impl Drop for HelperSession {
    fn drop(&mut self) {
        let state = match self.state.get_mut() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let State::Running(running) = state
            && running.process.close(CLOSE_GRACE).is_ok()
        {
            let _ = std::fs::remove_dir_all(&running.scratch);
        }
    }
}

/// A directory builder for the helper's private scratch: owner-only (0700) on
/// Unix; elsewhere the per-user temporary directory's own ACL protects it.
#[cfg(unix)]
fn private_dir_builder() -> std::fs::DirBuilder {
    let mut builder = std::fs::DirBuilder::new();
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
}
#[cfg(not(unix))]
fn private_dir_builder() -> std::fs::DirBuilder {
    std::fs::DirBuilder::new()
}

fn private_scratch(session: &[u8; SESSION_BYTES]) -> Result<PathBuf, String> {
    let scratch = std::env::temp_dir().join(format!(
        "vibe-native-helper-{}-{}",
        std::process::id(),
        wire::session_hex(session)
    ));
    private_dir_builder()
        .create(&scratch)
        .map_err(|error| format!("creating private helper scratch: {error}"))?;
    if !scratch.is_absolute() {
        let _ = std::fs::remove_dir_all(&scratch);
        return Err("helper scratch is not absolute".to_owned());
    }
    Ok(scratch)
}

/// The helper-side entry: run one session when `args` (after the program
/// name and any harness prefix) begin with [`HELPER_MODE_ARG`].
///
/// Returns `None` when `args` do not select helper mode, so the caller
/// continues its ordinary boot; otherwise the session's outcome.
pub fn native_helper_entry(args: &[OsString]) -> Option<Result<(), String>> {
    let position = args.iter().position(|arg| arg == HELPER_MODE_ARG)?;
    let rest = &args[position + 1..];
    let [scratch, session] = rest else {
        return Some(Err(
            "native helper mode takes exactly `<scratch> <session>`".to_owned(),
        ));
    };
    let Some(session) = session.to_str() else {
        return Some(Err("native helper session is not UTF-8".to_owned()));
    };
    Some(wire::serve(Path::new(scratch), session))
}
