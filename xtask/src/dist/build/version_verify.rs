//! Confirm that release executables report the pinned workspace version.

use super::*;

pub(super) fn verify_version(path: &Path, component: &str, version: &Version) -> Result<()> {
    let mut command = Command::new(path);
    command.arg("--version");
    scrub_release_credentials(&mut command);
    let output = command
        .output()
        .with_context(|| format!("running `{component} --version` at `{}`", path.display()))?;
    if !output.status.success() {
        bail!(
            "`{component} --version` failed with exit {:?}",
            output.status.code()
        );
    }
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let expected = format!("{component} {version}");
    if actual != expected {
        bail!("`{component} --version` returned `{actual}`; expected `{expected}`");
    }
    Ok(())
}
