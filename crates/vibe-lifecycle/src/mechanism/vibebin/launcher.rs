//! The launcher genre: its two marker spellings, its fixed per-platform
//! template, and §7.1's collision law.
//!
//! Three §7.1.0 rulings live here and none of them is re-decided:
//!
//! 1. **ruling 3 — the launcher is version-free by construction.** The body
//!    is a fixed template whose only variable is the command name; it
//!    carries no version, no digest and no absolute machine path, and it
//!    finds everything it needs relative to its own location. That is not a
//!    convention this cell follows, it is the only thing
//!    [`render`] can produce — there is no parameter through which a
//!    version or a digest could enter a body.
//! 2. **ruling 5 — the collision law.** §7.1: "A name already owned by the
//!    other genre—or by an unmarked user file—is a hard collision that
//!    names both origins and asks the target to choose another command
//!    alias." Both genres' exact marker spellings are constants here, so
//!    the refusal quotes DATA rather than prose, and a future reader of a
//!    launcher body can grep for either.
//! 3. **ruling 2's platform split.** `bin/<command>.cmd` on Windows,
//!    `bin/<command>` elsewhere. The flavour is a VALUE rather than a
//!    `cfg`-selected constant, so both bodies are provable on either host —
//!    a template that only one machine can render is a template only one
//!    machine can review.
//!
//! RATIFIED deviation from §7.1.0 ruling 2's literal `store/<sha256>`
//! (central, R8-VIBE-BIN acceptance): the Windows payload carries `.exe`,
//! because `cmd.exe` cannot execute a file whose name carries no PATHEXT
//! extension — verified live: an extension-less PE invoked from a `.cmd`
//! answers "is not recognized as an internal or external command" and
//! exits 9009, and the identical launcher against the same bytes named
//! `.exe` runs, forwards `%*` and preserves the exit code. §10's gate
//! requires the launcher to RUN, so the literal spelling is
//! unimplementable on Windows. The suffix is the platform's, exactly as
//! the launcher's own `.cmd` is one clause earlier; the content address is
//! unchanged and the pointer still names the bare digest. The
//! content-addressed-directory alternative (`store/<sha256>/payload.exe`)
//! was considered at acceptance and declined — one shape for both
//! platform suffixes beats a second directory level.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::Path;

use crate::mechanism::contain::{FileFault, prove_regular_file, read_file_bounded};
use crate::mechanism::error::DeployProviderError;

/// The launcher template's schema epoch — the provider portion of §4.1's
/// freshness digest, and the value that invalidates every deployed
/// launcher when the body changes.
pub(crate) const TEMPLATE_EPOCH: u32 = 1;

/// The prefix every VibeVM launcher genre's marker starts with. A body
/// that carries it belongs to SOME VibeVM genre; a body that does not is
/// an unmarked file of the user's own.
pub(crate) const MARKER_PREFIX: &str = "vibevm-launcher";

/// The exact marker a `deploy:vibe-bin` launcher body carries — OUR genre.
///
/// §7.1: "Their bodies carry an exact VibeVM marker naming the genre and
/// owner." The genre is the mechanism key the provider services; the owner
/// is the spec unit that rules the genre, which is the one identity this
/// project already uses everywhere a refusal has to name an authority.
pub(crate) const VIBE_BIN_MARKER: &str = "vibevm-launcher genre=deploy:vibe-bin \
     owner=spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS";

/// The exact marker the OTHER genre's bodies carry — PROP-025's
/// project-pinned `vibe bin` shim.
///
/// It is declared here, beside ours, because §7.1's collision law has to
/// name both origins and a refusal that quoted only one of them would send
/// a reader looking for a spelling nobody wrote down.
///
/// RATIFIED (central, R8-VIBE-BIN acceptance): a forward declaration, not
/// an observation — PROP-025 §4's shims are `spec/done` and unimplemented
/// (`vibe bin sync` is on that spec's own deferred list), so no writer
/// mints this spelling yet. The spelling LIVES HERE by decision: the atom
/// that lands `vibe bin sync` imports this constant rather than minting a
/// second one.
pub(crate) const PROJECT_SHIM_MARKER: &str = "vibevm-launcher genre=vibe-bin-shim \
     owner=spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch";

/// The reserved suffix of the active-payload pointer that sits beside a
/// launcher — `bin/<command>.current`.
pub(crate) const POINTER_SUFFIX: &str = ".current";

/// How much of an occupying file is read before the collision law decides.
///
/// A launcher body is under a kilobyte, so anything past this cap is
/// certainly not one; the cap exists so a `command` that happens to name a
/// large user file cannot make a refusal read it whole.
const OCCUPANT_CAP: u64 = 64 * 1024;

