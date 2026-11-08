//! The running `vibe` is the trusted helper of controlled native calls.
//!
//! A controlled project operation loads and invokes native plugins in an
//! operation-owned helper process: this same executable, re-executed in a
//! hidden mode that serves one session and never reaches ordinary CLI boot,
//! settings or journals.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-STOP-LAW");

use std::process::ExitCode;

use vibe_lifecycle::native::NATIVE_HELPER_MODE_ARG as HELPER_MODE_ARG;

/// The stack the in-process native call had on the `vibe` worker thread.
const STACK: usize = 32 << 20;

/// Serve one helper session when argv selects the hidden helper mode.
pub(crate) fn entry() -> Option<ExitCode> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().is_none_or(|first| first != HELPER_MODE_ARG) {
        return None;
    }
    let served = std::thread::Builder::new()
        .name("vibe-native-helper".into())
        .stack_size(STACK)
        .spawn(move || vibe_lifecycle::native::native_helper_entry(&args))
        .map_err(|error| format!("starting the helper thread: {error}"))
        .and_then(|worker| {
            worker
                .join()
                .map_err(|_| "the native helper panicked".to_owned())
        });
    Some(match served {
        Ok(Some(Ok(()))) => ExitCode::SUCCESS,
        Ok(Some(Err(reason))) | Err(reason) => {
            eprintln!("vibe native helper: {reason}");
            ExitCode::from(2)
        }
        Ok(None) => ExitCode::from(2),
    })
}

/// Name this executable as the helper of every controlled operation. When
/// the running executable cannot be resolved, controlled native calls keep
/// refusing before any load.
pub(crate) fn install() {
    let Ok(program) = std::env::current_exe() else {
        return;
    };
    if let Ok(command) = vibe_lifecycle::native::NativeHelperCommand::trusted(program) {
        let _ = vibe_lifecycle::native::install_native_helper(command);
    }
}
