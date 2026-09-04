//! Pure argv construction for built-in health presets.

mod cargo;
mod maven;
mod npm;
mod python;

pub(crate) use cargo::cargo_commands;
pub(crate) use maven::maven_commands;
pub(crate) use npm::npm_commands;
pub(crate) use python::python_commands;

use std::collections::BTreeMap;

use super::model::{EnvironmentValue, PreparedArg};

pub(crate) fn literal(value: impl Into<String>) -> PreparedArg {
    PreparedArg::Literal(value.into())
}

pub(crate) fn hermetic_environment() -> BTreeMap<String, EnvironmentValue> {
    let mut environment = [
        ("HOME", "home"),
        ("TMPDIR", "tmp"),
        ("TMP", "tmp"),
        ("TEMP", "tmp"),
        ("USERPROFILE", "home"),
        ("APPDATA", "appdata"),
        ("LOCALAPPDATA", "local-appdata"),
    ]
    .into_iter()
    .map(|(name, suffix)| {
        (
            name.to_owned(),
            EnvironmentValue::ScratchPath(suffix.to_owned()),
        )
    })
    .collect::<BTreeMap<_, _>>();
    if let Some(path) = std::env::var_os("PATH") {
        let filtered = std::env::split_paths(&path)
            .filter(|entry| {
                !entry
                    .display()
                    .to_string()
                    .to_ascii_lowercase()
                    .contains(".vibe")
            })
            .collect::<Vec<_>>();
        if let Ok(path) = std::env::join_paths(filtered) {
            environment.insert(
                "PATH".to_owned(),
                EnvironmentValue::Literal(path.to_string_lossy().into_owned()),
            );
        }
    }
    for name in ["SystemRoot", "WINDIR"] {
        if let Ok(value) = std::env::var(name) {
            environment.insert(name.to_owned(), EnvironmentValue::Literal(value));
        }
    }
    environment
}