/// Which platform's launcher body and payload spelling a deployment uses.
///
/// A value rather than a `cfg`-selected constant on purpose: both bodies
/// are then renderable — and therefore provable — on either host, and the
/// shipped provider simply asks for [`LauncherFlavour::NATIVE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LauncherFlavour {
    /// Windows: a `.cmd` batch launcher.
    WindowsCmd,
    /// Everything else: a `#!/bin/sh` twin.
    PosixSh,
}

impl LauncherFlavour {
    /// The flavour of the host this build runs on — §7.1.0 ruling 2's
    /// "`.cmd` on Windows, `#!/bin/sh` else".
    pub(crate) const NATIVE: Self = if cfg!(windows) {
        Self::WindowsCmd
    } else {
        Self::PosixSh
    };

    /// The word a fingerprint and an evidence line spell this flavour as.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::WindowsCmd => "windows-cmd",
            Self::PosixSh => "posix-sh",
        }
    }

    /// The suffix the launcher file's own name carries.
    pub(crate) const fn launcher_suffix(self) -> &'static str {
        match self {
            Self::WindowsCmd => ".cmd",
            Self::PosixSh => "",
        }
    }

    /// The suffix a CAS payload's file name carries — see the module
    /// doc's ratified ruling-2 deviation for why Windows needs one.
    pub(crate) const fn payload_suffix(self) -> &'static str {
        match self {
            Self::WindowsCmd => ".exe",
            Self::PosixSh => "",
        }
    }

    /// Whether a written file of this flavour needs the executable bit.
    pub(crate) const fn needs_executable_bit(self) -> bool {
        matches!(self, Self::PosixSh)
    }
}

/// Render one launcher body — the whole template, and the only place a
/// launcher body comes from.
///
/// `command` is a validated portable token (lowercase ASCII letters,
/// digits, `-` and `.`), so it cannot carry a quote, a `%`, a `$` or a path
/// separator: the substitutions below are safe by the config cell's own
/// law, not by escaping here.
pub(crate) fn render(flavour: LauncherFlavour, command: &str) -> Vec<u8> {
    match flavour {
        LauncherFlavour::WindowsCmd => windows_cmd(command).into_bytes(),
        LauncherFlavour::PosixSh => posix_sh(command).into_bytes(),
    }
}

/// The Windows body. CRLF throughout, because a `.cmd` with bare LF line
/// endings is read by some Windows shells one label at a time.
fn windows_cmd(command: &str) -> String {
    let lines = [
        "@echo off".to_owned(),
        format!("rem {VIBE_BIN_MARKER}"),
        format!("rem command={command}"),
        "setlocal".to_owned(),
        format!("set \"VIBE_BIN_POINTER=%~dp0{command}{POINTER_SUFFIX}\""),
        "if not exist \"%VIBE_BIN_POINTER%\" goto vibe_bin_no_payload".to_owned(),
        "set \"VIBE_BIN_PAYLOAD=\"".to_owned(),
        "set /p VIBE_BIN_PAYLOAD=<\"%VIBE_BIN_POINTER%\"".to_owned(),
        "if not defined VIBE_BIN_PAYLOAD goto vibe_bin_no_payload".to_owned(),
        "\"%~dp0..\\store\\%VIBE_BIN_PAYLOAD%.exe\" %*".to_owned(),
        "exit /b %ERRORLEVEL%".to_owned(),
        ":vibe_bin_no_payload".to_owned(),
        format!("echo vibe-bin: no active payload is recorded for \"{command}\" 1>&2"),
        "exit /b 1".to_owned(),
    ];
    let mut body = String::new();
    for line in lines {
        body.push_str(&line);
        body.push_str("\r\n");
    }
    body
}

/// The POSIX twin.
fn posix_sh(command: &str) -> String {
    let absent = format!("vibe-bin: no active payload is recorded for \"{command}\"");
    let lines = [
        "#!/bin/sh".to_owned(),
        format!("# {VIBE_BIN_MARKER}"),
        format!("# command={command}"),
        "vibe_bin_dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd) || exit 1".to_owned(),
        format!("vibe_bin_pointer=\"$vibe_bin_dir/{command}{POINTER_SUFFIX}\""),
        "if [ ! -r \"$vibe_bin_pointer\" ]; then".to_owned(),
        format!("  printf '%s\\n' '{absent}' >&2"),
        "  exit 1".to_owned(),
        "fi".to_owned(),
        "vibe_bin_payload=$(cat -- \"$vibe_bin_pointer\") || exit 1".to_owned(),
        "if [ -z \"$vibe_bin_payload\" ]; then".to_owned(),
        format!("  printf '%s\\n' '{absent}' >&2"),
        "  exit 1".to_owned(),
        "fi".to_owned(),
        "exec \"$vibe_bin_dir/../store/$vibe_bin_payload\" \"$@\"".to_owned(),
    ];
    let mut body = String::new();
    for line in lines {
        body.push_str(&line);
        body.push('\n');
    }
    body
}

