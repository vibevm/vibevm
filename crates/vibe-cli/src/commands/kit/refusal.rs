//! Kit refusals in the shared code/headline/why/fix/see spelling.
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

const SEE: &str = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface";

pub(super) fn refuse(
    code: &str,
    headline: impl std::fmt::Display,
    fix: Option<&str>,
    why: Option<&str>,
) -> anyhow::Error {
    let fix = fix.map(|fix| format!("; fix: {fix}")).unwrap_or_default();
    let why = why.map(|why| format!(": {why}")).unwrap_or_default();
    anyhow::anyhow!("{code}: {headline} (violates {SEE}{fix}){why}")
}

pub(super) fn quote(path: &std::path::Path) -> String {
    #[cfg(not(windows))]
    {
        crate::shell_syntax::posix_quote(&path.to_string_lossy())
    }
    #[cfg(windows)]
    {
        crate::shell_syntax::powershell_quote(&path.to_string_lossy())
    }
}

pub(super) fn missing(project: &vibe_safefs::Project, path: &str, label: &str) -> anyhow::Error {
    refuse(
        "kit_input_missing",
        format!("the {label} input {path} does not exist"),
        Some("put the file there, or correct its path in the recipe"),
        Some(&format!(
            "kit recipe paths are relative to {}",
            project.root_path().display()
        )),
    )
}

pub(super) fn output_parent(path: &std::path::Path, error: &anyhow::Error) -> anyhow::Error {
    let fix = private_parent_fix(
        path,
        error
            .downcast_ref::<vibe_safefs::PrivateDirectoryMode>()
            .is_some(),
    );
    refuse(
        "kit_output_unsafe",
        format!(
            "output parent {} must be owned by you with mode 0700",
            path.display()
        ),
        Some(&fix),
        None,
    )
}

fn private_parent_fix(path: &std::path::Path, owned_wrong_mode: bool) -> String {
    if cfg!(unix) && owned_wrong_mode {
        format!("chmod 700 {}", quote(path))
    } else {
        "choose a private directory owned by you".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncertain_parent_never_suggests_chmod() {
        let error = output_parent(
            std::path::Path::new("synthetic parent"),
            &anyhow::anyhow!("uncertain"),
        );
        assert!(
            error
                .to_string()
                .contains("choose a private directory owned by you")
        );
        assert!(!error.to_string().contains("chmod"));
    }

    #[cfg(unix)]
    #[test]
    fn confirmed_mode_advice_quotes_the_command_path() {
        assert_eq!(
            private_parent_fix(std::path::Path::new("my kit"), true),
            "chmod 700 'my kit'"
        );
        assert_eq!(
            private_parent_fix(std::path::Path::new("it's kit"), true),
            "chmod 700 'it'\"'\"'s kit'"
        );
    }
}