/// The pointer file's whole content — §7.1.0 ruling 2's "one line naming
/// the payload digest".
///
/// LF on both platforms and no second member: the pointer is the receipt's
/// projection, and every byte a shell has to parse is a byte a second
/// parser could disagree about. Windows `set /p` and POSIX `$(cat …)` both
/// yield exactly the digest from these bytes.
pub(crate) fn pointer_body(payload_digest: &str) -> Vec<u8> {
    format!("{payload_digest}\n").into_bytes()
}

/// The payload digest one pointer file's bytes name, or `None` when the
/// bytes are not a single digest line.
pub(crate) fn pointer_digest(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let line = text.lines().next()?.trim();
    if line.len() != 64 || !line.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(line.to_ascii_lowercase())
}

/// What occupies a launcher path right now, as the collision law sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupant {
    /// Nothing is there: a first deployment.
    Vacant,
    /// A body carrying OUR genre marker — an ordinary update (§7.1.0
    /// ruling 5: "Same-genre is an update, not a collision").
    OurGenre,
    /// A body carrying the PROP-025 project-pinned shim's marker.
    ProjectShim,
    /// A body carrying some other `vibevm-launcher` genre marker.
    ForeignGenre,
    /// A file carrying no VibeVM launcher marker at all — the user's own.
    Unmarked,
}

/// Classify whatever occupies one launcher path.
///
/// Read-only: it opens the path, reads at most [`OCCUPANT_CAP`] bytes and
/// writes nothing. That is what lets `plan` consult the same law `apply`
/// enforces without a plan ever becoming a mutation.
pub(crate) fn classify(path: &Path) -> Result<Occupant, FileFault> {
    match prove_regular_file(path) {
        Ok(_) => {}
        // Absence is the vacant case, not a fault: a first deployment is
        // the ordinary path, not an error to report.
        Err(FileFault::Missing(_)) => return Ok(Occupant::Vacant),
        Err(fault) => return Err(fault),
    }
    // Anything past the cap cannot be a launcher body, and reading it
    // whole to find that out is exactly what the cap prevents.
    let bytes = match read_file_bounded(path, OCCUPANT_CAP) {
        Ok(bytes) => bytes,
        Err(FileFault::Read(_)) => return Ok(Occupant::Unmarked),
        Err(fault) => return Err(fault),
    };
    if contains(&bytes, VIBE_BIN_MARKER) {
        return Ok(Occupant::OurGenre);
    }
    if contains(&bytes, PROJECT_SHIM_MARKER) {
        return Ok(Occupant::ProjectShim);
    }
    if contains(&bytes, MARKER_PREFIX) {
        return Ok(Occupant::ForeignGenre);
    }
    Ok(Occupant::Unmarked)
}

/// §7.1's collision law: refuse unless the path is vacant or already ours.
///
/// The refusal names BOTH origins with their exact marker spellings and
/// the fix, because a human who meets it has to decide which of two
/// VibeVM genres — or neither — put the file there, and the marker is the
/// only thing that answers.
pub(crate) fn refuse_collision(
    target: &str,
    resource: &str,
    occupant: Occupant,
) -> Result<(), DeployProviderError> {
    let observed = match occupant {
        Occupant::Vacant | Occupant::OurGenre => return Ok(()),
        Occupant::ProjectShim => format!(
            "it carries the PROP-025 project-pinned `vibe bin` shim's marker `{PROJECT_SHIM_MARKER}`"
        ),
        Occupant::ForeignGenre => format!(
            "it carries the `{MARKER_PREFIX}` marker prefix of some VibeVM launcher genre, but \
             neither this provider's nor the PROP-025 shim's"
        ),
        Occupant::Unmarked => {
            "it carries no VibeVM launcher marker at all, so it is a file of your own and this \
             provider will not judge what it is"
                .to_owned()
        }
    };
    Err(DeployProviderError::LauncherCollision {
        target: target.to_owned(),
        resource: resource.to_owned(),
        observed,
        ours: VIBE_BIN_MARKER,
        shim: PROJECT_SHIM_MARKER,
    })
}

/// Whether a byte slice contains one marker's bytes.
fn contains(haystack: &[u8], needle: &str) -> bool {
    let needle = needle.as_bytes();
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
